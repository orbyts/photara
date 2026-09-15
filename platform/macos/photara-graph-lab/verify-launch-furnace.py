#!/usr/bin/env python3
"""Exercise Graph's real signed process/window startup and owned-child cleanup.

Injected timeout cases are expected failures, recorded individually. Recovery
cases are new explicit attempts, never retries that erase a failed launch.
No build, install, permission request, or unrelated application is performed.
"""

import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--skip-focus", action="store_true",
                        help="Record the native foreground-focus scenario as an unrun gate.")
    options = parser.parse_args()
    app = options.app.resolve()
    output = (options.output.resolve() if options.output else
              Path(tempfile.mkdtemp(prefix="launch-furnace.", dir=app.parent)))
    if output.exists() and any(output.iterdir()):
        parser.error("--output must be a new or empty directory")
    output.mkdir(parents=True, exist_ok=True)
    spec = importlib.util.spec_from_file_location(
        "graph_launcher", Path(__file__).with_name("launch-verification.py"))
    launcher = importlib.util.module_from_spec(spec)
    sys.dont_write_bytecode = True
    spec.loader.exec_module(launcher)
    scenarios = [
        ("cold-launch", None, "passed", 15, 30),
        ("windowless-startup", "windowless", "startup-timeout", 2, 30),
        ("windowless-recovery", None, "passed", 15, 30),
        ("stalled-startup", "stalled", "startup-timeout", 2, 30),
        ("stalled-recovery", None, "passed", 15, 30),
        ("run-timeout-cleanup", "run-stalled", "run-timeout", 15, 2),
        ("run-timeout-recovery", None, "passed", 15, 30),
    ]
    if not options.skip_focus:
        scenarios.append(("foreground-focus-interruption", None, "passed", 15, 60))
    report = {"protocol": 1, "app": str(app), "cases": [], "failures": 0,
              "native_assertions": 0, "native_failures": 0,
              "unrun_gates": ["foreground-focus-interruption"] if options.skip_focus else []}
    environment = dict(os.environ)
    # Each scenario declares its own fault and mode; caller-provided test fault or
    # preflight flags cannot accidentally change the furnace's intended cases.
    for variable in ("PHOTARA_GRAPH_HOST_FAULT", "PHOTARA_GRAPH_PREFLIGHT_ONLY",
                     "PHOTARA_GRAPH_CONTEXT_MATRIX_ONLY", "PHOTARA_GRAPH_CAMERA_ONLY"):
        environment.pop(variable, None)
    for name, fault, expected, startup_timeout, run_timeout in scenarios:
        scenario_environment = dict(environment)
        if fault:
            scenario_environment["PHOTARA_GRAPH_HOST_FAULT"] = fault
        arguments = ["--launch-furnace"]
        if name == "foreground-focus-interruption":
            arguments.append("--launch-furnace-focus")
        print(f"FURNACE: {name}; expected={expected}", flush=True)
        actual = launcher.launch(app, output / name, arguments, startup_timeout,
                                 run_timeout, scenario_environment)
        checks = {"expected_outcome": actual["outcome"] == expected,
                  "process_entry": actual["process_started"],
                  "owned_process_reaped": actual["cleanup"]["reaped"],
                  "one_attempt": actual["attempts"] == 1,
                  "readiness": actual["ready"] == (expected != "startup-timeout")}
        if expected.endswith("timeout"):
            checks["watchdog_signaled_owned_child"] = bool(actual["cleanup"]["signals"])
            checks["bounded_completion"] = actual["elapsed_seconds"] < startup_timeout + run_timeout + 15
            checks["no_completed_result"] = not (output / name / "exit-code").exists()
        case = {"name": name, "fault": fault, "expected": expected,
                "actual": actual["outcome"], "checks": checks,
                "passed": all(checks.values()), "launch_report": str(output / name / "launch-report.json")}
        # Keep host assertion totals distinct from this launcher's protocol
        # assertions. A timeout intentionally has no completed native summary.
        native_summary = re.findall(r"^(\d+) assertions; (\d+) failures$",
                                    (output / name / "stdout.log").read_text(errors="replace"), re.M)
        case["native_summary"] = ({"assertions": int(native_summary[-1][0]),
                                    "failures": int(native_summary[-1][1])}
                                   if native_summary else None)
        if case["native_summary"] is not None:
            report["native_assertions"] += case["native_summary"]["assertions"]
            report["native_failures"] += case["native_summary"]["failures"]
        report["cases"].append(case)
        report["failures"] += sum(not value for value in checks.values())
        report["assertions"] = sum(len(entry["checks"]) for entry in report["cases"])
        launcher.atomic_json(output / "furnace-report.json", report)
    report["passed"] = report["failures"] == 0 and not report["unrun_gates"]
    launcher.atomic_json(output / "furnace-report.json", report)
    print(f"FURNACE RESULT: {report['assertions']} assertions, {report['failures']} failures; "
          f"native={report['native_assertions']} assertions/{report['native_failures']} failures; "
          f"unrun={len(report['unrun_gates'])}; {output / 'furnace-report.json'}", flush=True)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
