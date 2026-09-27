#!/usr/bin/env python3
"""Independent fixed vectors; Python stdlib only, never invokes the Rust codec.
All proposed IDs/layouts are experimental. Run from any directory to reproduce.
"""
import hashlib
import json
import struct
from pathlib import Path

OUT = Path(__file__).with_name('linked-vectors.json')
PROJECT = '10000000-0000-4000-8000-000000000001'
LIBRARY = '10000000-0000-4000-8000-000000000002'
COMMIT = '10000000-0000-4000-8000-000000000003'
FEATURE = 'example.ps2.scalable-storage.draft-v1'
MAGIC = b'PS2PKD01'
IDS = {k: f'20000000-0000-4000-8000-{i:012d}' for i, k in enumerate(['shared', 'active-data', 'active-meta', 'recovery-data', 'recovery-meta'], 1)}

def canon(v):
    return json.dumps(v, sort_keys=True, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()

def sha(b):
    return hashlib.sha256(b).hexdigest()

def ref(v):
    b = canon(v)
    return {'kind': 'json', 'sha256': sha(b), 'byte_length': str(len(b))}

def key(r):
    return r['sha256'], int(r['byte_length'])

def record(schema_name, **fields):
    # Input insertion order deliberately differs from canonical sorted keys.
    return dict(schema={'version': 1, 'id': 'example.ps2.' + schema_name}, project_id=PROJECT, extensions={}, **fields)

def frame(v, tag):
    b = canon(v)
    return MAGIC + bytes([tag, 0, 0, 0]) + struct.pack('<I', len(b)) + b

class Build:
    def __init__(self):
        self.records = {}
        self.allocations = {}
        self.loose = {}

    def save(self, name, v, tag=None):
        b = canon(v)
        entry = {'input': v, 'canonical': b.decode(), 'byte_length': len(b), 'sha256': sha(b)}
        if tag is not None:
            f = frame(v, tag)
            entry.update(frame_hex=f.hex(), frame_length=len(f), frame_sha256=sha(f), frame_tag=tag)
        self.records[name] = entry
        return ref(v)

    def loose_add(self, name, v):
        r = self.save(name, v)
        self.loose[r['sha256']] = name
        return r

    def pack(self, name, arena, entries):
        blob = bytearray()
        result = {}
        for label, v, tag in entries:
            self.save(label, v, tag)
            f = frame(v, tag)
            result[label] = {'allocation_id': IDS[name], 'arena': arena, 'offset': str(len(blob)), 'byte_length': str(len(f)), 'record_sha256': sha(f)}
            blob.extend(f)
        self.allocations[IDS[name]] = {'arena': arena, 'hex': bytes(blob).hex(), 'sha256': sha(blob), 'byte_length': len(blob)}
        return result


def build(mode='valid'):
    b = Build()
    manifest = {'format': 'photara.project-package', 'format_version': {'major': 1, 'minor': 1}, 'project_id': PROJECT, 'created_at': '2026-09-27T00:00:00.000Z', 'canonical_json': 'photara.canonical-json.v1', 'required_features': []}
    b.save('manifest', manifest)
    bootstrap = sha(canon(manifest))
    shared = record('probe-leaf', label='shared \u03bb', links=[])
    # This unavailable ref is deliberately present only in an optional extension.
    missing = {'kind': 'json', 'sha256': 'f' * 64, 'byte_length': '19'}
    shared['extensions'] = {'example.probe': {'apparently_a_ref': missing}}
    if mode == 'extension-promoted-to-edge':
        shared['links'] = [missing]
    history = record('history-probe', retained=[])
    operations = record('operation-index-root', accepted={'through_ordinal': '0', 'prefix_sha256': sha(canon({'domain': 'photara.package.accepted-prefix.v1', 'project_id': PROJECT, 'library_id': LIBRARY, 'bootstrap_sha256': bootstrap, 'through_ordinal': '0'}))}, operation_ids=[], ordinals=[])
    shared_entries = [('shared', shared, 1), ('history', history, 1), ('operations', operations, 1)]
    shared_refs = {n: ref(v) for n, v, _ in shared_entries}
    shared_locations = b.pack('shared', 'data', shared_entries)
    sealed_end = b.allocations[IDS['shared']]['byte_length']
    charge = record('sealed-charge', allocation_id=IDS['shared'], arena='data', domain_incarnation='30000000-0000-4000-8000-000000000001', measured_extent=str(sealed_end), charged_high_water=str((sealed_end + 4095) // 4096 * 4096), observation={'kind': 'synthetic-vector', 'observation_id': '30000000-0000-4000-8000-000000000002'})
    charge_ref = b.loose_add('sealed-charge', charge)
    alternate_charge = dict(charge, charged_high_water=str(int(charge['charged_high_water']) + 4096))
    alternate_ref = b.loose_add('alternate-charge', alternate_charge) if mode == 'inconsistent-charge' else None
    roots, selectors, closures = {}, {}, {}
    for role in ['active', 'recovery']:
        authored = record('probe-leaf', label=role, links=[shared_refs['shared']])
        if mode == 'unknown-required-field' and role == 'active':
            authored['unknown_required'] = True
        logical = sorted([ref(authored), *shared_refs.values()], key=key)
        inventory = record('inventory-root', count=str(len(logical)), entries=logical)
        if mode == 'flat-scalable-confusion' and role == 'active':
            inventory['schema']['id'] = 'photara.package.inventory'
        state = record('state-root', library_id=LIBRARY, bootstrap_sha256=bootstrap, root_id='40000000-0000-4000-8000-' + ('000000000001' if role == 'active' else '000000000002'), authored_revision='2' if role == 'active' else '1', authored=ref(authored), history=shared_refs['history'], resource_state=None, operation_index=shared_refs['operations'], accepted=operations['accepted'], journal_inclusion=None, predecessor={'root_sha256': 'e' * 64, 'authored_revision': '0'}, inventory=ref(inventory))
        semantic = [(role + '-authored', authored, 1), (role + '-inventory', inventory, 1), (role + '-state', state, 1)]
        extent = meta_extent = 16384
        claims = []
        for allocation, arena, owned, sealed in [('shared', 'data', sealed_end, True), (role + '-data', 'data', extent, False), (role + '-meta', 'metadata', meta_extent, False)]:
            claim = record('ownership-claim', allocation_id=IDS[allocation], arena=arena, owned_extent=str(owned), authenticated_prefix={'byte_length': str(sealed_end if sealed else 0), 'sha256': b.allocations[IDS['shared']]['sha256'] if sealed else sha(b'')}, sealed=sealed, sealed_charge=(alternate_ref if role == 'recovery' and mode == 'inconsistent-charge' else charge_ref) if sealed else None)
            claims.append((role + '-claim-' + allocation, claim, 2))
        if mode == 'missing-metadata-ownership' and role == 'recovery':
            claims = claims[:-1]
        # Claim leaves are written before their direct branch; no hash cycle.
        first = b.pack(role + '-data', 'data', semantic + claims)
        children = sorted([{'allocation_id': v['allocation_id'], 'node': first[n]} for n, v, _ in claims], key=lambda x: x['allocation_id'])
        owner = record('ownership-branch', count=str(len(children)), children=children)
        own_name = role + '-ownership'
        extras = [(role + '-unreachable-owner', dict(claims[0][1], extensions={'example.unreachable': True}), 2)] if mode == 'extra-owner-locator' and role == 'active' else []
        full = b.pack(role + '-data', 'data', semantic + claims + [(own_name, owner, 2)] + extras)
        locator_entries = []
        if mode == 'reference-to-padding' and role == 'active':
            start = b.allocations[IDS[role + '-data']]['byte_length']
            padding = MAGIC + bytes([0, 0, 0, 0]) + struct.pack('<I', 16384 - start - 16) + bytes(16384 - start - 16)
            full[role + '-authored'] = {'allocation_id': IDS[role + '-data'], 'arena': 'data', 'offset': str(start), 'byte_length': str(len(padding)), 'record_sha256': sha(padding)}
        for n, v, tag in shared_entries + semantic + claims + [(own_name, owner, 2)] + extras:
            location = shared_locations[n] if n in shared_locations else full[n]
            locator_entries.append({'object': ref(v), 'membership': 'semantic' if tag == 1 else 'ownership', 'physical': location})
        locator_entries.sort(key=lambda e: key(e['object']))
        if mode == 'wrong-membership' and role == 'active':
            next(e for e in locator_entries if e['object'] == ref(authored))['membership'] = 'ownership'
        if mode == 'noncanonical-offset' and role == 'active':
            next(e for e in locator_entries if e['object'] == ref(authored))['physical']['offset'] = '00'
        if mode == 'dangling-locator' and role == 'active':
            next(e for e in locator_entries if e['object'] == ref(authored))['physical']['allocation_id'] = '20000000-0000-4000-8000-000000000099'
        if mode == 'recovery-borrows-active' and role == 'recovery':
            next(e for e in locator_entries if e['object'] == ref(authored))['physical'] = selectors['active']['authored_physical']
        midpoint = len(locator_entries) // 2
        leaves = []
        for index, entries in enumerate([locator_entries[:midpoint], locator_entries[midpoint:]]):
            leaves.append((role + '-locator-leaf-' + str(index), record('locator-leaf', count=str(len(entries)), entries=entries), 3))
        leaf_locations = b.pack(role + '-meta', 'metadata', leaves)
        ranges = [{'first': entries['entries'][0]['object'], 'last': entries['entries'][-1]['object'], 'count': entries['count'], 'child': leaf_locations[n]} for n, entries, _ in leaves]
        branch = record('locator-branch', count=str(len(locator_entries)), children=ranges)
        meta = b.pack(role + '-meta', 'metadata', leaves + [(role + '-locator', branch, 3)])
        # Finite finalization extent, closed by a zero-padding frame. Padding is
        # a nonedge and cannot be selected by a PhysicalRef. No size fixedpoint.
        for allocation in [role + '-data', role + '-meta']:
            stored = b.allocations[IDS[allocation]]
            raw = bytes.fromhex(stored['hex'])
            gap = 16384 - len(raw)
            if gap < 16:
                raise ValueError('finite vector corridor exhausted')
            raw += MAGIC + bytes([0, 0, 0, 0]) + struct.pack('<I', gap - 16) + bytes(gap - 16)
            stored.update(hex=raw.hex(), sha256=sha(raw), byte_length=len(raw))
        roots[role] = ref(state)
        selectors[role] = {'locator': meta[role + '-locator'], 'ownership': full[own_name], 'authored_physical': full[role + '-authored']}
        closures[role] = sorted([ref(state), ref(inventory), *logical], key=key)
    tips = []
    for name in ['active-data', 'active-meta', 'recovery-data', 'recovery-meta']:
        a = b.allocations[IDS[name]]
        tips.append({'allocation_id': IDS[name], 'arena': a['arena'], 'owned_extent': str(a['byte_length']), 'charged_high_water': str((a['byte_length'] + 4095) // 4096 * 4096)})
    tips.sort(key=lambda x: x['allocation_id'])
    accounting = record('physical-accounting', domain_incarnation=charge['domain_incarnation'], standing_control='65536', tips=tips, sealed_charges=[charge_ref], unresolved=[], total_charge=str(65536 + int(charge['charged_high_water']) + sum(int(t['charged_high_water']) for t in tips)))
    accounting_ref = b.loose_add('accounting', accounting)
    union = {key(r): r for r in closures['active'] + closures['recovery'] + [accounting_ref, charge_ref]}
    root_inventory = record('inventory-root', count=str(len(union)), entries=[union[k] for k in sorted(union)])
    root_inventory_ref = b.loose_add('root-inventory', root_inventory)
    placement = {'generation': '1', 'active_locator': selectors['active']['locator'], 'recovery_locator': selectors['recovery']['locator'], 'active_ownership': selectors['active']['ownership'], 'recovery_ownership': selectors['recovery']['ownership'], 'accounting': accounting_ref}
    root_set = record('root-set', library_id=LIBRARY, bootstrap_sha256=bootstrap, kind='scalable-draft', active=roots['active'], recovery=roots['recovery'], pinned_roots=[], operation_index=shared_refs['operations'], conversion_source=None, inventory=root_inventory_ref, placement=placement)
    features = [FEATURE, 'photara.sealed-roots.v1']
    if mode == 'missing-capability':
        features.remove(FEATURE)
    commit = {'schema': {'id': 'photara.package.commit', 'version': 1}, 'project_id': PROJECT, 'commit_id': COMMIT, 'package_revision': '2', 'parent': None, 'write_id': '10000000-0000-4000-8000-000000000004', 'created_at': '2026-09-27T00:00:00.000Z', 'minimum_reader': {'major': 1, 'minor': 3}, 'required_features': features, 'authored': ref(b.records['active-authored']['input']), 'history': shared_refs['history'], 'inventory': ref(b.records['active-inventory']['input']), 'root_set': root_set, 'extensions': {}}
    if mode == 'wrong-bootstrap':
        root_set['bootstrap_sha256'] = 'd' * 64
    b.save('commit', commit)
    head = {'schema': {'id': 'photara.package.head', 'version': 1}, 'project_id': PROJECT, 'commit_id': COMMIT, 'commit_sha256': sha(canon(commit))}
    b.save('head', head)
    return {'expected': 'valid' if mode == 'valid' else 'refuse', 'records': b.records, 'loose': b.loose, 'allocations': b.allocations, 'expected_closures': closures, 'allocation_ids': IDS}

modes = ['valid', 'missing-capability', 'flat-scalable-confusion', 'missing-metadata-ownership', 'wrong-membership', 'inconsistent-charge', 'dangling-locator', 'extension-promoted-to-edge', 'recovery-borrows-active', 'wrong-bootstrap', 'reference-to-padding', 'extra-owner-locator', 'unknown-required-field', 'noncanonical-offset']
scenarios = {m: build(m) for m in modes}
record_pool, allocation_pool = {}, {}
for scenario in scenarios.values():
    for name, entry in list(scenario['records'].items()):
        record_pool[entry['sha256']] = entry
        scenario['records'][name] = entry['sha256']
    for allocation in scenario['allocations'].values():
        allocation_pool[allocation['sha256']] = allocation.pop('hex')
OUT.write_text(json.dumps({'status': 'unfrozen-pure-closure-candidate', 'generator': 'Python stdlib json sort_keys UTF-8 and hashlib; no Rust codec invocation', 'records': record_pool, 'allocation_bytes': allocation_pool, 'scenarios': scenarios}, indent=2, ensure_ascii=False) + '\n')
print(OUT, OUT.stat().st_size, sha(OUT.read_bytes()))
