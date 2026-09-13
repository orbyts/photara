#!/usr/bin/env python3
"""Reject retired Photara domain naming; permit only Cargo and Apple API syntax."""
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[1]
paths = subprocess.check_output(
    ['git', 'ls-files', '-co', '--exclude-standard'], cwd=root, text=True
).splitlines()
pattern = re.compile('work' + 'space', re.I)
errors = []
checked = 0
for name in sorted(set(paths)):
    path = root / name
    if not path.is_file() or path.name in {'Cargo.toml', 'Cargo.lock'}:
        continue
    if path.suffix not in {'.rs', '.swift', '.sql', '.md', '.json', '.py', '.sh'}:
        continue
    checked += 1
    for number, line in enumerate(path.read_text().splitlines(), 1):
        if name == 'docs/architecture/LIBRARY_REBASELINE_INVENTORY.md' and line.startswith('- Removed old path:'):
            continue  # Exact removed-path provenance; never forward source/schema vocabulary.
        # These are exact upstream build/platform APIs, not Photara concepts.
        source = line.replace('--work' + 'space', '').replace('NSWork' + 'space', '')
        if pattern.search(source):
            errors.append(f'{name}:{number}')
if errors:
    raise SystemExit('Retired domain vocabulary: ' + ', '.join(errors))
print(f'Naming guard passed: {checked} source/schema/document/fixture files')
