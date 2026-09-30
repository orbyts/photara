#!/usr/bin/env python3
"""UNFROZEN D19 same-project HEAD corpus; shared-reader integration still pending."""
import hashlib
import importlib.util
import json
import sys
from pathlib import Path
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
spec = importlib.util.spec_from_file_location('selected_blob_builder', HERE.parent/'integrated/generate.py')
g = importlib.util.module_from_spec(spec)
spec.loader.exec_module(g)
enc, sha, ref, rounded = g.enc, g.sha, g.ref, g.rounded
P='62000000-0000-4000-8000-000000020000'
L='62000000-0000-4000-8000-000000020001'
INC='97000000-0000-4000-8000-000000000001'
PROFILE='example.ps2.synthetic-local-profile-v1'
EXTENT=131072
STANDING=65536
DIRECTORY=16384 # Three allocation namespaces plus one containing directory; fixed fixture allowance.
def uid(n):return f'97000000-0000-4000-8000-{n:012d}'
def key(r):return (0 if r['kind']=='blob' else 1,r['sha256'],int(r['byte_length']))
g.P=P;g.L=L;g.INC=INC;g.uid=uid;g.key=key;g.EXTENT=EXTENT
obj=g.obj
def tree(b,name,entries,entry_key=lambda e:e,order=lambda x:x,subtotal=None):
 entries=sorted(entries,key=lambda e:order(entry_key(e)));nodes=[];level=[]
 def add(v):nodes.append(v);return b.add(v)
 leafsize=3 if name in ['photara.storage.local-observation','photara.storage.charge','photara.package.retained-file-charge'] else 4
 for i in range(0,max(1,len(entries)),leafsize):
  p=entries[i:i+leafsize];fields=dict(count=str(len(p)),entries=p)
  if subtotal:fields['charged_high_water']=str(sum(int(subtotal(e)) for e in p))
  r=add(obj(name+'-leaf',**fields));s=dict(first=entry_key(p[0]) if p else None,last=entry_key(p[-1]) if p else None,count=fields['count'],child=r)
  if subtotal:s['charged_high_water']=fields['charged_high_water']
  level.append(s)
 while len(level)>1:
  out=[]
  for i in range(0,len(level),4):
   p=level[i:i+4]
   if len(p)==1:out.extend(p);continue
   fields=dict(count=str(sum(int(c['count']) for c in p)),children=p)
   if subtotal:fields['charged_high_water']=str(sum(int(c['charged_high_water']) for c in p))
   r=add(obj(name+'-branch',**fields));s=dict(first=p[0]['first'],last=p[-1]['last'],count=fields['count'],child=r)
   if subtotal:s['charged_high_water']=fields['charged_high_water']
   out.append(s)
  level=out
 return level[0]['child'],nodes

def build(converted=False):
 global EXTENT
 EXTENT=262144 if converted else 131072;g.EXTENT=EXTENT
 original_path=ROOT/'docs/fixtures/generation-two/d19-package-specimen.json'
 original_bytes=original_path.read_bytes();original=json.loads(original_bytes)
 source={r['path']:r['utf8'].encode() for r in original['files']}
 head0=json.loads(source['HEAD.json']);commit0=json.loads(source['commits/'+head0['commit_id']+'.json']);manifest=json.loads(source['manifest.json'])
 def object_bytes(r):return source[f"objects/{'blobs' if r['kind']=='blob' else 'json'}/sha256/{r['sha256']}{'' if r['kind']=='blob' else '.json'}"]
 inv0=json.loads(object_bytes(commit0['inventory']))
 original_refs=inv0['objects'];blobs=[r for r in original_refs if r['kind']=='blob'];assert len(blobs)==1
 blob=blobs[0];raw=object_bytes(blob);assert raw==b'photo' and sha(raw)==blob['sha256']
 legacy={key(r):json.loads(object_bytes(r)) for r in original_refs if r['kind']=='json'}
 authored=legacy[key(commit0['authored'])];assert authored['project_id']==P and authored['owning_library_id']==L
 b=g.Builder('valid')
 for v in legacy.values():b.add(v)
 bootstrap=sha(source['manifest.json'])
 accepted=dict(through_ordinal='0',prefix_sha256=sha(enc(dict(domain='photara.package.accepted-prefix.v1',project_id=P,library_id=L,bootstrap_sha256=bootstrap,through_ordinal='0'))))
 ids,idnodes=tree(b,'photara.package.operation-id',[],lambda e:e['operation_id'])
 ordinals,ordnodes=tree(b,'photara.package.operation-ordinal',[],lambda e:e['acceptance_ordinal'])
 index=obj('photara.package.operation-index',version=2,library_id=L,bootstrap_sha256=bootstrap,accepted=accepted,by_id=ids,by_ordinal=ordinals)
 semantic_refs=original_refs+[ref(v) for v in idnodes+ordnodes+[index]]
 semantic,semantic_nodes=tree(b,'photara.package.inventory',semantic_refs,order=key)
 state=obj('photara.package.state-root',version=2,library_id=L,bootstrap_sha256=bootstrap,root_id=uid(101),authored_revision=authored['authored_revision'],authored=commit0['authored'],history=commit0['history'],resource_state=None,operation_index=ref(index),accepted=accepted,journal_inclusion=None,predecessor=None,inventory=semantic)
 pins=obj('photara.package.retained-root-leaf',count='0',entries=[])
 evidence=obj('photara.resource.retention-evidence-leaf',count='0',entries=[])
 holds=obj('photara.storage.hold-leaf',count='0',reserved='0',consumed='0',remaining='0',entries=[])
 tickets=obj('photara.storage.retirement-ticket-leaf',count='0',charged_high_water='0',entries=[])
 retained=obj('photara.package.retained-file-charge-leaf',count='0',charged_high_water='0',entries=[])
 conversion=None;sourceobs=[];retainednodes=[];retainedentries=[];snapshot={}
 if converted:
  conversion=obj('photara.package.conversion-source',conversion_id=uid(7001),snapshot_directory=['conversion-sources',uid(7001),'package'],source_bootstrap_sha256=bootstrap,source_commit_id=head0['commit_id'],source_format_version=manifest['format_version'],source_head_sha256=sha(source['HEAD.json']),files=[dict(components=p.split('/'),byte_length=str(len(data)),sha256=sha(data)) for p,data in sorted(source.items())])
  for i,(p,data) in enumerate(sorted(source.items())):
   physical=dict(device='7',inode=str(5000+i));charge=str(rounded(len(data)))
   observation=obj('photara.storage.local-observation',observation_id=uid(8000+i),profile=PROFILE,incarnation=INC,subject=dict(kind='retained-file',conversion_id=conversion['conversion_id'],namespace=conversion['snapshot_directory'],path=p.split('/')),physical=physical,measured_extent=str(len(data)),charged_high_water=charge);sourceobs.append(observation)
   retainedentries.append(dict(key=dict(conversion_id=conversion['conversion_id'],path=p.split('/')),conversion_source=ref(conversion),source_file=dict(byte_length=str(len(data)),sha256=sha(data)),registered_charge=charge,observation=ref(observation)))
   snapshot[p]=dict(hex=data.hex(),regular=True,witness=physical)
  retainedref,retainednodes=tree(b,'photara.package.retained-file-charge',retainedentries,lambda e:e['key'],order=lambda x:(x['conversion_id'],x['path']),subtotal=lambda e:e['registered_charge'])
  retained=b.values[key(retainedref)]
 observations=[]
 for n in [1,2,3]:
  extent=len(raw) if n==3 else EXTENT
  observations.append(obj('photara.storage.local-observation',observation_id=uid(3000+n),profile=PROFILE,incarnation=INC,subject=dict(kind='whole-blob' if n==3 else 'pack',allocation_id=g.aid(n),arena='metadata' if n==2 else 'data'),physical=g.witness(n),measured_extent=str(extent),charged_high_water=str(rounded(extent))))
 charge=obj('photara.storage.sealed-charge',allocation_id=g.aid(3),arena='data',domain_incarnation=INC,measured_extent=str(len(raw)),charged_high_water=str(rounded(len(raw))),observation=ref(observations[2]))
 obsroot,obsnodes=tree(b,'photara.storage.local-observation',[dict(observation_id=o['observation_id'],observation=ref(o)) for o in [observations[2]]+sourceobs],lambda e:e['observation_id'])
 charges,chargenodes=tree(b,'photara.storage.charge',[dict(allocation_id=g.aid(3),charge=ref(charge),charged_high_water=charge['charged_high_water'])],lambda e:e['allocation_id'],subtotal=lambda e:e['charged_high_water'])
 global_records=idnodes+ordnodes+[index,state,pins,evidence,holds,tickets,retained,observations[2],charge]+obsnodes+chargenodes+list(legacy.values())+sourceobs+retainednodes+([conversion] if converted else [])
 global_refs=[ref(v) for v in global_records]+[blob]
 global_refs={key(r):r for r in global_refs}
 baseinv,globalnodes=tree(b,'photara.package.inventory',list(global_refs.values()),order=key)
 # Inventory implementation pages are authenticated and owned outside global member enumeration.
 records={key(ref(v)):v for v in global_records+semantic_nodes+globalnodes}
 for v in records.values():b.add(v)
 claims=[]
 for n in [1,2,3]:
  sealed=n==3
  claims.append(obj('photara.storage.allocation-claim',allocation_id=g.aid(n),arena='metadata' if n==2 else 'data',layout='whole-blob' if sealed else 'framed-json',owned_extent=str(len(raw) if sealed else EXTENT),authenticated_prefix=dict(byte_length=str(len(raw) if sealed else 0),sha256=blob['sha256'] if sealed else sha(b'')),sealed=sealed,sealed_charge=ref(charge) if sealed else None))
 owner,ownnodes=tree(b,'photara.storage.ownership',[dict(allocation_id=v['allocation_id'],claim=ref(v)) for v in claims],lambda e:e['allocation_id'])
 records.update({key(ref(v)):v for v in claims+ownnodes})
 def tag(v):return 2 if v['schema']['id'].startswith('photara.storage.') or v['schema']['id'].startswith('photara.package.retained-file-charge') else 1
 loc=b.pack(1,'data',[(v,tag(v)) for _,v in sorted(records.items())])
 entries=[dict(object=ref(v),membership='ownership' if tag(v)==2 else 'semantic',physical=loc[k]) for k,v in sorted(records.items())]
 entries.append(dict(object=blob,membership='blob',physical=dict(kind='whole-blob',allocation_id=g.aid(3),byte_length=blob['byte_length'],sha256=blob['sha256'])))
 entries.sort(key=lambda e:key(e['object']))
 children=[]
 for i in range(0,len(entries),64):
  part=entries[i:i+64];node=obj('photara.storage.locator-leaf',count=str(len(part)),entries=part);physical=b.pack(2,'metadata',[(node,3)])[key(ref(node))]
  children.append(dict(first=part[0]['object'],last=part[-1]['object'],count=str(len(part)),child=physical))
 if len(children)==1:locator=children[0]['child']
 else:
  node=obj('photara.storage.locator-branch',count=str(len(entries)),children=children);locator=b.pack(2,'metadata',[(node,3)])[key(ref(node))]
 placement=obj('photara.storage.root-placement-leaf',count='1',entries=[dict(root=ref(state),root_id=state['root_id'],locator=locator,ownership=owner)])
 rootplacement=b.pack(2,'metadata',[(placement,3)])[key(ref(placement))]
 b.pad(1);b.pad(2)
 total=2*EXTENT+rounded(len(raw))+STANDING+DIRECTORY+(sum(int(e['registered_charge']) for e in retainedentries)+16384 if converted else 0)
 tips=[dict(allocation_id=g.aid(n),arena=observations[n-1]['subject']['arena'],extent=str(EXTENT),registered_charge=str(EXTENT),observation=observations[n-1]) for n in [1,2]]
 ledger=obj('photara.storage.ledger',profile=PROFILE,incarnation=INC,standing_control=str(STANDING),directory_allowance=str(DIRECTORY),retained_directory_allowance='16384' if converted else '0',tips=tips,sealed_charge_root=charges,observation_root=obsroot,retained_file_charge_root=ref(retained),conversion_source=ref(conversion) if converted else None,retirement_tickets=ref(tickets),total_charge=str(total))
 envelope=obj('photara.storage.accounting-envelope',ledger=ref(ledger),holds=ref(holds))
 controls=sorted([ref(ledger),ref(envelope)],key=key)
 overlay=obj('photara.package.inventory-overlay',base=baseinv,controls=controls,count=str(len(global_refs)+2))
 roots=obj('photara.package.root-set',version=2,library_id=L,bootstrap_sha256=bootstrap,kind='sealed',active=ref(state),recovery=ref(state),pinned_roots=ref(pins),operation_index=ref(index),conversion_source=ref(conversion) if converted else None,retention_evidence=ref(evidence),inventory=ref(overlay),placement=dict(generation='1',root_placements=rootplacement,accounting=ref(envelope)))
 capabilities=['photara.scalable-storage.v1','photara.sealed-roots.v1','photara.storage-accounting.v1','photara.whole-blob-storage.v1']
 commit=dict(schema=dict(id='photara.package.commit',version=1),project_id=P,commit_id=uid(6001),package_revision='1',bootstrap_sha256=bootstrap,parent=None,write_id=uid(6002),created_at='2026-09-27T00:00:00.000Z',minimum_reader=dict(major=1,minor=3),required_features=sorted(set(manifest['required_features']+capabilities)),authored=state['authored'],history=state['history'],inventory=semantic,root_set=roots,extensions={})
 head=dict(schema=dict(id='photara.package.head',version=1),project_id=P,commit_id=commit['commit_id'],commit_sha256=sha(enc(commit)))
 loose=[ledger,envelope,overlay];control_charge=sum(rounded(len(enc(v))) for v in loose+[manifest,head,commit]);assert control_charge<=STANDING
 allocations={k:dict(layout='framed-json',arena=f['arena'],hex=f['raw'].hex(),witness=f['witness']) for k,f in b.files.items()}
 allocations[g.aid(3)]=dict(layout='whole-blob',arena='data',hex=raw.hex(),witness=g.witness(3))
 registered={k:dict(layout=v['layout'],arena=v['arena'],extent=str(len(bytes.fromhex(v['hex']))),registered_charge=str(rounded(len(bytes.fromhex(v['hex'])))),witness=v['witness']) for k,v in allocations.items()}
 result=dict(status='unfrozen-preparatory-not-shared-head-verified',qualification=False,original_package_sha256=sha(original_bytes),bootstrap={k:enc(v).decode() for k,v in [('manifest',manifest),('head',head),('commit',commit)]},loose={ref(v)['sha256']:enc(v).decode() for v in loose},allocations=allocations,source_files={},source_witnesses={},journal=[],original_evidence=dict(profile=PROFILE,incarnation=INC,allocations=registered,standing_control=str(STANDING),directory_allowance=str(DIRECTORY),retained_directory_allowance='0'),expected=dict(total_charge=str(total),control_charge=str(control_charge),legacy_closure=original_refs,blob=blob,blob_allocation=g.aid(3),semantic_members=len(semantic_refs),global_members=len(global_refs)+2),records={ref(v)['sha256']:dict(canonical=enc(v).decode(),sha256=ref(v)['sha256'],byte_length=len(enc(v))) for v in list(b.values.values())+loose})
 if converted:
  result['snapshot_files']=snapshot
  result['original_evidence']['retained_directory_allowance']='16384'
  result['original_evidence']['conversion']=enc(conversion).decode()
  result['original_evidence']['retained_files']={p:dict(regular=True,extent=str(len(source[p])),registered_charge=str(rounded(len(source[p]))),sha256=sha(source[p]),witness=v['witness']) for p,v in snapshot.items()}
 return result

if __name__=='__main__':
 result=build('--converted' in sys.argv);(HERE/('converted.json' if '--converted' in sys.argv else 'linked.json')).write_text(json.dumps(result,sort_keys=True,indent=2)+'\n');print(result['status'],result['expected']['total_charge'])
