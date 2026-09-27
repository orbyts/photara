#!/usr/bin/env python3
"""Independent fixed canonical bytes for an UNFROZEN settled coordinate proposal."""
import copy
import hashlib
import importlib.util
import json
import struct
import sys
from pathlib import Path
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
P='10000000-0000-4000-8000-000000000001';L='10000000-0000-4000-8000-000000000002'
PROFILE='example.ps2.synthetic-local-profile-v1';INC='96000000-0000-4000-8000-000000000001'
EXTENT=262144;STANDING=131072;DIRECTORY=16384;RETAINED_DIRECTORY=16384
FEATURES=['photara.canonical-json.v1','photara.sealed-roots.v1','photara.scalable-storage.v1','photara.resource-backings.v1','photara.resource-state-trees.v1','photara.storage-accounting.v1']
def enc(v):return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()
def sha(b):return hashlib.sha256(b).hexdigest()
def ref(v):return dict(kind='json',sha256=sha(enc(v)),byte_length=str(len(enc(v))))
def key(r):return r['sha256'],int(r['byte_length'])
def obj(name,version=1,**kw):return dict(schema=dict(id=name,version=version),project_id=P,extensions={},**kw)
def uid(n):return f'96000000-0000-4000-8000-{n:012d}'
def aid(n):return uid(1000+n)
def rounded(n):return (n+4095)//4096*4096
def witness(n):return dict(device='7',inode=str(100+n))
def load(path):return json.loads(path.read_text())
class Builder:
 def __init__(self,mode):self.mode=mode;self.values={};self.files={};self.locations={}
 def add(self,v):self.values[key(ref(v))]=v;return ref(v)
 def pack(self,n,arena,items):
  f=self.files.setdefault(aid(n),dict(arena=arena,raw=bytearray(),witness=witness(n)));loc=self.locations.setdefault(aid(n),{})
  for v,tag in items:
   if self.mode=='wrong-resource-membership' and n==6 and v['schema']['id']=='photara.resource.backing':tag=2
   r=self.add(v)
   if key(r) in loc:continue
   body=enc(v);frame=b'PS2PKD01'+bytes([tag,0,0,0])+struct.pack('<I',len(body))+body
   loc[key(r)]=dict(allocation_id=aid(n),arena=arena,offset=str(len(f['raw'])),byte_length=str(len(frame)),record_sha256=sha(frame));f['raw'].extend(frame)
  return loc
 def pad(self,n):
  f=self.files[aid(n)]['raw'];gap=EXTENT-len(f)-16;assert gap>=0
  f.extend(b'PS2PKD01'+bytes(4)+struct.pack('<I',gap)+bytes(gap))
 def tree(self,name,entries,ek=lambda e:e,size=4,extra=None):
  entries=sorted(entries,key=lambda e:key(ek(e)) if isinstance(ek(e),dict) and ek(e).get('kind')=='json' else ek(e));nodes=[];level=[]
  def add(v):nodes.append(v);return self.add(v)
  for i in range(0,max(1,len(entries)),size):
   part=entries[i:i+size];fields=dict(count=str(len(part)),entries=part)
   if extra:fields.update(extra(part,False))
   r=add(obj(name+'-leaf',**fields));summary=dict(first=ek(part[0]) if part else None,last=ek(part[-1]) if part else None,count=str(len(part)),child=r)
   if name=='photara.package.operation-ordinal':summary.update(first=str(summary['first']),last=str(summary['last']))
   if extra:summary.update(extra(part,False))
   level.append(summary)
  while len(level)>1:
   out=[]
   for i in range(0,len(level),4):
    part=level[i:i+4]
    if len(part)==1:out.extend(part);continue
    fields=dict(count=str(sum(int(c['count']) for c in part)),children=part)
    if extra:fields.update(extra(part,True))
    r=add(obj(name+'-branch',**fields));summary=dict(first=part[0]['first'],last=part[-1]['last'],count=fields['count'],child=r)
    if extra:summary.update(extra(part,True))
    out.append(summary)
   level=out
  return level[0]['child'],nodes

def build(mode="valid", resource_builder=None, evidence_builder=None, extra_features=()):
 b=Builder(mode);ops=load(HERE.parent/'operations/linked-operations.json');old=load(HERE.parent/'resource-conversion/linked.json')['scenarios']['valid']
 spec=importlib.util.spec_from_file_location('factored',HERE.parent/'resource-factored/generate.py');mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod);resources=mod.build(8)
 pool={key(ref(v['input'])):v['input'] for v in ops['records'].values()};pool.update({key(ref(v['input'])):v['input'] for v in resources['records'].values()})
 def authored(r,out):
  k=key(r)
  if k in out:return
  v=pool[k];out[k]=v;s=v['schema']['id'];edges=[]
  if s=='photara.project.authored':edges=[v[x] for x in ['party_assignments','location_assignments','asset_ledger','resource_ledger','context']]+[g['document'] for g in v['graphs']]
  elif s=='photara.project.saved-graph':edges=[v['context']]+[x['manifest'] for x in v['required_packages']+v['node_contracts']]
  elif s=='photara.context.authored':edges=[x['context'] for x in v['node_contexts']]
  for e in edges:authored(e,out)
 def rclosure(r,out):
  k=key(r)
  if k in out:return
  v=pool[k];out[k]=v;s=v['schema']['id'];edges=[]
  if s=='photara.resource.state':edges=[v[x] for x in ['identities','working_bindings','versions','backings','requirements','retention_sources']]
  elif s=='photara.resource.selection-leaf':edges=[e['record'] for e in v['entries']]
  elif s=='photara.resource.selection-branch':edges=[e['child'] for e in v['children']]
  elif s=='photara.resource.retention-source':edges=[v['requirements']]
  elif s=='photara.resource.captured-version':edges=[v['capture_evidence']]
  elif s=='photara.resource.backing':edges=[v['publication_evidence']]
  for e in edges:rclosure(e,out)
 def remap_resource(root_id,slot):
  original=resources['roots']['a'];allr={};rclosure(original['resource_state'],allr)
  rs=copy.deepcopy(pool[key(original['resource_state'])]);source=pool[key(rs['retention_sources'])];assert source['schema']['id'].endswith('-leaf')
  source=copy.deepcopy(source)
  for j,e in enumerate(source['entries']):
   association=copy.deepcopy(pool[key(e['record'])]);association['association_id']=uid(2000+(1 if mode=='conflicting-association-id' and slot==3 else slot)*10+j);association['origin']['source_id']=uid(101) if mode=='invented-origin' and slot==3 else root_id
   rr=b.add(association);pool[key(rr)]=association;e['id']=association['association_id'];e['record']=rr
  sr=b.add(source);pool[key(sr)]=source;rs['retention_sources']=sr;rr=b.add(rs);pool[key(rr)]=rs
  out={};rclosure(rr,out)
  if resource_builder is not None:return resource_builder(root_id,slot,rs,out,b)
  return rr,out
 receipt1=ops['records']['receipt-1']['input'];b.pack(1,'data',[(receipt1,1)])
 observations=[];sealed=[]
 for n in range(1,9):
  extent=len(b.files[aid(1)]['raw']) if n==1 else EXTENT
  ob=obj('photara.storage.local-observation',observation_id=uid(3000+n),profile=PROFILE,incarnation=INC,subject=dict(kind='pack',allocation_id=aid(n),arena='metadata' if n in [3,5,7,8] else 'data'),physical=witness(n),measured_extent=str(extent),charged_high_water=str(rounded(extent)));observations.append(ob);b.add(ob)
  if n==1:sealed.append(obj('photara.storage.sealed-charge',allocation_id=aid(1),arena='data',domain_incarnation=INC,measured_extent=str(extent),charged_high_water=str(rounded(extent)),observation=ref(ob)))
 conversion=copy.deepcopy(old['records'][old['conversion']['sha256']]['input']);
 if mode=='changed-original-conversion':conversion['source_commit_id']=uid(9999)
 source={p:bytes.fromhex(h) for p,h in old['source_files'].items()};source_witnesses={};retained=[]
 for i,(path,raw) in enumerate(sorted(source.items())):
  ob=obj('photara.storage.local-observation',observation_id=uid(4000+i),profile=PROFILE,incarnation=INC,subject=dict(kind='retained-file',conversion_id=conversion['conversion_id'],namespace=conversion['snapshot_directory'],path=path.split('/')),physical=dict(device='7',inode=str(500+i)),measured_extent=str(len(raw)),charged_high_water=str(rounded(len(raw))));observations.append(ob);source_witnesses[path]=ob['physical']
  retained.append(dict(key=dict(conversion_id=conversion['conversion_id'],path=path.split('/')),conversion_source=ref(conversion),source_file=dict(byte_length=str(len(raw)),sha256=sha(raw)),registered_charge=str(rounded(len(raw))),observation=ref(ob)))
 obsroot,obsnodes=b.tree('photara.storage.local-observation',[dict(observation_id=v['observation_id'],observation=ref(v)) for v in observations[:1]+observations[8:]],lambda e:e['observation_id'],size=3)
 charge_root,charge_nodes=b.tree('photara.storage.charge',[dict(allocation_id=v['allocation_id'],charge=ref(v),charged_high_water=v['charged_high_water']) for v in sealed],lambda e:e['allocation_id'],extra=lambda p,br:dict(charged_high_water=str(sum(int(x['charged_high_water']) for x in p))))
 def retained_tree(entries):
  nodes=[];levels=[]
  for i in range(0,len(entries),3):
   p=entries[i:i+3];v=obj('photara.package.retained-file-charge-leaf',count=str(len(p)),charged_high_water=str(sum(int(e['registered_charge']) for e in p)),entries=p);nodes.append(v);levels.append(dict(first=p[0]['key'],last=p[-1]['key'],count=v['count'],charged_high_water=v['charged_high_water'],child=ref(v)))
  while len(levels)>1:
   out=[]
   for i in range(0,len(levels),4):
    p=levels[i:i+4]
    if len(p)==1:out+=p;continue
    v=obj('photara.package.retained-file-charge-branch',count=str(sum(int(c['count']) for c in p)),charged_high_water=str(sum(int(c['charged_high_water']) for c in p)),children=p);nodes.append(v);out.append(dict(first=p[0]['first'],last=p[-1]['last'],count=v['count'],charged_high_water=v['charged_high_water'],child=ref(v)))
   levels=out
  return levels[0]['child'],nodes
 retained_root,retained_nodes=retained_tree(retained)
 holds=obj('photara.storage.hold-leaf',count='0',reserved='0',consumed='0',remaining='0',entries=[]);tickets=obj('photara.storage.retirement-ticket-leaf',count='0',charged_high_water='0',entries=[])
 evidence=obj('photara.resource.retention-evidence-leaf',count='0',entries=[])
 states={};role_records={};semantic={};invnodes={}
 for role,n,count,slot in [('active',2,3,1),('recovery',4,2,2),('retained',6,1,3)]:
  base={};auth=ops['records']['authored-2' if count==3 else 'authored-1']['input'];history=copy.deepcopy(ops['records']['history']['input']);
  if mode=='unknown-legacy-history-version':history['schema']['version']=99;pool[key(ref(history))]=history
  if mode=='wrong-history-edge':history=auth
  authored(ref(auth),base);authored(ref(history),base)
  receipts=[ops['records'][f'receipt-{i}']['input'] for i in range(1,count+1)]
  entries=[dict(operation_id=r['operation_id'],request_sha256=r['request_sha256'],acceptance_ordinal=r['acceptance_ordinal'],receipt=ref(r)) for r in receipts]
  idroot,idnodes=b.tree('photara.package.operation-id',entries,lambda e:e['operation_id'],size=1);ordroot,ordnodes=b.tree('photara.package.operation-ordinal',entries,lambda e:int(e['acceptance_ordinal']),size=1)
  accepted=ops['records'][f'operation-root-{count}']['input']['accepted'] if count>1 else dict(through_ordinal='1',prefix_sha256=ops['expected_accepted_prefixes'][0])
  if count==1:
   prefix=sha(enc(dict(domain='photara.package.accepted-prefix.v1',project_id=P,library_id=L,bootstrap_sha256=ops['records']['manifest']['sha256'],through_ordinal='0')))
   r=receipts[0];accepted['prefix_sha256']=sha(enc(dict(domain='photara.package.accepted-prefix-link.v1',previous_sha256=prefix,acceptance_ordinal=r['acceptance_ordinal'],operation_id=r['operation_id'],request_sha256=r['request_sha256'],receipt_sha256=ref(r)['sha256'])))
  index=obj('photara.package.operation-index',version=2,library_id=L,bootstrap_sha256=ops['records']['manifest']['sha256'],accepted=accepted,by_id=idroot,by_ordinal=ordroot)
  for v in receipts+idnodes+ordnodes+[index]:base[key(ref(v))]=v
  rid=uid(101 if mode=='duplicate-root-id' and role=='retained' else 100+slot);rs,rvalues=remap_resource(rid,slot);base.update(rvalues)
  inv,ins=b.tree('photara.package.inventory',[ref(base[k]) for k in sorted(base)]);invnodes[role]=ins
  frames=[ops['records'][f'journal-frame-{i}']['input'] for i in range(1,count+1)];prefix=sha(enc(dict(domain='photara.package.journal-prefix.v1',journal_id=frames[0]['journal_id'],through_sequence='0')))
  for f in frames:prefix=sha(enc(dict(domain='photara.package.journal-prefix-link.v1',previous_sha256=prefix,frame_sha256=sha(enc(f)))))
  j=dict(journal_id=frames[0]['journal_id'],through_sequence=frames[-1]['sequence'],prefix_sha256=prefix,resulting_authored_revision=auth['authored_revision'],resulting_authored_sha256=ref(auth)['sha256'])
  state=obj('photara.package.state-root',version=2,library_id=L,bootstrap_sha256=ops['records']['manifest']['sha256'],root_id=rid,authored_revision=auth['authored_revision'],authored=ref(auth),history=ref(history),resource_state=rs,operation_index=ref(index),accepted=accepted,journal_inclusion=j,predecessor=None,inventory=inv)
  states[role]=ref(state);semantic[role]=base;role_records[role]={**base,**{key(ref(v)):v for v in ins+[state]}}
 pins=obj('photara.package.retained-root-leaf',count='1',entries=[dict(pin_id=uid(5001),reason='explicit-history',root=states['retained'])])
 tips=[dict(allocation_id=aid(n),arena=observations[n-1]['subject']['arena'],extent=str(EXTENT),registered_charge=str(EXTENT),observation=observations[n-1]) for n in range(2,9)]
 total=STANDING+DIRECTORY+RETAINED_DIRECTORY+7*EXTENT+sum(int(x['charged_high_water']) for x in sealed)+sum(int(x['registered_charge']) for x in retained)
 ledger=obj('photara.storage.ledger',profile=PROFILE,incarnation=INC,standing_control=str(STANDING),directory_allowance=str(DIRECTORY),retained_directory_allowance=str(RETAINED_DIRECTORY),tips=tips,sealed_charge_root=charge_root,observation_root=obsroot,retained_file_charge_root=retained_root,conversion_source=ref(conversion),retirement_tickets=ref(tickets),total_charge=str(total))
 envelope=obj('photara.storage.accounting-envelope',ledger=ref(ledger),holds=ref(holds))
 alternate=copy.deepcopy(sealed[0]);alternate['extensions']={'example.changed':True}
 extra_records=[];selected_recovery=states['recovery']
 if evidence_builder is not None:
  addition=evidence_builder(states,role_records,semantic,pins,b)
  evidence=addition['evidence'];extra_records=addition['records'];pins=addition.get('pins',pins);selected_recovery=addition.get('recovery',selected_recovery)
 globals_=observations[:1]+observations[8:]+sealed+([alternate] if mode=='alternate-sealed-claim' else [])+obsnodes+charge_nodes+retained_nodes+[holds,tickets,evidence,pins,conversion]+extra_records
 global_map={key(ref(v)):v for v in globals_}
 union=dict(global_map)
 for role,records in role_records.items():union.update({k:v for k,v in records.items() if not v['schema']['id'].startswith('photara.package.inventory-')})
 if mode=='missing-global-member':union.pop(key(states['retained']))
 baseinv,globalnodes=b.tree('photara.package.inventory',[ref(union[k]) for k in sorted(union)])
 placements=[]
 for role,n in [('active',2),('recovery',4),('retained',6)]:
  records=role_records[role];records.update(global_map);records.update({key(ref(v)):v for v in globalnodes})
  b.pack(n,'data',[(v,1 if not (v['schema']['id'].startswith('photara.storage.') or v['schema']['id'].startswith('photara.package.retained-file-charge')) else 2) for k,v in sorted(records.items()) if k!=key(ref(receipt1))])
  claims=[]
  for unit in ([1,n,n+1] if mode=='missing-shared-ownership' and role=='retained' else [1,n,n+1,8]):
   sealedunit=unit==1;raw=bytes(b.files[aid(1)]['raw']) if sealedunit else b''
   claims.append(obj('photara.storage.allocation-claim',allocation_id=aid(unit),arena='metadata' if unit in [n+1,8] else 'data',layout='framed-json',owned_extent=str(len(raw) if sealedunit else EXTENT),authenticated_prefix=dict(byte_length=str(len(raw)),sha256=sha(raw)),sealed=sealedunit,sealed_charge=ref(sealed[0]) if sealedunit else None))
  if mode=='alternate-sealed-claim':claims[0]['sealed_charge']=ref(alternate)
  if mode=='sealed-as-tip':
   claims[0]['sealed']=False;claims[0]['sealed_charge']=None;claims[0]['authenticated_prefix']=dict(byte_length='0',sha256=sha(b''))
  owner,ownnodes=b.tree('photara.storage.ownership',[dict(allocation_id=v['allocation_id'],claim=ref(v)) for v in claims],lambda e:e['allocation_id'],size=2)
  b.pack(n,'data',[(v,2) for v in claims+ownnodes]);records.update({key(ref(v)):v for v in claims+ownnodes})
  locs={**b.locations[aid(1)],**b.locations[aid(n)]};entries=[]
  for k,v in sorted(records.items()):
   if mode=='missing-resource-locator' and role=='retained' and v['schema']['id']=='photara.resource.retention-requirement':continue
   tag=2 if v['schema']['id'].startswith('photara.storage.') or v['schema']['id'].startswith('photara.package.retained-file-charge') else 1
   if mode=='wrong-resource-membership' and n==6 and v['schema']['id']=='photara.resource.backing':tag=2
   entries.append(dict(object=ref(v),membership='ownership' if tag==2 else 'semantic',physical=locs[k]))
  leaves=[]
  for i in range(0,len(entries),64):
   p=entries[i:i+64];v=obj('photara.storage.locator-leaf',count=str(len(p)),entries=p);physical=b.pack(n+1,'metadata',[(v,3)])[key(ref(v))];leaves.append(dict(first=p[0]['object'],last=p[-1]['object'],count=str(len(p)),child=physical))
  locator=obj('photara.storage.locator-branch',count=str(len(entries)),children=leaves);lp=b.pack(n+1,'metadata',[(locator,3)])[key(ref(locator))]
  placements.append(dict(root=states[role],root_id=b.values[key(states[role])]['root_id'],locator=lp,ownership=owner))
  b.pad(n);b.pad(n+1)
 if mode=='borrowed-pin-placement':placements[2]['locator']=placements[0]['locator'];placements[2]['ownership']=placements[0]['ownership']
 placements.sort(key=lambda x:key(x['root']));children=[]
 for e in placements:
  v=obj('photara.storage.root-placement-leaf',count='1',entries=[e]);p=b.pack(8,'metadata',[(v,3)])[key(ref(v))];children.append(dict(first=e['root'],last=e['root'],count='1',child=p))
 rootplacement=obj('photara.storage.root-placement-branch',count=str(len(placements)),children=children);pr=b.pack(8,'metadata',[(rootplacement,3)])[key(ref(rootplacement))];b.pad(8)
 if mode=='undercharge':ledger['total_charge']=str(total-4096);envelope['ledger']=ref(ledger)
 controls=sorted([ref(ledger),ref(envelope)],key=key);overlay=obj('photara.package.inventory-overlay',base=baseinv,controls=controls,count=str(len(union)+len(controls)))
 roots=obj('photara.package.root-set',version=2,library_id=L,bootstrap_sha256=ops['records']['manifest']['sha256'],kind='sealed',active=states['active'],recovery=selected_recovery,pinned_roots=ref(pins),operation_index=b.values[key(states['active'])]['operation_index'],conversion_source=ref(conversion),retention_evidence=ref(evidence),inventory=ref(overlay),placement=dict(generation='1',root_placements=pr,accounting=ref(envelope)))
 active=b.values[key(states['active'])];manifest_features=ops['records']['manifest']['input']['required_features'];commit=dict(schema=dict(id='photara.package.commit',version=1),project_id=P,commit_id=uid(6001),package_revision='1',bootstrap_sha256=ops['records']['manifest']['sha256'],parent=None,write_id=uid(6002),created_at='2026-09-27T00:00:00.000Z',minimum_reader=dict(major=1,minor=3),required_features=sorted(set(FEATURES+manifest_features+list(extra_features))),authored=active['authored'],history=active['history'],inventory=active['inventory'],root_set=roots,extensions={})
 if mode=='missing-capability':commit['required_features'].remove('photara.scalable-storage.v1')
 head=dict(schema=dict(id='photara.package.head',version=1),project_id=P,commit_id=commit['commit_id'],commit_sha256=sha(enc(commit)))
 loose=[ledger,envelope,overlay];manifest=ops['records']['manifest']['input'];controlcharge=sum(rounded(len(enc(v))) for v in loose+[manifest,head,commit]);assert controlcharge<=STANDING
 return dict(status='unfrozen-proposed-settled-integrated',qualification=False,bootstrap={k:enc(v).decode() for k,v in [('manifest',manifest),('head',head),('commit',commit)]},loose={ref(v)['sha256']:enc(v).decode() for v in loose},allocations={k:dict(arena=v['arena'],hex=v['raw'].hex(),witness=v['witness']) for k,v in b.files.items()},source_files={p:v.hex() for p,v in source.items()},source_witnesses=source_witnesses,journal=[ops['records'][f'journal-frame-{i}']['canonical'] for i in [1,2,3]],fixture_profile=resources['fixture_profile'],expected=dict(total_charge=str(total),control_charge=str(controlcharge),semantic_counts={r:len(v) for r,v in semantic.items()},global_members=len(union)+2,roles=states),records={k[0]:dict(canonical=enc(v).decode(),sha256=k[0],byte_length=k[1]) for k,v in b.values.items()})
def delta(base,changed):
 out={k:changed[k] for k in ['bootstrap','loose','source_files','source_witnesses']};out['allocations']={}
 for allocation,v in changed['allocations'].items():
  before=bytes.fromhex(base['allocations'][allocation]['hex']);after=bytes.fromhex(v['hex']);assert len(before)==len(after)
  spans=[];start=None;last=None
  for i,(x,y) in enumerate(zip(before,after)):
   if x!=y:
    if start is None:start=i
    elif i-last>32:spans.append(dict(offset=start,hex=after[start:last+1].hex()));start=i
    last=i
  if start is not None:spans.append(dict(offset=start,hex=after[start:last+1].hex()))
  if spans:out['allocations'][allocation]=dict(patches=spans)
 return out
if __name__=='__main__':
 result=build();(HERE/'linked.json').write_text(json.dumps(result,sort_keys=True,indent=2)+'\n');print(result['expected'])
 negatives={mode:delta(result,build(mode)) for mode in ['missing-global-member','missing-shared-ownership','borrowed-pin-placement','undercharge','missing-capability','invented-origin','missing-resource-locator','changed-original-conversion','duplicate-root-id','sealed-as-tip','unknown-legacy-history-version','alternate-sealed-claim','conflicting-association-id','wrong-resource-membership','wrong-history-edge']}
 (HERE/'negatives.json').write_text(json.dumps(negatives,sort_keys=True,indent=2)+'\n')
