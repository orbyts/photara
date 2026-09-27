#!/usr/bin/env python3
"""Independent bounded tree bytes. No Rust serializer or reader is invoked."""
import hashlib
import json
import struct
from pathlib import Path
HERE = Path(__file__).resolve().parent
SOURCE = HERE.parent / 'operations/linked-operations.json'
PROJECT = '10000000-0000-4000-8000-000000000001'
DOMAIN = '30000000-0000-4000-8000-000000000001'
DATA = '70000000-0000-4000-8000-000000000001'
META = '70000000-0000-4000-8000-000000000002'
FEATURE = 'example.ps2.typed-branches.draft-v1'
def canonical(v): return json.dumps(v, sort_keys=True, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()
def sha(b): return hashlib.sha256(b).hexdigest()
def ref(v):
    b = canonical(v)
    return dict(kind='json', sha256=sha(b), byte_length=str(len(b)))
def key(r): return r['sha256'], int(r['byte_length'])
def node(kind, **fields): return dict(schema=dict(id='example.ps2.' + kind, version=1), project_id=PROJECT, extensions={}, **fields)
source = json.loads(SOURCE.read_text())
records = {}
by_key = {key(r['reference'] if 'reference' in r else ref(r['input'])): r['input'] for r in source['records'].values()}
def add(v, tag=1):
    r = ref(v); records[key(r)] = (v, tag); return r
def descend(r, seen):
    k = key(r)
    if k in seen: return
    v = records[k][0] if k in records else by_key[k]
    seen[k] = r
    def walk(value):
        if isinstance(value, dict):
            if set(value) == {'kind', 'sha256', 'byte_length'}: descend(value, seen)
            else:
                for item in value.values(): walk(item)
        elif isinstance(value, list):
            for item in value: walk(item)
    walk(v)
    add(v)
def tree(kind, entries, leaf_size):
    charge = kind == 'owned-charge'
    def entry_key(e):
        if kind == 'inventory': return e
        if kind == 'operation-id': return e['operation_id']
        if kind == 'operation-ordinal': return e['acceptance_ordinal']
        return e['allocation_id']
    summaries = []
    for start in range(0, len(entries), leaf_size):
        part = entries[start:start+leaf_size]
        fields = dict(count=str(len(part)), entries=part)
        if charge: fields['charged_high_water'] = str(sum(int(e['charged_high_water']) for e in part))
        v = node(kind + '-leaf', **fields)
        s = dict(first=entry_key(part[0]), last=entry_key(part[-1]), count=str(len(part)), child=add(v, 2 if charge else 1))
        if charge: s['charged_high_water'] = fields['charged_high_water']
        summaries.append(s)
    while len(summaries) > 1:
        next_level = []
        for start in range(0, len(summaries), 2):
            part = summaries[start:start+2]
            if len(part) == 1:
                next_level.append(part[0]); continue
            fields = dict(count=str(sum(int(c['count']) for c in part)), children=part)
            if charge: fields['charged_high_water'] = str(sum(int(c['charged_high_water']) for c in part))
            v = node(kind + '-branch', **fields)
            s = dict(first=part[0]['first'], last=part[-1]['last'], count=fields['count'], child=add(v, 2 if charge else 1))
            if charge: s['charged_high_water'] = fields['charged_high_water']
            next_level.append(s)
        summaries = next_level
    return summaries[0]['child']
roles = {}
expected = {}
for role in ['active', 'recovery']:
    old = source['selected_roots'][role]
    index = json.loads(json.dumps(by_key[key(old['operation_index'])]))
    for field, kind in [('by_id','operation-id'),('by_ordinal','operation-ordinal')]:
        entries = by_key[key(index[field])]['entries']
        index[field] = tree(kind, entries, 1)
    selected = dict(old, operation_index=add(index))
    closure = {}
    for field in ['authored', 'history', 'operation_index']: descend(selected[field], closure)
    entries = [closure[k] for k in sorted(closure)]
    selected['inventory'] = tree('inventory', entries, 3)
    roles[role] = selected
    expected[role] = entries
charges = []
for n in range(1, 7):
    allocation = f'71000000-0000-4000-8000-{n:012d}'
    v = node('sealed-charge', allocation_id=allocation, arena='data' if n % 2 else 'metadata', domain_incarnation=DOMAIN, measured_extent=str(n*100), charged_high_water='4096', observation=dict(kind='synthetic-vector', observation_id=f'72000000-0000-4000-8000-{n:012d}'))
    charges.append(dict(allocation_id=allocation, charge=add(v,2), charged_high_water='4096'))
charge_root = tree('owned-charge', charges, 1)
# Every node and charge lives in a real tagged frame; no lifetime loose charge files.
pack = bytearray(); locations = []
for k, (v, tag) in sorted(records.items()):
    body = canonical(v)
    frame = b'PS2PKD01' + bytes([tag,0,0,0]) + struct.pack('<I',len(body)) + body
    locations.append(dict(object=ref(v), membership='ownership' if tag == 2 else 'semantic', physical=dict(allocation_id=DATA, arena='data', offset=str(len(pack)), byte_length=str(len(frame)), record_sha256=sha(frame))))
    pack.extend(frame)
def pad(pack, extent):
    count = extent - len(pack) - 16
    assert count > 0
    pack.extend(b'PS2PKD01' + bytes(4) + struct.pack('<I',count) + bytes(count))
pad(pack,65536)
locator = node('locator-leaf',count=str(len(locations)),entries=locations)
body = canonical(locator)
meta = bytearray(b'PS2PKD01' + bytes([3,0,0,0]) + struct.pack('<I',len(body)) + body)
locator_ref = dict(allocation_id=META,arena='metadata',offset='0',byte_length=str(len(meta)),record_sha256=sha(meta))
pad(meta,65536)
selector = dict(kind='typed-branches-draft',minimum_reader=dict(major=1,minor=3),required_features=['example.ps2.scalable-storage.draft-v1',FEATURE],roles=roles,owned_charges=charge_root,domain_incarnation=DOMAIN,tips=[dict(allocation_id=a,arena=arena,owned_extent='65536',charged_high_water='65536') for a,arena in [(DATA,'data'),(META,'metadata')]],sealed_observations=[dict(allocation_id=e['allocation_id'],**{k:records[key(e['charge'])][0][k] for k in ['arena','domain_incarnation','measured_extent','charged_high_water']}) for e in charges],total_charge=str(2*65536+6*4096),locator=locator_ref)
output = dict(format='example.ps2.linked-branches.draft-v1',operation_corpus_sha256=sha(SOURCE.read_bytes()),selector=selector,expected_semantic=expected,records={k[0]:dict(input=v,canonical_utf8=canonical(v).decode(),sha256=k[0],byte_length=str(k[1])) for k,(v,_) in records.items()},locator=dict(input=locator,canonical_utf8=canonical(locator).decode(),sha256=sha(canonical(locator))),allocations={DATA:dict(arena='data',hex=pack.hex(),sha256=sha(pack)),META:dict(arena='metadata',hex=meta.hex(),sha256=sha(meta))})
(HERE/'linked-branches.json').write_text(json.dumps(output,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
print(f'{len(records)} records, {len(canonical(locator))}B locator, source {output["operation_corpus_sha256"]}')
