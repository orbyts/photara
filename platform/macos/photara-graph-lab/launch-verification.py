#!/usr/bin/env python3
"""Bounded, one-attempt launcher for the exact signed Graph verification host.

The host explicitly owns an AppKit window, so LaunchServices/open -W is neither
needed nor used. Owning the real child allows a windowless or stalled process to
be diagnosed and reaped without searching for/killing other application instances.
"""

import argparse
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import uuid


class InterruptedRun(Exception):
    pass


def positive_seconds(value):
    value = float(value)
    if not math.isfinite(value) or value <= 0:
        raise argparse.ArgumentTypeError("timeout must be a finite positive number")
    return value


def read_json(path):
    try:
        value = json.loads(path.read_text())
    except (OSError, ValueError):
        return None
    return value if isinstance(value, dict) else None


def atomic_json(path, value):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def identity_matches(record, child, token):
    return (record is not None and record.get("protocol") == 1
            and record.get("pid") == child.pid and record.get("token") == token)


def ready_matches(record, child, token):
    return (identity_matches(record, child, token)
            and type(record.get("window_number")) is int
            and record["window_number"] > 0
            and record.get("window_identifier") == "photara.graph.verification." + token
            and record.get("event_surface") is True
            and record.get("lifecycle_started") is True)


def run_diagnostic(command, destination, timeout):
    try:
        with destination.open("w") as output:
            result = subprocess.run(command, stdin=subprocess.DEVNULL, stdout=output,
                                    stderr=subprocess.STDOUT, timeout=timeout, check=False)
        return {"file": destination.name, "exit_code": result.returncode}
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"file": destination.name, "error": type(error).__name__}


def diagnose(child, run_root):
    # Whitelist fields: never dump the environment, arguments, preferences,
    # Keychain, user projects, or the process table for unrelated applications.
    diagnostics = []
    if child.poll() is None:
        diagnostics.append(run_diagnostic(
            ["/bin/ps", "-p", str(child.pid), "-o", "pid=,ppid=,state=,etime=,comm="],
            run_root / "process-state.txt", 2))
        if Path("/usr/bin/sample").exists():
            diagnostics.append(run_diagnostic(
                ["/usr/bin/sample", str(child.pid), "1", "1", "-file",
                 str(run_root / "process-sample.txt")],
                run_root / "sample-command.txt", 3))
    return diagnostics


def cleanup(child):
    """Signal only the Popen-owned, still unreaped child. Never use pkill/killall."""
    actions = []
    if child.poll() is None:
        child.terminate()
        actions.append("SIGTERM")
        try:
            child.wait(timeout=2)
        except subprocess.TimeoutExpired:
            child.kill()
            actions.append("SIGKILL")
            try:
                child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                # Report the cleanup failure without an unbounded wait. Never
                # attempt to kill other instances as a substitute.
                pass
    return {"signals": actions, "reaped": child.poll() is not None,
            "child_exit_code": child.returncode}


def launch(app, run_root, arguments, startup_timeout=15, run_timeout=1200, environment=None):
    executable = app / "Contents" / "MacOS" / "PhotaraGraphLab"
    if not executable.is_file():
        raise ValueError(f"Graph verifier executable is missing: {executable}")
    if run_root.exists() and any(run_root.iterdir()):
        raise ValueError(f"Refusing nonempty run directory (stale results): {run_root}")
    run_root.mkdir(parents=True, exist_ok=True)
    token = uuid.uuid4().hex
    child_environment = dict(os.environ if environment is None else environment)
    child_environment.update({"PHOTARA_GRAPH_EXIT_FILE": str(run_root / "exit-code"),
                              "PHOTARA_GRAPH_RUN_TOKEN": token})
    report = {"protocol": 1, "attempts": 1, "app": str(app), "run_root": str(run_root),
              "startup_timeout_seconds": startup_timeout, "run_timeout_seconds": run_timeout,
              "outcome": "launch-error", "ready": False, "process_started": False,
              "diagnostics": [], "cleanup": {"signals": [], "reaped": True}}
    start = time.monotonic()
    ready_at = None
    child = None
    previous_handlers = {}

    def interrupt(signum, _frame):
        raise InterruptedRun(f"received signal {signum}")

    try:
        for signum in (signal.SIGINT, signal.SIGTERM):
            previous_handlers[signum] = signal.signal(signum, interrupt)
        with (run_root / "stdout.log").open("wb") as stdout, (run_root / "stderr.log").open("wb") as stderr:
            child = subprocess.Popen([str(executable), *arguments], env=child_environment,
                                     stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
            report["pid"] = child.pid
            print(f"Graph verification pid={child.pid}; log={run_root / 'stdout.log'}", flush=True)
            while True:
                elapsed = time.monotonic() - start
                # Poll first: once exit is observed, all final atomic records
                # exist. Reading markers before poll can misclassify a very
                # fast successful child that writes them between those calls.
                child_status = child.poll()
                process_record = read_json(run_root / "process.json")
                ready_record = read_json(run_root / "ready.json")
                report["process_started"] = identity_matches(process_record, child, token)
                if ready_at is None and report["process_started"] and ready_matches(ready_record, child, token):
                    ready_at = time.monotonic()
                    report.update(ready=True, ready_after_seconds=round(elapsed, 3),
                                  window_number=ready_record["window_number"],
                                  window_identifier=ready_record["window_identifier"])
                    print(f"Graph verification ready: window={ready_record['window_number']}", flush=True)
                if child_status is not None:
                    try:
                        result = (run_root / "exit-code").read_text().strip()
                    except OSError:
                        result = None
                    report["recorded_exit_code"] = result
                    if not report["process_started"]:
                        report["outcome"] = "missing-process-handshake"
                    elif ready_at is None:
                        report["outcome"] = "exit-before-ready"
                    elif result not in ("0", "1"):
                        report["outcome"] = "missing-or-invalid-result"
                    elif child_status != int(result):
                        report["outcome"] = "exit-result-mismatch"
                    else:
                        report["outcome"] = "passed" if result == "0" else "verification-failed"
                    break
                if ready_at is None and elapsed >= startup_timeout:
                    report["outcome"] = "startup-timeout"
                    report["detail"] = (
                        "Process entered, but no identified window/event surface became ready."
                        if report["process_started"] else
                        "Executable launched, but its process-entry handshake never appeared.")
                    break
                if ready_at is not None and time.monotonic() - ready_at >= run_timeout:
                    report["outcome"] = "run-timeout"
                    report["detail"] = "Window became ready, but verification did not finish within its deadline."
                    break
                time.sleep(0.05)
    except InterruptedRun as error:
        report.update(outcome="interrupted", detail=str(error))
    except OSError as error:
        report.update(outcome="launch-error", detail=str(error))
    finally:
        # Do not allow repeated terminal signals to skip cleanup of this child.
        for signum in previous_handlers:
            signal.signal(signum, signal.SIG_IGN)
        if child is not None:
            if report["outcome"] != "passed":
                report["diagnostics"] = diagnose(child, run_root)
            report["cleanup"] = cleanup(child)
        report["elapsed_seconds"] = round(time.monotonic() - start, 3)
        atomic_json(run_root / "launch-report.json", report)
        for signum, previous in previous_handlers.items():
            signal.signal(signum, previous)
    print(f"Graph verification {report['outcome']}; report={run_root / 'launch-report.json'}", flush=True)
    if report["outcome"] != "passed":
        print("Inspect stdout.log, stderr.log and process-sample.txt (when available). "
              "No automatic retry was attempted.", file=sys.stderr, flush=True)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, required=True)
    parser.add_argument("--run-root", type=Path)
    parser.add_argument("--startup-timeout", type=positive_seconds, default=15)
    parser.add_argument("--run-timeout", type=positive_seconds, default=1200)
    parser.add_argument("arguments", nargs=argparse.REMAINDER)
    options = parser.parse_args()
    app = options.app.resolve()
    run_root = (options.run_root.resolve() if options.run_root else
                Path(tempfile.mkdtemp(prefix="run.", dir=app.parent)))
    arguments = options.arguments[1:] if options.arguments[:1] == ["--"] else options.arguments
    try:
        report = launch(app, run_root, arguments, options.startup_timeout, options.run_timeout)
    except (OSError, ValueError) as error:
        parser.exit(2, f"Graph verification launcher: {error}\n")
    for name in ("stdout.log", "stderr.log"):
        path = run_root / name
        if path.exists():
            print(path.read_text(errors="replace"), end="", flush=True)
    return 0 if report["outcome"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
