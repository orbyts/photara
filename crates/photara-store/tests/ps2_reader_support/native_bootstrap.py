#!/usr/bin/env python3
"""Build the existing integrated specimen with separately captured native pins.
Default generation is pure. --preallocate is an explicit fresh private-image
helper; no production qualification. Frozen generator/default bytes untouched.
"""
import importlib.util
import json
import os
import sys
from pathlib import Path
sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[4]
GENERATOR = ROOT / 'docs/architecture/proposals/ps2/integrated/generate.py'

def seed_project(project_id, destination):
    """Rebuild this same specimen under a distinct ID before native registration."""
    import shutil
    import subprocess
    import uuid
    if str(uuid.UUID(project_id)) != project_id:
        raise ValueError('canonical fixture Project ID')
    destination = Path(destination)
    if not destination.is_absolute() or destination.parent.resolve() != Path('/private/tmp'):
        raise ValueError('fresh private temporary seed directory')
    destination.mkdir(mode=0o700, exist_ok=False)
    relative = Path('docs/architecture/proposals/ps2')
    scripts = ['operations/generate.py', 'resource-conversion/generate.py',
               'resource-factored/generate.py', 'integrated/generate.py']
    files = [relative / name for name in scripts] + [
        relative / 'sealed-wire-golden.json',
        Path('docs/fixtures/generation-two/d19-package-specimen.json')]
    for name in files:
        target = destination / name
        target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        if name.suffix == '.py':
            # Change the input constant before any dependency hashes are made.
            target.write_text((ROOT / name).read_text().replace(
                '10000000-0000-4000-8000-000000000001', project_id))
        else:
            shutil.copyfile(ROOT / name, target)
    for name in scripts[:2]:
        subprocess.run([sys.executable, str(destination / relative / name)],
                       check=True, stdout=subprocess.DEVNULL)
    path = destination / relative / 'integrated/generate.py'
    spec = importlib.util.spec_from_file_location('seed_integrated', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    result = module.build()
    path.with_name('linked.json').write_text(json.dumps(result, sort_keys=True, separators=(',', ':')))
    print(destination)

def build(registration):
    seed = os.environ.get('PHOTARA_PS4_SEED_ROOT')
    generator = Path(seed) / GENERATOR.relative_to(ROOT) if seed else GENERATOR
    spec = importlib.util.spec_from_file_location('native_integrated', generator)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    if set(registration) not in ({'allocations', 'retained'}, {'allocations', 'retained', 'tip_charges'}):
        raise ValueError('exact native registration fields')
    expected = {module.aid(n) for n in range(1, 9)}
    if set(registration['allocations']) != expected:
        raise ValueError('exact native allocation set')
    source = module.load(module.HERE.parent / 'resource-conversion/linked.json')['scenarios']['valid']['source_files']
    if set(registration['retained']) != set(source):
        raise ValueError('exact native retained-file set')
    for group in (registration["allocations"], registration["retained"]):
        for witness in group.values():
            if set(witness) != {'device', 'inode'}:
                raise ValueError('exact witness fields')
            for value in witness.values():
                if not isinstance(value, str) or str(int(value)) != value or not 0 < int(value) <= 2**64-1:
                    raise ValueError('positive canonical native witness')
    # Explicit initial experiment registration, also used by repeatable unit tests.
    # The increased allowance is charged in the first ledger; never retroactive.
    module.STANDING = int(os.environ.get("PHOTARA_PS3_STANDING_BYTES", "262144"))
    if module.STANDING <= 0 or module.STANDING % 4096:
        raise ValueError("explicit aligned disposable standing allowance")
    module.witness = lambda n: dict(registration['allocations'][module.aid(n)])
    charges = registration.get('tip_charges', {})
    if charges and set(charges) != expected - {module.aid(1)}:
        raise ValueError('exact measured growable allocation charges')
    for charge in charges.values():
        if not isinstance(charge, str) or str(int(charge)) != charge or int(charge) < module.EXTENT or int(charge) % 4096:
            raise ValueError('canonical measured preallocation charge')
    original_obj = module.obj
    def native_obj(name, version=1, **fields):
        if name == 'photara.storage.local-observation' and fields['subject']['kind'] == 'retained-file':
            fields['physical'] = dict(registration['retained']['/'.join(fields['subject']['path'])])
        if name == 'photara.storage.local-observation' and fields['subject']['kind'] == 'pack' and fields['subject']['allocation_id'] in charges:
            fields['charged_high_water'] = charges[fields['subject']['allocation_id']]
        if name == 'photara.storage.ledger' and charges:
            delta = 0
            for tip in fields['tips']:
                charge = charges[tip['allocation_id']]
                delta += int(charge) - int(tip['registered_charge'])
                tip['registered_charge'] = charge
                assert tip['observation']['charged_high_water'] == charge
            fields['total_charge'] = str(int(fields['total_charge']) + delta)
        return original_obj(name, version, **fields)
    module.obj = native_obj
    result = module.build()
    if charges:
        ledgers = [json.loads(raw) for raw in result['loose'].values() if json.loads(raw).get('schema', {}).get('id') == 'photara.storage.ledger']
        assert len(ledgers) == 1
        result['expected']['total_charge'] = ledgers[0]['total_charge']
    assert result['source_witnesses'] == registration['retained']
    assert {key: value['witness'] for key, value in result['allocations'].items()} == registration['allocations']
    return result

def preallocate(directory, device, inode, requested):
    # Only the native disposable harness calls this before its first HEAD exists.
    # Python exposes fcntl without adding unsafe Rust to the production crate.
    import fcntl
    import stat
    import struct
    if sys.platform != 'darwin' or requested <= 0 or requested % 4096:
        raise ValueError('explicit macOS fixture preallocation')
    pins = json.load(sys.stdin)
    root = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        observed = os.fstat(root)
        if (observed.st_dev, observed.st_ino) != (device, inode) or observed.st_uid != os.geteuid() or observed.st_mode & 0o077:
            raise ValueError('private pinned package directory')
        for allocation, witness in sorted(pins['allocations'].items()):
            if allocation.endswith('000000001001'):
                continue
            fd = os.open('pack-' + allocation, os.O_RDWR | os.O_NOFOLLOW, dir_fd=root)
            try:
                before = os.fstat(fd)
                if (before.st_dev, before.st_ino) != (int(witness['device']), int(witness['inode'])) or not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or before.st_uid != os.geteuid() or before.st_mode & 0o077 or before.st_size != 0:
                    raise ValueError('fresh pinned allocation')
                # SDK fstore_t: flags, posmode, offset, length, bytesallocated.
                # No ftruncate: logical EOF stays unchanged. PERSIST retains the
                # measured prepaid extents when the helper closes its handle.
                fcntl.fcntl(fd, 42, struct.pack('=IIqqq', 0x04 | 0x08, 3, 0, requested, 0))
                os.fsync(fd)
                fcntl.fcntl(fd, 51)  # F_FULLFSYNC, approved disposable profile.
                after = os.fstat(fd)
                if after.st_size != 0 or after.st_blocks * 512 < requested:
                    raise ValueError('incomplete or EOF-changing preallocation')
            finally:
                os.close(fd)
        os.fsync(root)
        fcntl.fcntl(root, 51)
    finally:
        os.close(root)

if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == '--seed-project':
        seed_project(sys.argv[2], sys.argv[3])
        raise SystemExit(0)
    if len(sys.argv) == 6 and sys.argv[1] == '--preallocate':
        preallocate(sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5]))
        raise SystemExit(0)
    if len(sys.argv) != 2:
        raise SystemExit('usage: native_bootstrap.py native-registration.json')
    print(json.dumps(build(json.loads(Path(sys.argv[1]).read_text())), sort_keys=True, separators=(',', ':')))
