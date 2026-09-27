#!/usr/bin/env python3
"""Fixed linked operation bytes, independently assembled without the Rust codec."""
import copy
import hashlib
import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
OUT = Path(__file__).with_name('linked-operations.json')
PROJECT = '10000000-0000-4000-8000-000000000001'
LIBRARY = '10000000-0000-4000-8000-000000000002'
JOURNAL = '50000000-0000-4000-8000-000000000001'
TIME = '2026-09-27T00:00:00.000Z'
FEATURES = ['photara.asset-set.v2', 'photara.context.v1', 'photara.history.v1', 'photara.immutable-objects.v1', 'photara.library-project.v1', 'photara.node-contract.v2', 'photara.resources.v2']
OP_IDS = ['50000000-0000-4000-8000-000000000030', '50000000-0000-4000-8000-000000000020', '50000000-0000-4000-8000-000000000010']

def canon(v):
    return json.dumps(v, sort_keys=True, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()
def sha(b):
    return hashlib.sha256(b).hexdigest()
def ref(v):
    b = canon(v)
    return {'kind': 'json', 'sha256': sha(b), 'byte_length': str(len(b))}
def standard(schema, version=1, **values):
    return dict(schema={'version': version, 'id': schema}, project_id=PROJECT, **values)
def candidate(suffix, **values):
    return standard('example.ps2.' + suffix, extensions={}, **values)
def reproject(v):
    if isinstance(v, list): return [reproject(x) for x in v]
    if isinstance(v, dict): return {k: reproject(x) for k, x in v.items()}
    if v == '62000000-0000-4000-8000-000000020000': return PROJECT
    if v == '62000000-0000-4000-8000-000000020001': return LIBRARY
    return v

def source_objects():
    source = json.loads((ROOT / 'docs/fixtures/generation-two/d19-package-specimen.json').read_text())
    records = [reproject(json.loads(f['utf8'])) for f in source['files'] if f['path'].endswith('.json')]
    get = lambda schema: copy.deepcopy(next(v for v in records if v.get('schema', {}).get('id') == schema))
    graph = get('photara.project.saved-graph')
    graph_id, node_id = graph['graph_id'], graph['graph']['nodes'][0]['id']
    out = {}
    for key, schema, field in [('parties','photara.project.party-assignments','assignments'),('locations','photara.project.location-assignments','assignments'),('assets','photara.project.asset-ledger','assets')]:
        out[key] = standard(schema, **{field: []})
    out['resources'] = standard('photara.project.resource-ledger', 1, managed_resources=[], external_resources=[], artifacts=[])
    for name, scope in [('context', {'kind':'project','project_id':PROJECT}), ('node-context', {'kind':'node','project_id':PROJECT,'graph_id':graph_id,'node_id':node_id}), ('graph-context', {'kind':'graph','project_id':PROJECT,'graph_id':graph_id})]:
        out[name] = standard('photara.context.authored', scope=scope, variables=[], expressions=[], captures=[], metadata_selections=[], node_contexts=[])
    out['graph-context']['node_contexts'] = [{'node_id':node_id,'context':ref(out['node-context'])}]
    out['manifest-object'] = get('photara.node.manifest')
    graph['context'] = ref(out['graph-context'])
    graph['required_packages'][0]['manifest'] = ref(out['manifest-object'])
    graph['node_contracts'][0]['manifest'] = ref(out['manifest-object'])
    out['graph-0'] = graph
    authored = get('photara.project.authored')
    for field, key in [('party_assignments','parties'),('location_assignments','locations'),('asset_ledger','assets'),('resource_ledger','resources'),('context','context')]: authored[field] = ref(out[key])
    authored['graphs'][0]['document'] = ref(graph)
    out['authored-0'] = authored
    out['history'] = standard('photara.project.history', 2, **{k:[] for k in ['runs','operations','evidence','snapshots','metadata_observations','proposals','apply_receipts','artifacts']})
    return out

objects = source_objects()
manifest = {'format':'photara.project-package','format_version':{'major':1,'minor':1},'project_id':PROJECT,'created_at':'2026-09-12T00:00:00.000Z','canonical_json':'photara.canonical-json.v1','required_features':FEATURES}
BOOTSTRAP = sha(canon(manifest))
base_inventory = standard('photara.package.inventory', objects=sorted([ref(v) for v in objects.values()], key=lambda r:(r['sha256'],int(r['byte_length']))))
base_commit = standard('photara.package.commit', commit_id='50000000-0000-4000-8000-000000000002', package_revision='1', parent=None, write_id='50000000-0000-4000-8000-000000000003', created_at='2026-09-12T00:00:00.000Z', bootstrap_sha256=BOOTSTRAP, minimum_reader={'major':1,'minor':1},required_features=FEATURES,authored=ref(objects['authored-0']),history=ref(objects['history']),inventory=ref(base_inventory))
base_head = standard('photara.package.head',commit_id=base_commit['commit_id'],commit_sha256=sha(canon(base_commit)))
base_files = {f"objects/json/sha256/{sha(canon(v))}.json":canon(v).decode() for v in [*objects.values(),base_inventory]}
base_files.update({'HEAD.json':canon(base_head).decode(),'manifest.json':canon(manifest).decode(),f"commits/{base_commit['commit_id']}.json":canon(base_commit).decode()})
for n, (x,y) in enumerate([(5,6),(9,10)],1):
    graph = copy.deepcopy(objects[f'graph-{n-1}'])
    graph['graph']['revision'] = n
    graph['graph']['nodes'][0]['photara.graph-position'] = {'x':x,'y':y}
    graph['updated_at'] = TIME
    objects[f'graph-{n}'] = graph
    authored = copy.deepcopy(objects[f'authored-{n-1}'])
    authored['authored_revision'] = str(n+1)
    authored['updated_at'] = TIME
    authored['graphs'][0]['document'] = ref(graph)
    objects[f'authored-{n}'] = authored

def coordinate(n):
    graph, authored = objects[f'graph-{n}'], objects[f'authored-{n}']
    payload = sha(canon(graph['graph']))
    return {'revision':authored['authored_revision'],'authored_digest':sha(canon(authored)),'graphs':{graph['graph_id']:{'revision':str(graph['graph']['revision']),'semantic_digest':payload,'payload_digest':payload,'envelope_digest':sha(canon(graph))}}}
def initial_prefix():
    return sha(canon({'domain':'photara.package.accepted-prefix.v1','project_id':PROJECT,'library_id':LIBRARY,'bootstrap_sha256':BOOTSTRAP,'through_ordinal':'0'}))
provenance = {'principal':{'kind':'account','account_id':'50000000-0000-4000-8000-000000000050'},'actor':{'kind':'photara.gui','actor_id':'50000000-0000-4000-8000-000000000051'},'grantor':{'kind':'account','account_id':'50000000-0000-4000-8000-000000000050'},'grant_ref':{'kind':'photara.project-grant','reference_id':'50000000-0000-4000-8000-000000000052'},'effective_scope':{'project_id':PROJECT,'actions':71},'policy_decision_sha256':sha(b'fixed decision evidence; not a live authorization')}
intents, receipts, frames, prefixes = [], [], [], [initial_prefix()]
journal_prefixes = [sha(canon({'domain':'photara.package.journal-prefix.v1','journal_id':JOURNAL,'through_sequence':'0'}))]
for ordinal,(before,after,x,y,sequence) in enumerate([(0,1,5,6,3),(1,1,5,6,7),(1,2,9,10,11)],1):
    operation = OP_IDS[ordinal-1]
    g = objects[f'graph-{before}']['graph']
    command = {'kind':'graph','envelope':{'command_id':operation,'graph_id':g['id'],'expected_revision':g['revision'],'command':{'kind':'set-node-position','node_id':g['nodes'][0]['id'],'x':x,'y':y}}}
    intent = {'domain':'photara.package.operation-intent.v1','version':1,'project_id':PROJECT,'library_id':LIBRARY,'bootstrap_sha256':BOOTSTRAP,'operation_id':operation,'expected':coordinate(before),'command':command,'updated_at':TIME,'undo_group_id':None,'boundary':'single'}
    receipt = standard('photara.package.operation-receipt', library_id=LIBRARY,bootstrap_sha256=BOOTSTRAP,operation_id=operation,request_sha256=sha(canon(intent)),acceptance_ordinal=str(ordinal),journal_id=JOURNAL,journal_sequence=str(sequence),outcome='accepted',before={'revision':coordinate(before)['revision'],'digest':coordinate(before)['authored_digest']},after={'revision':coordinate(after)['revision'],'digest':coordinate(after)['authored_digest']},undo_group_id=None,provenance=provenance,extensions={})
    frame = {'schema':{'id':'photara.package.accepted-journal-frame','version':1},'journal_id':JOURNAL,'sequence':str(sequence),'operation_id':operation,'request_sha256':receipt['request_sha256'],'receipt_sha256':ref(receipt)['sha256']}
    prefixes.append(sha(canon({'domain':'photara.package.accepted-prefix-link.v1','previous_sha256':prefixes[-1],'acceptance_ordinal':str(ordinal),'operation_id':operation,'request_sha256':receipt['request_sha256'],'receipt_sha256':ref(receipt)['sha256']})))
    journal_prefixes.append(sha(canon({'domain':'photara.package.journal-prefix-link.v1','previous_sha256':journal_prefixes[-1],'frame_sha256':sha(canon(frame))})))
    intents.append(intent);receipts.append(receipt);frames.append(frame)

pool = {}
def save(name,v):
    b=canon(v);pool[name]={'input':v,'canonical':b.decode(),'byte_length':len(b),'sha256':sha(b)}
    return ref(v)
for name,v in objects.items(): save(name,v)
for name,v in [('manifest',manifest),('base-inventory',base_inventory),('base-commit',base_commit),('base-head',base_head)]:save(name,v)
for i,(intent,receipt,frame) in enumerate(zip(intents,receipts,frames),1):
    save(f'intent-{i}',intent);save(f'receipt-{i}',receipt);save(f'journal-frame-{i}',frame)
indexes={}
for count in [2,3]:
    entries=[{'operation_id':r['operation_id'],'request_sha256':r['request_sha256'],'acceptance_ordinal':r['acceptance_ordinal'],'receipt':ref(r)} for r in receipts[:count]]
    ids=candidate('operation-id-leaf',count=str(count),entries=sorted(entries,key=lambda e:e['operation_id']))
    ordinals=candidate('operation-ordinal-leaf',count=str(count),entries=entries)
    root=candidate('operation-index-tree',library_id=LIBRARY,bootstrap_sha256=BOOTSTRAP,accepted={'through_ordinal':str(count),'prefix_sha256':prefixes[count]},by_id=save(f'id-index-{count}',ids),by_ordinal=save(f'ordinal-index-{count}',ordinals))
    indexes[count]=save(f'operation-root-{count}',root)
# Recovery's packed prefix precedes active-only data. Deleting the latter cannot
# affect recovery's indexes, original receipts or authored bytes.
shared_names=[k for k in objects if k not in {f'{kind}-{n}' for kind in ['authored','graph'] for n in range(3)}]+['graph-1','authored-1','receipt-1','receipt-2','id-index-2','ordinal-index-2','operation-root-2']
active_names=['graph-2','authored-2','receipt-3','id-index-3','ordinal-index-3','operation-root-3']
pack=bytearray();locations={};allocation='60000000-0000-4000-8000-000000000001'
for name in shared_names+active_names:
    body=pool[name]['canonical'].encode();frame=b'PS2PKD01'+bytes([1,0,0,0])+struct.pack('<I',len(body))+body
    pool[name].update(frame_hex=frame.hex(),frame_sha256=sha(frame),frame_length=len(frame))
    locations[pool[name]['sha256']]={'allocation_id':allocation,'arena':'data','offset':str(len(pack)),'byte_length':str(len(frame)),'record_sha256':sha(frame),'object':ref(pool[name]['input'])}
    pack.extend(frame)
    if name=='operation-root-2': recovery_end=len(pack)
roots={}
for role,count,n in [('active',3,2),('recovery',2,1)]:
    roots[role]={'authored':ref(objects[f'authored-{n}']),'history':ref(objects['history']),'operation_index':indexes[count],'accepted':{'through_ordinal':str(count),'prefix_sha256':prefixes[count]},'journal_inclusion':{'journal_id':JOURNAL,'through_sequence':receipts[count-1]['journal_sequence'],'prefix_sha256':journal_prefixes[count],'resulting_authored_revision':str(n+1),'resulting_authored_sha256':ref(objects[f'authored-{n}'])['sha256']}}
# A separate lookup specimen stores actual locator entries, not an authority.
locator=candidate('locator-leaf',count=str(len(locations)),entries=sorted([{'object':v['object'],'membership':'semantic','physical':{k:x for k,x in v.items() if k!='object'}} for v in locations.values()],key=lambda e:(e['object']['sha256'],int(e['object']['byte_length']))))
save('locator',locator)
output={'status':'unfrozen-real-core-operation-candidate','records':pool,'base_files':base_files,'selected_roots':roots,'pack_hex':bytes(pack).hex(),'pack_sha256':sha(pack),'allocation_id':allocation,'recovery_pack_end':recovery_end,'selected_record_names':shared_names+active_names,'original_provenance':provenance,'expected_coordinates':[coordinate(n) for n in range(3)],'expected_accepted_prefixes':prefixes,'expected_journal_prefixes':journal_prefixes}
OUT.write_text(json.dumps(output,indent=2,ensure_ascii=False)+'\n')
print(OUT,OUT.stat().st_size,sha(OUT.read_bytes()))
