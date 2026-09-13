#!/usr/bin/env python3
"""Check synthetic fixture bytes with the actual Rust canonical codec; no database I/O."""
from pathlib import Path
import hashlib
import json
import subprocess

root = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(
    ['cargo', 'metadata', '--offline', '--no-deps', '--format-version', '1'], cwd=root
))
binary = Path(metadata['target_directory']) / 'debug/examples/fixture_codec'
if not binary.is_file():
    raise SystemExit('Build the codec: cargo build --offline -p photara-core --example fixture_codec')
codec = subprocess.Popen([str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
checked = 0

def canonical(value):
    codec.stdin.write(json.dumps(value, ensure_ascii=False, separators=(',', ':')) + '\n')
    codec.stdin.flush()
    return json.loads(codec.stdout.readline())['utf8']

def verify(value):
    global checked
    if isinstance(value, list):
        for child in value:
            verify(child)
    elif isinstance(value, dict):
        for key in ['utf8', 'body_utf8', 'canonical_utf8']:
            if key not in value:
                continue
            raw = value[key].encode()
            if 'sha256' in value:
                assert hashlib.sha256(raw).hexdigest() == value['sha256']
            if 'byte_length' in value:
                assert str(len(raw)) == str(value['byte_length'])
            if 'canonical_hex' in value:
                assert raw.hex() == value['canonical_hex']
            try:
                parsed = json.loads(value[key])
            except ValueError:
                pass  # Explicit synthetic blob bytes.
            else:
                assert canonical(parsed) == value[key]
            checked += 1
        if 'manifest_utf8' in value:
            manifest = json.loads(value['manifest_utf8'])
            assert canonical(manifest) == value['manifest_utf8']
            assert hashlib.sha256(value['manifest_utf8'].encode()).hexdigest() == value['manifest_sha256']
            members = {item['path']: item for item in value['members']}
            for member in manifest['members']:
                actual = members[member['path']]
                assert actual['sha256'] == member['sha256']
                assert actual['byte_length'] == member['byte_length']
            checked += 1
        for child in value.values():
            verify(child)

files = sorted((root / 'docs/fixtures/generation-two').glob('*.json'))
for path in files:
    value = json.loads(path.read_text())
    # Every container now uses the real Rust encoding too.
    assert canonical(value).encode() == path.read_bytes(), path
    verify(value)
codec.stdin.close()
assert codec.wait() == 0
print(f'Fixture verification passed: {len(files)} canonical containers, {checked} embedded byte records')
