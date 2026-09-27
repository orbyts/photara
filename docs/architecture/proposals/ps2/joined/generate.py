#!/usr/bin/env python3
"""Unfrozen complete selected snapshot; pure independent byte construction.
Builder deliberately separates logical roots, exact packed placement and control
selection so a later phase route can reuse the same resulting schema.
"""
import hashlib
import json
import struct
from pathlib import Path
HERE=Path(__file__).resolve().parent
PROJECT='10000000-0000-4000-8000-000000000001'
LIBRARY='10000000-0000-4000-8000-000000000002'
PROFILE='example.ps2.synthetic-local-profile-v1'
INCARNATION='71000000-0000-4000-8000-000000000001'
EXTENT=131072
STANDING=131072
DIRECTORY=16384

def canonical(v):return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()
def sha(b):return hashlib.sha256(b).hexdigest()
def ref(v):
 b=canonical(v);return dict(kind='json',sha256=sha(b),byte_length=str(len(b)))
def key(r):return r['sha256'],int(r['byte_length'])
def obj(schema,**fields):return dict(schema=dict(id='example.ps2.'+schema,version=1),project_id=PROJECT,extensions={},**fields)
def rounded(n):return (n+4095)//4096*4096
def aid(n):return f'80000000-0000-4000-8000-{n:012d}'
def witness(n):return dict(profile=PROFILE,incarnation=INCARNATION,allocation_id=aid(n),device='7',inode=str(100+n))

class Builder:
 def __init__(self,mode):
  self.mode=mode
  self.values={};self.files={};self.locations={};self.names={}
 def tag(self,v,tag):
  if (self.mode=='wrong-resource-membership' and v['schema']['id']=='photara.resource.backing') or (self.mode=='wrong-conversion-membership' and v['schema']['id']=='photara.package.conversion-source'):return 2
  return tag
 def remember(self,v):
  r=ref(v);self.values[key(r)]=v;return r
 def pack(self,n,arena,items):
  allocation=aid(n);raw=self.files.setdefault(allocation,dict(arena=arena,raw=bytearray(),witness=witness(n)))['raw']
  loc=self.locations.setdefault(allocation,{})
  for v,tag in items:
   tag=self.tag(v,tag)
   r=self.remember(v)
   if key(r) in loc:continue
   body=canonical(v);frame=b'PS2PKD01'+bytes([tag,0,0,0])+struct.pack('<I',len(body))+body
   loc[key(r)]=dict(allocation_id=allocation,arena=arena,offset=str(len(raw)),byte_length=str(len(frame)),record_sha256=sha(frame))
   raw.extend(frame)
  return loc
 def pad(self,n):
  raw=self.files[aid(n)]['raw'];gap=EXTENT-len(raw)-16;assert gap>0,'finite chosen corridor exhausted'
  raw.extend(b'PS2PKD01'+bytes(4)+struct.pack('<I',gap)+bytes(gap))
 def tree(self,kind,entries,size=3):
  created=[];charge=kind=='owned-charge'
  def ek(e):return e['allocation_id'] if charge else e
  def add(v):created.append(v);return self.remember(v)
  level=[]
  for start in range(0,len(entries),size):
   part=entries[start:start+size];fields=dict(count=str(len(part)),entries=part)
   if charge:fields['charged_high_water']=str(sum(int(e['charged_high_water']) for e in part))
   r=add(obj(kind+'-leaf',**fields));summary=dict(first=ek(part[0]),last=ek(part[-1]),count=str(len(part)),child=r)
   if charge:summary['charged_high_water']=fields['charged_high_water']
   level.append(summary)
  while len(level)>1:
   next_level=[]
   for start in range(0,len(level),2):
    part=level[start:start+2]
    if len(part)==1:next_level.extend(part);continue
    fields=dict(count=str(sum(int(c['count']) for c in part)),children=part)
    if charge:fields['charged_high_water']=str(sum(int(c['charged_high_water']) for c in part))
    r=add(obj(kind+'-branch',**fields));summary=dict(first=part[0]['first'],last=part[-1]['last'],count=fields['count'],child=r)
    if charge:summary['charged_high_water']=fields['charged_high_water']
    next_level.append(summary)
   level=next_level
  return level[0]['child'],created

def resources(pool,r):
 found={}
 def visit(r):
  k=key(r)
  if k in found:return
  v=pool[k];found[k]=v;s=v['schema']['id']
  if s=='photara.resource.state':
   for field,target in [('identities','identity'),('working_bindings','binding'),('versions','version'),('backings','backing'),('obligations','obligation')]:
    for e in v[field]:visit(e[target])
  elif s=='photara.resource.captured-version':visit(v['capture_evidence'])
  elif s=='photara.resource.backing':visit(v['publication_evidence'])
  elif s not in ['photara.resource.identity','photara.resource.working-binding','photara.resource.retention-obligation','photara.resource.publication-evidence']:raise ValueError(s)
 visit(r);return found

def build(mode="valid"):
 opspath=HERE.parent/'operations/linked-operations.json';branchpath=HERE.parent/'branches/linked-branches.json';resourcepath=HERE.parent/'resource-conversion/linked.json'
 ops=json.loads(opspath.read_text());branch=json.loads(branchpath.read_text());resource=json.loads(resourcepath.read_text())['scenarios']['valid']
 b=Builder(mode);pool={key(ref(r['input'])):r['input'] for r in branch['records'].values()};rp={key(ref(r['input'])):r['input'] for r in resource['records'].values()}
 if mode=='alternate-source-bootstrap':
  files={p:bytes.fromhex(h) for p,h in resource['source_files'].items()}
  manifest=json.loads(files['manifest.json']);manifest['created_at']='2026-09-13T00:00:00.000Z';files['manifest.json']=canonical(manifest)
  head=json.loads(files['HEAD.json']);commit_path='commits/'+head['commit_id']+'.json'
  source_commit=json.loads(files[commit_path]);source_commit['bootstrap_sha256']=sha(files['manifest.json']);files[commit_path]=canonical(source_commit)
  head['commit_sha256']=sha(files[commit_path]);files['HEAD.json']=canonical(head)
  descriptor=json.loads(json.dumps(rp[key(resource['conversion'])]));descriptor['source_bootstrap_sha256']=sha(files['manifest.json']);descriptor['source_head_sha256']=sha(files['HEAD.json'])
  for row in descriptor['files']:
   data=files['/'.join(row['components'])];row['byte_length']=str(len(data));row['sha256']=sha(data)
  resource['source_files']={p:data.hex() for p,data in files.items()};resource['conversion']=ref(descriptor);rp[key(ref(descriptor))]=descriptor
 role_base={};resource_roots={}
 for role in ['active','recovery']:
  projection=rp[key(resource['roots'][role])];resource_roots[role]=projection
  base={key(r):pool[key(r)] for r in branch['expected_semantic'][role]};base.update(resources(rp,projection['resource_state']));role_base[role]=base
 common=sorted(set(role_base['active'])&set(role_base['recovery']))
 for n in [1,2,3]:b.pack(n,'data',[(role_base['active'][k],1) for k in common[n-1::3]])
 charges=[]
 for n in [1,2,3]:
  raw=b.files[aid(n)]['raw'];v=obj('sealed-charge',allocation_id=aid(n),arena='data',domain_incarnation=INCARNATION,measured_extent=str(len(raw)),charged_high_water=str(rounded(len(raw))),observation=dict(kind='synthetic-vector',observation_id=f'81000000-0000-4000-8000-{n:012d}'))
  charges.append(v)
 charge_root,charge_nodes=b.tree('owned-charge',[dict(allocation_id=v['allocation_id'],charge=b.remember(v),charged_high_water=v['charged_high_water']) for v in charges],1)
 conversion=rp[key(resource['conversion'])];b.remember(conversion)
 source_files={p:bytes.fromhex(h) for p,h in resource['source_files'].items()}
 source_witnesses={p:dict(profile=PROFILE,incarnation=INCARNATION,namespace=conversion['snapshot_directory'],path=p.split('/'),device='7',inode=str(500+i)) for i,p in enumerate(sorted(source_files))}
 conversion_rows=[dict(path=p,logical_bytes=str(len(raw)),registered_charge=str(rounded(len(raw))),witness=source_witnesses[p]) for p,raw in sorted(source_files.items())]
 retained=dict(source=ref(conversion),files=conversion_rows,logical_bytes=str(sum(len(raw) for raw in source_files.values())),registered_charge=str(sum(int(v['registered_charge']) for v in conversion_rows)),directory_control=str(DIRECTORY))
 assert len(canonical(conversion_rows))<=DIRECTORY
 tips=[dict(witness=witness(n),arena='data' if n in [4,6] else 'metadata',extent=str(EXTENT),registered_charge=str(EXTENT)) for n in [4,5,6,7]]
 total=STANDING+4*EXTENT+sum(int(c['charged_high_water']) for c in charges)+int(retained['registered_charge'])+DIRECTORY
 accounting=obj('accounting-tree',profile=PROFILE,incarnation=INCARNATION,standing_control=str(STANDING),tips=tips,sealed_charge_tree=charge_root,sealed_observations=[dict(observation_id=v['observation']['observation_id'],witness=witness(n)) for n,v in enumerate(charges,1)],tickets=[],retained_conversion=retained,total_charge=str(total))
 accounting_ref=b.remember(accounting)
 states={};inventories={};role_records={}
 for role,n in [('active',4),('recovery',6)]:
  base=role_base[role];inv,invnodes=b.tree('inventory',[ref(base[k]) for k in sorted(base)])
  selected=branch['selector']['roles'][role];rr=resource_roots[role]
  state=obj('state-root',library_id=LIBRARY,bootstrap_sha256=ops['records']['manifest']['sha256'],root_id=rr['root_id'],authored_revision=selected['journal_inclusion']['resulting_authored_revision'],authored=selected['authored'],history=selected['history'],resource_state=rr['resource_state'],operation_index=selected['operation_index'],accepted=selected['accepted'],journal_inclusion=selected['journal_inclusion'],predecessor=None,inventory=inv)
  states[role]=b.remember(state);inventories[role]=invnodes
  semantic=[base[k] for k in sorted(set(base)-set(common))]+invnodes+[state]
  b.pack(n,'data',[(v,1) for v in semantic]);role_records[role]={key(ref(v)):(v,1) for v in list(base.values())+invnodes+[state]}
 # Global semantic inventory remains acyclic: excludes its own pages, ownership
 # implementation nodes and locator pages. It includes direct accounting and its
 # charge dependencies, conversion metadata, and both complete StateRoot closures.
 union={key(accounting_ref):accounting}
 for records in role_records.values():union.update({k:v for k,(v,_) in records.items()})
 union.update({key(ref(v)):v for v in charges+charge_nodes+[conversion]})
 union_root,union_nodes=b.tree('inventory',[ref(union[k]) for k in sorted(union)])
 placements={};owners={}
 for role,n in [('active',4),('recovery',6)]:
  globals=[(v,2) for v in charges+charge_nodes]+[(conversion,1)]+[(v,1) for v in union_nodes]
  b.pack(n,'data',globals)
  role_records[role].update({key(ref(v)):(v,t) for v,t in globals})
  claims=[]
  for allocated in [1,2,3,n,n+1]:
   sealed=allocated<=3;raw=b.files[aid(allocated)]['raw'] if sealed else b''
   claim=obj('ownership-claim',allocation_id=aid(allocated),arena='metadata' if allocated==n+1 else 'data',owned_extent=str(len(raw) if sealed else EXTENT),authenticated_prefix=dict(byte_length=str(len(raw)),sha256=sha(raw)),sealed=sealed,sealed_charge=ref(charges[allocated-1]) if sealed else None)
   claims.append(claim)
  loc=b.pack(n,'data',[(v,2) for v in claims])
  owner=obj('ownership-branch',count=str(len(claims)),children=[dict(allocation_id=v['allocation_id'],node=loc[key(ref(v))]) for v in claims])
  b.pack(n,'data',[(owner,2)]);owners[role]=loc[key(ref(owner))]
  role_records[role].update({key(ref(v)):(v,2) for v in claims+[owner]})
  locations={}
  for allocated in [1,2,3,n]:locations.update(b.locations[aid(allocated)])
  entries=[dict(object=ref(v),membership='semantic' if b.tag(v,tag)==1 else 'ownership',physical=locations[k]) for k,(v,tag) in sorted(role_records[role].items())]
  midpoint=len(entries)//2;children=[]
  for part in [entries[:midpoint],entries[midpoint:]]:
   leaf=obj('locator-leaf',count=str(len(part)),entries=part);p=b.pack(n+1,'metadata',[(leaf,3)])[key(ref(leaf))]
   children.append(dict(first=part[0]['object'],last=part[-1]['object'],count=str(len(part)),child=p))
  locator=obj('locator-branch',count=str(len(entries)),children=children)
  placements[role]=b.pack(n+1,'metadata',[(locator,3)])[key(ref(locator))]
  b.pad(n);b.pad(n+1)
 roots=obj('root-set',library_id=LIBRARY,bootstrap_sha256=ops['records']['manifest']['sha256'],kind='scalable-draft',active=states['active'],recovery=states['recovery'],pinned_roots=[],operation_index=branch['selector']['roles']['active']['operation_index'],conversion_source=ref(conversion),inventory=union_root,placement=dict(generation='1',active_locator=placements['active'],recovery_locator=placements['recovery'],active_ownership=owners['active'],recovery_ownership=owners['recovery'],accounting=accounting_ref))
 active=b.values[key(states['active'])]
 features=sorted(ops['records']['manifest']['input']['required_features']+['photara.sealed-roots.v1','photara.resource-backings.v1','example.ps2.scalable-storage.draft-v1','example.ps2.typed-branches.draft-v1','example.ps2.owned-accounting.draft-v1'])
 commit=dict(schema=dict(id='photara.package.commit',version=1),project_id=PROJECT,commit_id='82000000-0000-4000-8000-000000000001',package_revision='1',bootstrap_sha256=ops['records']['manifest']['sha256'],parent=None,write_id='83000000-0000-4000-8000-000000000001',created_at='2026-09-27T00:00:00.000Z',minimum_reader=dict(major=1,minor=3),required_features=features,authored=active['authored'],history=active['history'],inventory=active['inventory'],root_set=roots,extensions={})
 head=dict(schema=dict(id='photara.package.head',version=1),project_id=PROJECT,commit_id=commit['commit_id'],commit_sha256=sha(canonical(commit)))
 manifest=ops['records']['manifest']['input']
 control_bytes=sum(len(canonical(v)) for v in [manifest,head,commit,accounting]);assert control_bytes<=STANDING
 return dict(status='unfrozen-joined-settled-candidate',qualification=False,dependencies={name:sha(path.read_bytes()) for name,path in [('operations',opspath),('branches',branchpath),('resource_conversion',resourcepath)]},bootstrap={name:canonical(v).decode() for name,v in [('manifest',manifest),('head',head),('commit',commit)]},loose={accounting_ref['sha256']:canonical(accounting).decode()},allocations={k:dict(arena=v['arena'],hex=v['raw'].hex(),sha256=sha(v['raw']),witness=v['witness']) for k,v in b.files.items()},records={k[0]:dict(input=v,canonical=canonical(v).decode(),sha256=k[0],byte_length=k[1]) for k,v in b.values.items()},source_files={p:raw.hex() for p,raw in source_files.items()},source_witnesses=source_witnesses,expected=dict(total_charge=str(total),control_bytes=control_bytes,conversion_directory_bytes=len(canonical(conversion_rows)),semantic_counts={r:len(v) for r,v in role_base.items()},role_locator_counts={r:len(v) for r,v in role_records.items()}))
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
  if spans:out['allocations'][allocation]=dict(patches=spans,sha256=v['sha256'])
 return out
if __name__=='__main__':
 output=build();(HERE/'linked-joined.json').write_text(json.dumps(output,sort_keys=True,indent=2,ensure_ascii=False)+'\n');print(output['expected'])
 negatives={mode:delta(output,build(mode)) for mode in ['wrong-resource-membership','wrong-conversion-membership','alternate-source-bootstrap']}
 (HERE/'joined-negatives.json').write_text(json.dumps(negatives,sort_keys=True,indent=2,ensure_ascii=False)+'\n')
