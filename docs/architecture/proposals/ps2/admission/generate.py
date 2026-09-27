#!/usr/bin/env python3
"""Unfrozen admission-only construction; pure bytes, no package effects."""
import copy, importlib.util, json, struct, sys
from pathlib import Path
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('integrated',HERE.parent/'integrated/generate.py')
g=importlib.util.module_from_spec(spec);spec.loader.exec_module(g)
enc,sha,ref,key,obj,aid,uid=g.enc,g.sha,g.ref,g.key,g.obj,g.aid,g.uid
STEP=g.EXTENT

def unpack(c):
 b=g.Builder('valid')
 for allocation,a in c['allocations'].items():
  raw=bytes.fromhex(a['hex']);b.files[allocation]=dict(arena=a['arena'],raw=bytearray(raw),witness=a['witness']);b.locations[allocation]={};p=0
  while p<len(raw):
   assert raw[p:p+8]==b'PS2PKD01';tag=raw[p+8];end=p+16+struct.unpack('<I',raw[p+12:p+16])[0];assert end<=len(raw)
   if tag:
    v=json.loads(raw[p+16:end]);r=b.add(v);b.locations[allocation][key(r)]=dict(allocation_id=allocation,arena=a['arena'],offset=str(p),byte_length=str(end-p),record_sha256=sha(raw[p:end]))
   p=end
 for s in c['loose'].values():b.add(json.loads(s))
 return b

def closure(b,roots):
 out={}
 def walk(r):
  k=key(r)
  if k in out:return
  v=b.values[k];out[k]=v;s=v['schema']['id'];edges=[]
  if s=='photara.package.state-root':edges=[v[x] for x in ['authored','history','resource_state','operation_index','inventory']]
  elif s=='photara.project.authored':edges=[v[x] for x in ['party_assignments','location_assignments','asset_ledger','resource_ledger','context']]+[x['document'] for x in v['graphs']]
  elif s=='photara.project.saved-graph':edges=[v['context']]+[x['manifest'] for x in v['required_packages']+v['node_contracts']]
  elif s=='photara.context.authored':edges=[x['context'] for x in v['node_contexts']]
  elif s=='photara.resource.state':edges=[v[x] for x in ['identities','working_bindings','versions','backings','requirements','retention_sources']]
  elif s=='photara.resource.selection-leaf':edges=[x['record'] for x in v['entries']]
  elif s=='photara.resource.selection-branch':edges=[x['child'] for x in v['children']]
  elif s=='photara.resource.retention-source':edges=[v['requirements']]
  elif s=='photara.resource.captured-version':edges=[v['capture_evidence']]
  elif s=='photara.resource.backing':edges=[v['publication_evidence']]
  elif s=='photara.package.operation-index':edges=[v['by_id'],v['by_ordinal']]
  elif s in ['photara.package.operation-id-leaf','photara.package.operation-ordinal-leaf']:edges=[x['receipt'] for x in v['entries']]
  elif s in ['photara.package.operation-id-branch','photara.package.operation-ordinal-branch']:edges=[x['child'] for x in v['children']]
  elif s=='photara.package.inventory-leaf':edges=v['entries']
  elif s=='photara.package.inventory-branch':edges=[x['child'] for x in v['children']]
  for e in edges:
   if e is not None:walk(e)
 for r in roots:walk(r)
 return out

def planned(old,operation):
 b=unpack(old);commit=json.loads(old['bootstrap']['commit']);root=commit['root_set'];envelope=b.values[key(root['placement']['accounting'])];ledger=b.values[key(envelope['ledger'])]
 original_pool=dict(b.values);active=copy.deepcopy(b.values[key(root['active'])]);oldactive=copy.deepcopy(active)
 for row in operation['records'].values():b.add(row['input'])
 # New authored-origin associations have fresh identity, never rebound old identity.
 rs=copy.deepcopy(b.values[key(active['resource_state'])]);sources=copy.deepcopy(b.values[key(rs['retention_sources'])]);assert sources['schema']['id'].endswith('-leaf')
 for i,e in enumerate(sources['entries']):
  association=copy.deepcopy(b.values[key(e['record'])]);association['association_id']=uid(8000+i);association['origin']['source_id']=uid(8001);e.update(id=association['association_id'],record=b.add(association))
 rs['retention_sources']=b.add(sources);active['resource_state']=b.add(rs)
 entries=operation['records']['id-index-4']['input']['entries'];ir,ins=b.tree('photara.package.operation-id',entries,lambda e:e['operation_id'],size=1);orr,ons=b.tree('photara.package.operation-ordinal',entries,lambda e:int(e['acceptance_ordinal']),size=1)
 accepted=operation['records']['operation-root-4']['input']['accepted'];index=obj('photara.package.operation-index',version=2,library_id=root['library_id'],bootstrap_sha256=root['bootstrap_sha256'],accepted=accepted,by_id=ir,by_ordinal=orr)
 active.update(root_id=uid(8001),authored_revision='4',authored=ref(operation['records']['authored-3']['input']),operation_index=b.add(index),accepted=accepted,journal_inclusion=operation['selected_roots']['active']['journal_inclusion'],predecessor=dict(root_sha256=root['active']['sha256'],authored_revision=oldactive['authored_revision']))
 semantic=closure(b,[active[x] for x in ['authored','history','resource_state','operation_index']]);active['inventory'],_=b.tree('photara.package.inventory',[ref(semantic[k]) for k in sorted(semantic)])
 states={'active':b.add(active),'recovery':root['active'],'retained':old['expected']['roles']['retained']}
 role_records={role:closure(b,[r]) for role,r in states.items()}
 # Original globals are exact selected base minus old semantic roots/closures.
 oldroles={r:closure(b,[v]) for r,v in old['expected']['roles'].items()};oldsemantic=set().union(*(set(v) for v in oldroles.values()))
 oldoverlay=b.values[key(root['inventory'])];originalglobal=closure(b,[oldoverlay['base']]);globals_={k:v for k,v in originalglobal.items() if k not in oldsemantic and not v['schema']['id'].startswith('photara.package.inventory-')}
 union=dict(globals_)
 for records in role_records.values():union.update({k:v for k,v in records.items() if not v['schema']['id'].startswith('photara.package.inventory-')})
 baseinv,globalnodes=b.tree('photara.package.inventory',[ref(union[k]) for k in sorted(union)])
 ends={aid(n):len(b.files[aid(n)]['raw'])+STEP for n in range(2,9)};placements=[]
 # Original prefixes including padding remain immutable; every write is append-only.
 for role,n in [('active',2),('recovery',4),('retained',6)]:
  records={**role_records[role],**globals_,**{key(ref(v)):v for v in globalnodes}}
  b.pack(n,'data',[(v,2 if v['schema']['id'].startswith('photara.storage.') or v['schema']['id'].startswith('photara.package.retained-file-charge') else 1) for k,v in sorted(records.items()) if k not in b.locations[aid(1)]])
  claims=[]
  for unit in [1,n,n+1,8]:
   sealed=unit==1;raw=bytes(b.files[aid(1)]['raw']) if sealed else b'';charge=next(v for v in globals_.values() if v['schema']['id']=='photara.storage.sealed-charge') if sealed else None
   claims.append(obj('photara.storage.allocation-claim',allocation_id=aid(unit),arena='metadata' if unit in [n+1,8] else 'data',layout='framed-json',owned_extent=str(len(raw) if sealed else ends[aid(unit)]),authenticated_prefix=dict(byte_length=str(len(raw)),sha256=sha(raw)),sealed=sealed,sealed_charge=ref(charge) if sealed else None))
  owner,ownnodes=b.tree('photara.storage.ownership',[dict(allocation_id=v['allocation_id'],claim=ref(v)) for v in claims],lambda e:e['allocation_id'],size=2);b.pack(n,'data',[(v,2) for v in claims+ownnodes]);records.update({key(ref(v)):v for v in claims+ownnodes});locations={**b.locations[aid(1)],**b.locations[aid(n)]}
  entries=[dict(object=ref(v),membership='ownership' if v['schema']['id'].startswith('photara.storage.') or v['schema']['id'].startswith('photara.package.retained-file-charge') else 'semantic',physical=locations[k]) for k,v in sorted(records.items())];children=[]
  for i in range(0,len(entries),64):
   part=entries[i:i+64];v=obj('photara.storage.locator-leaf',count=str(len(part)),entries=part);p=b.pack(n+1,'metadata',[(v,3)])[key(ref(v))];children.append(dict(first=part[0]['object'],last=part[-1]['object'],count=str(len(part)),child=p))
  locator=obj('photara.storage.locator-branch',count=str(len(entries)),children=children);lp=b.pack(n+1,'metadata',[(locator,3)])[key(ref(locator))];placements.append(dict(root=states[role],root_id=b.values[key(states[role])]['root_id'],locator=lp,ownership=owner))
 children=[]
 for e in sorted(placements,key=lambda e:key(e['root'])):
  v=obj('photara.storage.root-placement-leaf',count='1',entries=[e]);p=b.pack(8,'metadata',[(v,3)])[key(ref(v))];children.append(dict(first=e['root'],last=e['root'],count='1',child=p))
 v=obj('photara.storage.root-placement-branch',count=str(len(placements)),children=children);pr=b.pack(8,'metadata',[(v,3)])[key(ref(v))]
 for n in range(2,9):
  raw=b.files[aid(n)]['raw'];gap=ends[aid(n)]-len(raw)-16;assert gap>=0,'admitted finalizer corridor exhausted';raw.extend(b'PS2PKD01'+bytes(4)+struct.pack('<I',gap)+bytes(gap))
 target=dict(active=states['active'],recovery=states['recovery'],pinned_roots=root['pinned_roots'],operation_index=active['operation_index'],conversion_source=root['conversion_source'])
 descriptors=[]
 for n in range(2,9):
  a=b.files[aid(n)];before=bytes.fromhex(old['allocations'][aid(n)]['hex']);after=bytes(a['raw']);assert after.startswith(before);p=len(before);count=0
  while p<len(after):count+=1;p+=16+struct.unpack('<I',after[p+12:p+16])[0]
  descriptors.append(dict(allocation_id=aid(n),arena=a['arena'],witness=a['witness'],original_end=str(len(before)),original_sha256=sha(before),final_end=str(len(after)),frame_count=str(count),framed_sha256=sha(after[len(before):])))
 return dict(target=target,base_inventory=baseinv,root_placements=pr,allocations={k:dict(arena=v['arena'],hex=bytes(v['raw']).hex(),witness=v['witness']) for k,v in b.files.items()},recipe=descriptors,records={k[0]:enc(v).decode() for k,v in b.values.items() if k not in original_pool},global_count=str(len(union)),states=states)

def prospective(old,dry,operation):
 c=copy.deepcopy(old);c['allocations']=dry['allocations'];commit=json.loads(c['bootstrap']['commit']);oldcommit=copy.deepcopy(commit);root=commit['root_set'];pool=unpack(old).values;envelope=copy.deepcopy(pool[key(root['placement']['accounting'])]);ledger=copy.deepcopy(pool[key(envelope['ledger'])])
 for tip in ledger['tips']:
  end=len(bytes.fromhex(dry['allocations'][tip['allocation_id']]['hex']));tip['extent']=tip['registered_charge']=str(end);tip['observation']['measured_extent']=tip['observation']['charged_high_water']=str(end)
 ledger['total_charge']=str(int(ledger['total_charge'])+7*STEP);envelope['ledger']=ref(ledger);controls=sorted([ref(ledger),ref(envelope)],key=key);overlay=obj('photara.package.inventory-overlay',base=dry['base_inventory'],controls=controls,count=str(int(dry['global_count'])+len(controls)))
 root.update(dry['target']);root['inventory']=ref(overlay);root['placement']=dict(generation='2',root_placements=dry['root_placements'],accounting=ref(envelope));active=json.loads(dry['records'][dry['target']['active']['sha256']]);commit.update(commit_id=uid(9001),write_id=uid(9002),package_revision='2',parent=dict(commit_id=oldcommit['commit_id'],commit_sha256=sha(enc(oldcommit))),authored=active['authored'],history=active['history'],inventory=active['inventory'])
 head=json.loads(c['bootstrap']['head']);head.update(commit_id=commit['commit_id'],commit_sha256=sha(enc(commit)));c['bootstrap']['head']=enc(head).decode();c['bootstrap']['commit']=enc(commit).decode();c['loose']={ref(v)['sha256']:enc(v).decode() for v in [ledger,envelope,overlay]};c['journal']=old['journal']+[operation['records']['journal-frame-4']['canonical']];c['expected'].update(total_charge=ledger['total_charge'],roles=dry['states'],global_members=int(dry['global_count'])+2)
 c['status']='unfrozen-prospective-sizing-only-not-selected';return c

def admission(old,dry,operation):
 c=copy.deepcopy(old);commit=json.loads(c['bootstrap']['commit']);oldcommit=copy.deepcopy(commit);head=json.loads(c['bootstrap']['head']);root=commit['root_set'];pool=unpack(old).values;envelope=copy.deepcopy(pool[key(root['placement']['accounting'])]);ledger=pool[key(envelope['ledger'])];oldoverlay=pool[key(root['inventory'])]
 request=[operation['records'][n]['input'] for n in ['intent-4','receipt-4']];reserve=7*STEP+65536
 original=obj('photara.storage.original-admission',token='1',kind='graph',nonce='17'*32,scope=dict(profile=g.PROFILE,incarnation=g.INC,directory=dict(device='7',inode='99')),original_codec='photara.codec.ps2-same-tip-admission-v1',request=dict(operation_id=request[0]['operation_id'],request_sha256=request[1]['request_sha256'],intent=ref(request[0]),receipt=ref(request[1])),old=dict(head=head,commit=oldcommit,envelope=copy.deepcopy(envelope),ledger=ledger,overlay=oldoverlay,states=[dict(root=r,value=pool[key(r)]) for r in sorted(old['expected']['roles'].values(),key=key)]),semantic_target=dry['target'],payload_recipe=dict(codec='photara.codec.ps2-same-tip-layout-v1',allocations=dry['recipe'],target_placement=dict(generation='2',root_placements=dry['root_placements']),base_inventory=dry['base_inventory']),old_hold=envelope['holds'],generation_plan=None,retention_intent=None,reserve=str(reserve),cleanup_bound='65536',project_limit=str(int(ledger['total_charge'])+reserve),control_bounds=dict(marker_bytes='0',newborn_binding_bytes='0',finalization_bytes='16384',phase_bytes='4096',aggregate_control_bytes=str(g.STANDING),aggregate_control_count='24'))
 phase=obj('photara.storage.operation-phase',token='1',original=ref(original),stage='admitted',consumed='0',remaining=str(reserve),newborn_binding=None,finalization=None,cleanup=None)
 holds=obj('photara.storage.hold-leaf',count='1',reserved=str(reserve),consumed='0',remaining=str(reserve),entries=[dict(token='1',original=ref(original),phase=ref(phase))]);envelope['holds']=ref(holds)
 loose=[ledger,envelope,original,phase,holds]+request;controls=sorted([ref(v) for v in loose],key=key);overlay=obj('photara.package.inventory-overlay',base=oldoverlay['base'],controls=controls,count=str(int(oldoverlay['count'])-len(oldoverlay['controls'])+len(controls)))
 root['inventory']=ref(overlay);root['placement']['accounting']=ref(envelope)
 commit.update(commit_id=uid(9101),write_id=uid(9102),package_revision='2',parent=dict(commit_id=oldcommit['commit_id'],commit_sha256=sha(enc(oldcommit))))
 nexthead=copy.deepcopy(head);nexthead.update(commit_id=commit['commit_id'],commit_sha256=sha(enc(commit)));c['bootstrap']['head']=enc(nexthead).decode();c['bootstrap']['commit']=enc(commit).decode();c['loose']={ref(v)['sha256']:enc(v).decode() for v in loose+[overlay]};c['status']='unfrozen-admission-only-no-packed-effects';c['expected'].update(reserve=str(reserve),accepted_operations='3',global_members=int(overlay['count']))
 return c

def build():
 old=g.load(HERE.parent/'integrated/linked.json');operation=g.load(HERE.parent/'route/operation-four.json');dry=planned(old,operation);future=prospective(old,dry,operation);selected=admission(old,dry,operation)
 # Original unchanged physical/source maps are referenced by digest and copied by the test loader.
 selected={k:v for k,v in selected.items() if k not in ['allocations','source_files','source_witnesses','records']}
 del dry['allocations']
 future.pop('records',None)
 return dict(status='unfrozen-admission-dry-run-work-in-progress',qualification=False,original_corpus_sha256=sha(enc(old)),operation_corpus_sha256=sha(enc(operation)),dry_run=dry,prospective=future,selected=selected)
if __name__=='__main__':
 result=build();(HERE/'linked.json').write_text(json.dumps(result,sort_keys=True,indent=2)+'\n');print({k:v for k,v in result['dry_run'].items() if k in ['recipe','global_count']})
