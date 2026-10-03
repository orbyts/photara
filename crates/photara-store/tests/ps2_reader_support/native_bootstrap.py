#!/usr/bin/env python3
"""Build the existing integrated specimen with separately captured native pins.
No filesystem effects; no qualification. Frozen generator/default bytes untouched.
"""
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
GENERATOR = ROOT / 'docs/architecture/proposals/ps2/integrated/generate.py'

def build(registration):
    spec = importlib.util.spec_from_file_location('native_integrated', GENERATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    if set(registration) != {'allocations', 'retained'}:
        raise ValueError('exact native registration fields')
    expected = {module.aid(n) for n in range(1, 9)}
    if set(registration['allocations']) != expected:
        raise ValueError('exact native allocation set')
    source = module.load(module.HERE.parent / 'resource-conversion/linked.json')['scenarios']['valid']['source_files']
    if set(registration['retained']) != set(source):
        raise ValueError('exact native retained-file set')
    for group in registration.values():
        for witness in group.values():
            if set(witness) != {'device', 'inode'}:
                raise ValueError('exact witness fields')
            for value in witness.values():
                if not isinstance(value, str) or str(int(value)) != value or not 0 < int(value) <= 2**64-1:
                    raise ValueError('positive canonical native witness')
    # Explicit initial experiment registration, also used by repeatable unit tests.
    # The increased allowance is charged in the first ledger; never retroactive.
    module.STANDING = 262144
    module.witness = lambda n: dict(registration['allocations'][module.aid(n)])
    original_obj = module.obj
    def native_obj(name, version=1, **fields):
        if name == 'photara.storage.local-observation' and fields['subject']['kind'] == 'retained-file':
            fields['physical'] = dict(registration['retained']['/'.join(fields['subject']['path'])])
        return original_obj(name, version, **fields)
    module.obj = native_obj
    result = module.build()
    assert result['source_witnesses'] == registration['retained']
    assert {key: value['witness'] for key, value in result['allocations'].items()} == registration['allocations']
    return result

if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('usage: native_bootstrap.py native-registration.json')
    print(json.dumps(build(json.loads(Path(sys.argv[1]).read_text())), sort_keys=True, separators=(',', ':')))
