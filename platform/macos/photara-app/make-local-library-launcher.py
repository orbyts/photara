#!/usr/bin/python3
"""Build a double-click launcher for one already mounted disposable registration.

Does not create, attach, detach, rebind, install, or delete anything. The shared
Rust constructor independently checks every actual storage/SQL authority pin.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import stat
import subprocess
import sys
import time

LIMIT = 65536


def observe(path, limit):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_uid != os.getuid() or info.st_size > limit:
            raise ValueError("Expected a bounded owned regular file")
        chunks = []
        left = limit + 1
        while left:
            chunk = os.read(fd, min(left, 65536))
            if not chunk:
                break
            chunks.append(chunk)
            left -= len(chunk)
        data = b"".join(chunks)
        after = os.fstat(fd)
        if len(data) > limit or (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns) != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns):
            raise ValueError("File changed while reading")
        return data, {"path": str(path), "device": info.st_dev, "inode": info.st_ino,
                      "sha256": hashlib.sha256(data).hexdigest(), "limit": limit}
    finally:
        os.close(fd)


def checked(record):
    data, actual = observe(Path(record["path"]), record["limit"])
    if actual != record:
        raise ValueError("Registered launcher input changed; preserve the disposable session")
    return data


def create(args):
    root = args.manifest.absolute().parent
    if root.parent != Path("/private/tmp") or not root.name.startswith("photara-ps2-remount-"):
        raise ValueError("Expected the explicit controller-owned scope")
    records = {}
    for name, path, limit in [("executable", args.executable, 256 * 1024 * 1024),
                              ("library", args.executable.parents[1] / "Frameworks/libphotara_bridge.dylib", 256 * 1024 * 1024),
                              ("manifest", args.manifest, LIMIT), ("binding", args.binding, LIMIT),
                              ("targets", args.target_bindings, LIMIT), ("database", args.database_registration, LIMIT)]:
        path = path.absolute()
        if name not in ("executable", "library") and path.parent != root:
            raise ValueError("Controller input must belong to the original scope")
        data, records[name] = observe(path, limit)
        if name not in ("executable", "library") and not isinstance(json.loads(data), dict):
            raise ValueError("Controller object required")
    bundle = args.output.absolute()
    if bundle.suffix != ".app" or bundle.exists():
        raise ValueError("Choose a new .app output; existing launcher is preserved")
    executable_dir = bundle / "Contents/MacOS"
    resources = bundle / "Contents/Resources"
    executable_dir.mkdir(parents=True, mode=0o700)
    resources.mkdir(mode=0o700)
    config = {"version": 1, "records": records, "log_root": str(root)}
    (resources / "launch.json").write_text(json.dumps(config, sort_keys=True, separators=(",", ":")))
    launcher = executable_dir / "LocalLibraries"
    launcher.write_bytes(Path(__file__).read_bytes())
    launcher.chmod(0o700)
    with (bundle / "Contents/Info.plist").open("wb") as output:
        identity = hashlib.sha256(json.dumps(config, sort_keys=True).encode()).hexdigest()[:16]
        plistlib.dump({"CFBundleIdentifier": "com.photara.disposable-launcher.c" + identity, "CFBundleName": bundle.stem,
                      "CFBundleDisplayName": bundle.stem, "CFBundleExecutable": "LocalLibraries",
                      "CFBundlePackageType": "APPL", "LSUIElement": True}, output)
    subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-", str(bundle)], check=True)
    print(bundle)


def launch():
    bundle = Path(__file__).absolute().parents[2]
    data, _ = observe(bundle / "Contents/Resources/launch.json", LIMIT)
    config = json.loads(data)
    if config.get("version") != 1 or set(config.get("records", {})) != {"executable", "library", "manifest", "binding", "targets", "database"}:
        raise ValueError("Unsupported launcher registration")
    values = {key: checked(value) for key, value in config["records"].items()}
    root = Path(config["log_root"])
    info = root.lstat()
    if root.parent != Path("/private/tmp") or not root.name.startswith("photara-ps2-remount-") or not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise ValueError("Controller scope changed")
    env = {key: value for key, value in os.environ.items() if not key.startswith("PHOTARA_")}
    records = config["records"]
    env.update(PHOTARA_PS2_REMOUNT_MANIFEST=records["manifest"]["path"],
               PHOTARA_PS2_REMOUNT_BINDING=values["binding"].decode("utf-8"),
               PHOTARA_PS4_TARGET_BINDINGS=values["targets"].decode("utf-8"),
               PHOTARA_LL2A_DATABASE_REGISTRATION=records["database"]["path"])
    log = root / ("local-library-app-" + str(time.time_ns()) + ".log")
    fd = os.open(log, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "wb") as output:
        result = subprocess.run([records["executable"]["path"]], env=env, stdout=output, stderr=subprocess.STDOUT)
    if result.returncode:
        raise ValueError("Photara did not close normally. The disposable files remain preserved; inspect " + str(log))
    # Deliberately leave both images mounted. Reopening uses the same pins.


if __name__ == "__main__":
    if len(sys.argv) > 1:
        parser = argparse.ArgumentParser(description=__doc__)
        for name in ("executable", "manifest", "binding", "target-bindings", "database-registration", "output"):
            parser.add_argument("--" + name, type=Path, required=True)
        create(parser.parse_args())
    else:
        try:
            launch()
        except Exception as error:
            # Fixed user-facing text; diagnostics stay in stderr, not script code.
            print(str(error), file=sys.stderr)
            subprocess.run(["/usr/bin/osascript", "-e",
                'display alert "Disposable Library session unavailable" message "The registered disposable session or application changed. No personal Library was opened. Preserve its files and ask to reopen the registered session." as critical'], check=False)
            sys.exit(1)
