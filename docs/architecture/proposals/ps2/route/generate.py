#!/usr/bin/env python3
"""Independent, unfrozen one-HEAD route construction. No production writer."""
import copy
import importlib.util
import json
import struct
import sys
sys.dont_write_bytecode=True
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('joined_builder',HERE.parent/'joined/generate.py')
j=importlib.util.module_from_spec(spec);spec.loader.exec_module(j)
canonical,sha,ref,key,obj,aid=j.canonical,j.sha,j.ref,j.key,j.obj,j.aid
STEP=131072
LIMIT=2621440
R=1048576
CLEANUP=65536

def frames(raw):
 p=0
 while p<len(raw):
  assert raw[p:p+8]==b'PS2PKD01'
  tag=raw[p+8];length=struct.unpack('<I',raw[p+12:p+16])[0];end=p+16+length
  assert end<=len(raw)
  yield p,end,tag,raw[p+16:end]
  p=end

def inflate(c):
 b=j.Builder('valid')
 for allocation,a in c['allocations'].items():
  raw=bytes.fromhex(a['hex']);b.files[allocation]=dict(arena=a['arena'],raw=bytearray(raw),witness=a['witness']);b.locations[allocation]={}
  for start,end,tag,body in frames(raw):
   if tag:
    v=json.loads(body);r=b.remember(v)
    b.locations[allocation][key(r)]=dict(allocation_id=allocation,arena=a['arena'],offset=str(start),byte_length=str(end-start),record_sha256=sha(raw[start:end]))
 for raw in c['loose'].values():b.remember(json.loads(raw))
 return b

def closure(pool,roots):
 out={}
 def walk(v):
  if isinstance(v,dict):
   if set(v)=={'kind','sha256','byte_length'}:
    k=key(v)
    if k not in out:out[k]=pool[k];walk(pool[k])
   else:
    for name,value in v.items():
     if name!='extensions':walk(value)
  elif isinstance(v,list):
   for x in v:walk(x)
 for r in roots:walk(r)
 return out

def core(c):
 root=json.loads(c['bootstrap']['commit'])['root_set'];v=json.loads(c['loose'][root['placement']['accounting']['sha256']])
 return json.loads(c['loose'][v['ledger']['sha256']]) if v['schema']['id']=='example.ps2.route-accounting' else v

def payload(old,operation,retire=False,mode="valid"):
 b=inflate(old);oldroot=json.loads(old['bootstrap']['commit'])['root_set'];oldcore=core(old)
 for r in operation['records'].values():b.remember(r['input'])
 states={role:copy.deepcopy(b.values[key(oldroot[role])]) for role in ['active','recovery']}
 if not retire:
  entries=operation['records']['id-index-4']['input']['entries'];index=copy.deepcopy(operation['records']['operation-root-4']['input'])
  for kind,sortby,field in [('operation-id','operation_id','by_id'),('operation-ordinal','acceptance_ordinal','by_ordinal')]:
   ordered=sorted(entries,key=lambda e:e[sortby] if sortby=='operation_id' else int(e[sortby]));children=[]
   for part in [ordered[:2],ordered[2:]]:
    leaf=obj(kind+'-leaf',count=str(len(part)),entries=part);r=b.remember(leaf)
    children.append(dict(first=part[0][sortby],last=part[-1][sortby],count=str(len(part)),child=r))
   index[field]=b.remember(obj(kind+'-branch',count='4',children=children))
  indexref=b.remember(index);s=states['active'];oldstate=copy.deepcopy(s)
  new_id='84000000-0000-4000-8000-000000000001';resource=copy.deepcopy(b.values[key(s['resource_state'])])
  for entry in resource['obligations']:
   obligation=copy.deepcopy(b.values[key(entry['obligation'])]);obligation['origin']['source_id']=new_id
   obligation['revision']=str(int(obligation['revision'])+1);entry['obligation']=b.remember(obligation)
  s.update(root_id=new_id,resource_state=b.remember(resource),authored_revision='4',authored=ref(operation['records']['authored-3']['input']),operation_index=indexref,accepted=index['accepted'],journal_inclusion=operation['selected_roots']['active']['journal_inclusion'],predecessor=dict(root_sha256=oldroot['active']['sha256'],authored_revision=oldstate['authored_revision']))
  rolebase=closure(b.values,[s[f] for f in ['authored','history','resource_state','operation_index']]);s['inventory'],_=b.tree('inventory',[ref(rolebase[k]) for k in sorted(rolebase)])
 if retire and mode=='changed-retirement-state':states['active']['predecessor']['root_sha256']='f'*64
 shared=[2,3] if retire else [1,2,3]
 if retire:del b.files[aid(1)];del b.locations[aid(1)]
 oldends={a:len(v['raw']) for a,v in b.files.items()};ends={aid(n):oldends[aid(n)]+STEP for n in [4,5,6,7]}
 charges=[]
 for n in shared:
  charges.append(next(v for v in b.values.values() if v.get('schema',{}).get('id')=='example.ps2.sealed-charge' and v['allocation_id']==aid(n)))
 if retire and mode=='changed-survivor-charge':
  charges=copy.deepcopy(charges);charges[0]['extensions']['example.route-negative']=True
 chargeroot,chargenodes=b.tree('owned-charge',[dict(allocation_id=v['allocation_id'],charge=b.remember(v),charged_high_water=v['charged_high_water']) for v in charges],1)
 role_records={};staterefs={}
 for role,n in [('active',4),('recovery',6)]:
  s=states[role];staterefs[role]=b.remember(s)
  semantic=closure(b.values,[s[f] for f in ['authored','history','resource_state','operation_index','inventory']]);semantic[key(ref(s))]=s
  role_records[role]={k:(v,1) for k,v in semantic.items()}
 union={k:v for records in role_records.values() for k,(v,_) in records.items()}
 conversion=b.values[key(oldroot['conversion_source'])]
 union.update({key(ref(v)):v for v in charges+chargenodes+[conversion]})
 unionroot,unionnodes=b.tree('inventory',[ref(union[k]) for k in sorted(union)])
 placements={};owners={}
 for role,n in [('active',4),('recovery',6)]:
  globals=[(v,2) for v in charges+chargenodes]+[(conversion,1)]+[(v,1) for v in unionnodes]
  role_records[role].update({key(ref(v)):(v,t) for v,t in globals})
  sharedkeys={k for allocation in shared for k in b.locations[aid(allocation)]}
  b.pack(n,'data',[(v,t) for k,(v,t) in role_records[role].items() if k not in sharedkeys])
  claims=[]
  for allocation in shared+[n,n+1]:
   sealed=allocation in shared;raw=b.files[aid(allocation)]['raw'] if sealed else b''
   charge=next((v for v in charges if v['allocation_id']==aid(allocation)),None)
   claims.append(obj('ownership-claim',allocation_id=aid(allocation),arena='metadata' if allocation==n+1 else 'data',owned_extent=str(len(raw) if sealed else ends[aid(allocation)]),authenticated_prefix=dict(byte_length=str(len(raw)),sha256=sha(raw)),sealed=sealed,sealed_charge=ref(charge) if sealed else None))
  loc=b.pack(n,'data',[(v,2) for v in claims]);owner=obj('ownership-branch',count=str(len(claims)),children=[dict(allocation_id=v['allocation_id'],node=loc[key(ref(v))]) for v in claims]);b.pack(n,'data',[(owner,2)]);owners[role]=loc[key(ref(owner))]
  role_records[role].update({key(ref(v)):(v,2) for v in claims+[owner]})
  locations={}
  for allocation in shared+[n]:locations.update(b.locations[aid(allocation)])
  entries=[dict(object=ref(v),membership='semantic' if tag==1 else 'ownership',physical=locations[k]) for k,(v,tag) in sorted(role_records[role].items())]
  midpoint=len(entries)//2;children=[]
  for part in [entries[:midpoint],entries[midpoint:]]:
   leaf=obj('locator-leaf',count=str(len(part)),entries=part);p=b.pack(n+1,'metadata',[(leaf,3)])[key(ref(leaf))];children.append(dict(first=part[0]['object'],last=part[-1]['object'],count=str(len(part)),child=p))
  placements[role]=b.pack(n+1,'metadata',[(obj('locator-branch',count=str(len(entries)),children=children),3)])[key(ref(obj('locator-branch',count=str(len(entries)),children=children)))]
  for allocation in [n,n+1]:
   raw=b.files[aid(allocation)]['raw'];gap=ends[aid(allocation)]-len(raw)-16;assert gap>=0,'preproved finite corridor exhausted'
   raw.extend(b'PS2PKD01'+bytes(4)+struct.pack('<I',gap)+bytes(gap))
 ledger=copy.deepcopy(oldcore);ledger['sealed_charge_tree']=chargeroot;ledger['sealed_observations']=[x for x in ledger['sealed_observations'] if x['witness']['allocation_id'] in [aid(n) for n in shared]]
 for tip in ledger['tips']:tip['extent']=tip['registered_charge']=str(ends[tip['witness']['allocation_id']])
 ledger['tickets']=[dict(token='202',allocation_id=aid(1),registered_charge='4096')] if retire else []
 ledger['total_charge']=str(int(oldcore['total_charge'])+4*STEP)
 target=dict(active=staterefs['active'],recovery=staterefs['recovery'],operation_index=states['active']['operation_index'],conversion_source=oldroot['conversion_source'],base_inventory=unionroot,placement=dict(generation=str(int(oldroot['placement']['generation'])+1),active_locator=placements['active'],recovery_locator=placements['recovery'],active_ownership=owners['active'],recovery_ownership=owners['recovery']))
 recipe=[]
 for n in [4,5,6,7]:
  a=aid(n);raw=bytes(b.files[a]['raw']);before=bytes.fromhex(old['allocations'][a]['hex']);suffix=raw[len(before):]
  recipe.append(dict(allocation_id=a,arena=b.files[a]['arena'],witness=b.files[a]['witness'],old_end=str(len(before)),old_sha256=sha(before),end=str(len(raw)),sha256=sha(raw),append_sha256=sha(suffix),frame_count=str(sum(1 for _ in frames(suffix)))))
 files={a:dict(arena=v['arena'],hex=v['raw'].hex(),sha256=sha(v['raw']),witness=v['witness']) for a,v in b.files.items()}
 return dict(target=target,recipe=recipe,files=files,ledger=ledger,base_entries=[ref(union[k]) for k in sorted(union)],states=states)

def original(old,p,operation,retire):
 commit=json.loads(old['bootstrap']['commit']);oldroot=commit['root_set'];control=json.loads(old['loose'][oldroot['placement']['accounting']['sha256']]);envelope=control if control['schema']['id']=='example.ps2.route-accounting' else None
 overlay=json.loads(old['loose'][oldroot['inventory']['sha256']]) if envelope else None
 source=old['allocations'][aid(1)] if retire else None
 return obj('route-original',token='202' if retire else '101',kind='retire' if retire else 'graph',scope=dict(profile=j.PROFILE,incarnation=j.INCARNATION,codec='example.ps2.route-v1'),old=dict(head=json.loads(old['bootstrap']['head']),commit=commit,ledger=core(old),envelope=envelope,overlay=overlay,states={role:inflate(old).values[key(oldroot[role])] for role in ['active','recovery']}),target=p['target'],recipe=dict(codec='example.ps2.closed-layout-v1',allocations=p['recipe'],source=dict(allocation_id=aid(1),witness=source['witness'],byte_length=str(len(bytes.fromhex(source['hex']))),sha256=source['sha256'],registered_charge='4096',charge=next(v for v in inflate(old).values.values() if v.get('schema',{}).get('id')=='example.ps2.sealed-charge' and v['allocation_id']==aid(1)),charge_path=list(closure(inflate(old).values,[core(old)['sealed_charge_tree']]).values())) if retire else None),reserve=str(R),project_limit=str(LIMIT),cleanup_bound=str(CLEANUP),maximum_control_bytes=str(j.STANDING),maximum_control_count='24',operation=None if retire else dict(intent=ref(operation['records']['intent-4']['input']),receipt=ref(operation['records']['receipt-4']['input'])))

def publish(previous,base,p,o,stage,operation):
 admitted=stage=='admitted';retire=o['kind']=='retire';clean=stage=='clean';unlinked=retire and stage in ['unlinked','clean'];ledger=copy.deepcopy(core(base) if admitted else p['ledger']);consumed=0 if admitted else 4*STEP
 if clean and retire:ledger['tickets']=[];ledger['total_charge']=str(int(ledger['total_charge'])-4096)
 phase=obj('route-phase',original=ref(o),stage=stage,consumed=str(consumed),remaining=str(0 if clean else R-consumed))
 if not admitted:phase['observations']=[dict(allocation_id=t['witness']['allocation_id'],witness=t['witness'],extent=t['extent'],registered_charge=t['registered_charge']) for t in ledger['tips']]
 if clean:phase.update(credit='4096' if retire else '0',cleanup=dict(remaining_roles=[],directory_barrier=True))
 envelope=obj('route-accounting',ledger=ref(ledger),original=ref(o),phase=ref(phase),project_limit=str(LIMIT),reserve=str(R),consumed=str(consumed),remaining=phase['remaining'])
 root=copy.deepcopy(json.loads(base['bootstrap']['commit'])['root_set'])
 if not admitted:
  root.update({k:v for k,v in p['target'].items() if k not in ['base_inventory','placement']});root['placement']=copy.deepcopy(p['target']['placement']);base_inv=p['target']['base_inventory'];base_entries=p['base_entries']
 else:
  old_overlay=o['old']['overlay'];base_inv=old_overlay['base'] if old_overlay else root['inventory'];base_entries=inventory_entries(inflate(base).values,base_inv)
 root['placement']['accounting']=ref(envelope)
 controls=[ledger,envelope,o,phase]
 if not retire:controls += [operation['records']['intent-4']['input'],operation['records']['receipt-4']['input']]
 basekeys={key(r) for r in base_entries};controlrefs=sorted({key(ref(v)):ref(v) for v in controls if key(ref(v)) not in basekeys}.values(),key=key)
 overlay=obj('inventory-overlay',base=base_inv,controls=controlrefs,count=str(len(base_entries)+len(controlrefs)))
 root['inventory']=ref(overlay)
 oldcommit=json.loads(previous['bootstrap']['commit']);revision=int(oldcommit['package_revision'])+1
 commit=copy.deepcopy(oldcommit);commit['required_features']=sorted(set(commit['required_features'])|{'example.ps2.selected-route.draft-v1','example.ps2.inventory-overlay.draft-v1'});commit.update(commit_id=f'85000000-0000-4000-8000-{revision:012d}',write_id=f'86000000-0000-4000-8000-{revision:012d}',package_revision=str(revision),parent=dict(commit_id=oldcommit['commit_id'],sha256=sha(canonical(oldcommit))),root_set=root)
 state=inflate(base).values[key(root['active'])] if admitted else p['states']['active']
 commit.update(authored=state['authored'],history=state['history'],inventory=state['inventory'])
 head=copy.deepcopy(json.loads(previous['bootstrap']['head']));head.update(commit_id=commit['commit_id'],commit_sha256=sha(canonical(commit)))
 loose={ref(v)['sha256']:canonical(v).decode() for v in controls+[overlay]}
 # Original frozen base contains its direct accounting object as an exact member.
 base_locations={k for loc in inflate(base).locations.values() for k in loc}
 for r in base_entries:
  if r['sha256'] in base['loose'] and key(r) not in base_locations:loose.setdefault(r['sha256'],base['loose'][r['sha256']])
 files=copy.deepcopy(base['allocations'] if admitted else p['files'])
 if retire and not unlinked:files[aid(1)]=base['allocations'][aid(1)]
 control_bytes=sum(len(x.encode()) for x in loose.values())+sum(len(canonical(v)) for v in [json.loads(base['bootstrap']['manifest']),head,commit])
 assert control_bytes<=j.STANDING,(stage,control_bytes)
 return dict(status='unfrozen-joined-settled-candidate',qualification=False,bootstrap=dict(manifest=base['bootstrap']['manifest'],head=canonical(head).decode(),commit=canonical(commit).decode()),loose=loose,allocations=files,source_files=base['source_files'],source_witnesses=base['source_witnesses'],expected=dict(total_charge=ledger['total_charge'],control_bytes=control_bytes,stage=stage,token=o['token']))

def inventory_entries(pool,r):
 v=pool[key(r)]
 return v['entries'] if v['schema']['id'].endswith('-leaf') else [e for c in v['children'] for e in inventory_entries(pool,c['child'])]

def control_peak(before,after):
 files={('manifest',sha(before['bootstrap']['manifest'].encode())):before['bootstrap']['manifest'].encode()}
 for world in [before,after]:
  raw=world['bootstrap']['commit'].encode();files[('commit',sha(raw))]=raw
  for name,value in world['loose'].items():
   raw=value.encode();assert name==sha(raw);files[('loose',name)]=raw
 return dict(rounded_bytes=sum(j.rounded(len(v)) for v in files.values())+sum(j.rounded(len(world['bootstrap']['head'].encode())) for world in [before,after])+3*4096,roles=len(files)+5)

def build(mode="valid"):
 old=json.loads((HERE.parent/'joined/linked-joined.json').read_text());operation=json.loads((HERE/'operation-four.json').read_text());snapshots={'old':old}
 p=payload(old,operation);o=original(old,p,operation,False);previous=old
 for stage in ['admitted','published','clean']:
  previous=publish(previous,old,p,o,stage,operation);snapshots['graph-'+stage]=previous
 base=previous;p=payload(base,operation,True,mode);o=original(base,p,operation,True)
 for stage in ['admitted','released','unlinked','clean']:
  previous=publish(previous,base,p,o,stage,operation);snapshots['retire-'+stage]=previous
 intents={};peaks={};previous=snapshots['old']
 for name,snapshot in list(snapshots.items())[1:]:
  root=json.loads(snapshot['bootstrap']['commit'])['root_set'];envelope=json.loads(snapshot['loose'][root['placement']['accounting']['sha256']]);original_value=json.loads(snapshot['loose'][envelope['original']['sha256']])
  value=obj('route-selector-intent',token=original_value['token'],original=ref(original_value),current_head_sha256=sha(previous['bootstrap']['head'].encode()),next_head_sha256=sha(snapshot['bootstrap']['head'].encode()))
  raw=canonical(value);assert len(raw)<=4096;intents[name]=dict(input=value,canonical=raw.decode(),sha256=sha(raw),byte_length=len(raw));peaks[name]=control_peak(previous,snapshot);assert peaks[name]['rounded_bytes']<=j.STANDING and peaks[name]['roles']<=24;previous=snapshot
 blobs={}
 for snapshot in snapshots.values():
  references={}
  for allocation,a in snapshot['allocations'].items():blobs[a['sha256']]=a;references[allocation]=a['sha256']
  snapshot['allocations']=references;snapshot.pop('records',None)
 return dict(status='unfrozen-single-head-route',qualification=False,allocation_blobs=blobs,transient_intents=intents,control_peaks=peaks,snapshots=snapshots,order=list(snapshots),operation_four_sha256=sha((HERE/'operation-four.json').read_bytes()))
if __name__=='__main__':
 out=build();(HERE/'linked-route.json').write_text(json.dumps(out,sort_keys=True,indent=2)+'\n');
 negatives={}
 for mode in ['changed-retirement-state','changed-survivor-charge']:
  candidate=build(mode);snapshot=candidate['snapshots']['retire-released'];negatives[mode]=dict(snapshot=snapshot,allocation_blobs={h:candidate['allocation_blobs'][h] for h in snapshot['allocations'].values() if h not in out['allocation_blobs']})
 (HERE/'route-negatives.json').write_text(json.dumps(negatives,sort_keys=True,indent=2)+'\n');print({k:v['expected'] for k,v in out['snapshots'].items() if k!='old'})
