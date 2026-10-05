#!/usr/bin/env python3
"""Opt-in local lab launcher. Reuses a separately reviewed image controller.

--controller and --controller-sha256 select that exact existing tool; this wrapper
never substitutes a downloader, mounts a user image, force-detaches, or deletes.
Builds only with --build-only. Otherwise creates a new disposable image, or
reopens the exact retained controller-owned image supplied with --resume.
Fresh lab example: --standing-bytes 4194304 registers a 4 MiB initial control/
journal allowance for this disposable specimen only. It is not a production
limit, native physical reservation, or a promise of unbounded editing.
--pack-reserve-bytes 4194304 separately requests a fresh 4 MiB reserve per
writable tip, measured by the native adapter before registration. Resume neither
changes this registered reserve nor enlarges existing files.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
TEST = "package::v1_3::repeatable::tests::native_test::native_repeatable_phase"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def regular(path):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
        raise RuntimeError("Expected an exact regular single-link tool/file")
    return info.st_dev, info.st_ino


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--controller", type=Path, required=True)
    parser.add_argument("--controller-sha256", required=True)
    parser.add_argument("--host-binary", type=Path)
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument("--standing-bytes", type=int,
                        help="fresh disposable registration only; positive 4096-aligned bytes (lab example: 4194304)")
    parser.add_argument("--pack-reserve-bytes", type=int,
                        help="fresh native tip reserve only; positive 4096-aligned bytes (lab example: 4194304)")
    parser.add_argument("--resume", type=Path, help="exact detached controller-owned scratch to reopen")
    args = parser.parse_args()
    if args.resume and (args.standing_bytes is not None or args.pack_reserve_bytes is not None):
        parser.error("--resume preserves registration; omit --standing-bytes and --pack-reserve-bytes")
    for name, value in [("--standing-bytes", args.standing_bytes),
                        ("--pack-reserve-bytes", args.pack_reserve_bytes)]:
        if value is not None and (value <= 0 or value % 4096 or value > (1 << 64) - 1):
            parser.error(f"{name} must be positive, 4096-aligned and fit u64")
        if not args.build_only and not args.resume and value is None:
            parser.error(f"fresh preparation requires explicit {name}")
    controller = args.controller.absolute()
    controller_pin = regular(controller)
    if digest(controller) != args.controller_sha256:
        raise RuntimeError("Controller source differs from the reviewed digest")
    build = Path(tempfile.mkdtemp(prefix="photara-ps3-lab-build-", dir="/private/tmp"))
    os.chmod(build, 0o700)

    def command(argv, log, env=None, timeout=1200):
        with (build / log).open("xb") as output:
            result = subprocess.run(argv, cwd=REPO, env=env, stdout=output,
                                    stderr=subprocess.STDOUT, timeout=timeout, check=False)
        if result.returncode:
            raise RuntimeError(f"Command failed ({result.returncode}); inspect {build / log}")
        return build / log

    if args.host_binary:
        host = args.host_binary.absolute()
    else:
        artifacts = command(["cargo", "test", "-p", "photara-store", "--lib", "--release",
                             "--no-run", "--message-format=json"], "host-build.log")
        if artifacts.stat().st_size > 16 * 1024 * 1024:
            raise RuntimeError("Compiler metadata exceeded the launcher bound")
        hosts = []
        for line in artifacts.read_bytes().splitlines():
            if not line.startswith(b"{"):
                continue
            value = json.loads(line)
            if (value.get("reason") == "compiler-artifact"
                    and value.get("target", {}).get("name") == "photara_store"
                    and value.get("profile", {}).get("test") and value.get("executable")):
                hosts.append(Path(value["executable"]))
        if len(hosts) != 1:
            raise RuntimeError("Expected one compiled photara-store libtest host")
        host = hosts[0]
    host_pin, host_sha = regular(host), digest(host)
    environment = os.environ.copy()
    environment.pop("PHOTARA_PS3_STANDING_BYTES", None)
    environment.pop("PHOTARA_PS3_PACK_RESERVE_BYTES", None)
    if args.standing_bytes is not None:
        environment["PHOTARA_PS3_STANDING_BYTES"] = str(args.standing_bytes)
    if args.pack_reserve_bytes is not None:
        environment["PHOTARA_PS3_PACK_RESERVE_BYTES"] = str(args.pack_reserve_bytes)
    environment["PHOTARA_PS3_LAB_BUILD"] = str(build / "app")
    command(["zsh", str(HERE / "build-disposable-autosave.sh")], "app-build.log", environment)
    executable = build / "app/Disposable Autosave.app/Contents/MacOS/DisposableAutosave"
    regular(executable)
    print(json.dumps({"build": str(build), "host": str(host), "host_sha256": host_sha,
                      "controller_sha256": args.controller_sha256,
                      "fresh_standing_bytes": args.standing_bytes,
                      "fresh_pack_reserve_bytes": args.pack_reserve_bytes, "installed": False}), flush=True)
    if args.build_only:
        return

    def controller_command(*arguments):
        if regular(controller) != controller_pin or digest(controller) != args.controller_sha256:
            raise RuntimeError("Controller identity changed")
        result = subprocess.run([sys.executable, str(controller), *map(str, arguments)],
                                cwd=REPO, capture_output=True, timeout=300, check=False)
        if result.returncode or len(result.stdout) > 65536 or len(result.stderr) > 65536:
            raise RuntimeError("Reviewed controller refused; preserve its owned image and logs")
        return json.loads(result.stdout)

    if args.resume:
        root = args.resume.absolute()
        if root.parent != Path("/private/tmp") or not root.name.startswith("photara-ps2-remount-"):
            raise RuntimeError("Unexpected resume scope")
        generations = []
        for entry in root.glob("binding-*.json"):
            regular(entry)
            suffix = entry.name.removeprefix("binding-").removesuffix(".json")
            if not suffix.isascii() or not suffix.isdigit():
                raise RuntimeError("Unexpected controller binding name")
            generations.append(int(suffix))
        if not generations:
            raise RuntimeError("No retained controller generation; refuse new registration")
        generation = max(generations) + 1
        if generation > (1 << 64) - 1:
            raise RuntimeError("Controller generation overflow")
        attached = controller_command("attach", root, generation)
        if (attached.get("scratch") != str(root) or attached.get("generation") != generation
                or attached.get("mounted") is not True):
            raise RuntimeError("Controller did not confirm the requested resume generation")
    else:
        created = controller_command("init")
        root = Path(created["scratch"])
        if root.parent != Path("/private/tmp") or not root.name.startswith("photara-ps2-remount-"):
            raise RuntimeError("Unexpected controller scope")
        generation = created["generation"]
        if generation != 1 or created["mounted"] is not True:
            raise RuntimeError("Expected a fresh controller-owned image")
    print(json.dumps({"scratch": str(root), "mounted": True, "retained": True}), flush=True)
    # Use the exact built host for preparation, avoiding a second Cargo build.
    binding_path = root / f"binding-{generation}.json"
    regular(binding_path)
    environment.update(PHOTARA_PS3_HOST=str(host),
                       PHOTARA_PS2_REMOUNT_MANIFEST=str(root / "manifest.json"),
                       PHOTARA_PS2_REMOUNT_BINDING=binding_path.read_text(),
                       PHOTARA_PS2_REPEATABLE_PHASE="prepare-session")
    try:
        if regular(host) != host_pin or digest(host) != host_sha:
            raise RuntimeError("Compiled Rust host changed before preparation")
        if not args.resume:
            command([str(host), TEST, "--exact", "--ignored", "--nocapture"],
                    "prepare-session.log", environment, timeout=300)
            # Establish genuine checkpoint-only Saved evidence before first UI
            # attachment. Resume never repeats this initialization operation.
            environment["PHOTARA_PS2_REPEATABLE_PHASE"] = "complete"
            command([str(host), TEST, "--exact", "--ignored", "--nocapture"],
                    "initial-complete.log", environment, timeout=300)
        if regular(host) != host_pin or digest(host) != host_sha:
            raise RuntimeError("Compiled Rust host changed before launch")
        with (build / "native-app.log").open("xb") as output:
            app = subprocess.Popen([str(executable)], env=environment,
                                   stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
            print(json.dumps({"app_pid": app.pid, "scratch": str(root),
                              "native_log": str(build / "native-app.log")}), flush=True)
            result = app.wait()
        if result != 0:
            raise RuntimeError("App did not close normally; leave image mounted for explicit recovery")
        detached = controller_command("detach", root, generation)
        if detached.get("detached") is not True or detached.get("scratch") != str(root):
            raise RuntimeError("Controller did not confirm exact clean detach")
        print(json.dumps({"scratch": str(root), "detached": True,
                          "retained": True, "build": str(build)}), flush=True)
    except BaseException:
        print(json.dumps({"scratch": str(root), "retained": True,
                          "automatic_detach": False,
                          "note": "Preserve app/image; use the reviewed controller for recovery."}),
              file=sys.stderr, flush=True)
        raise


if __name__ == "__main__":
    main()
