#!/usr/bin/env python3
"""Unfrozen same-tip selected byte states; no filesystem durability or publication."""
import copy, importlib.util, json, struct, sys
from pathlib import Path
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('admission_builder', HERE.parent/'admission/generate.py')
a = importlib.util.module_from_spec(spec); spec.loader.exec_module(a)
g = a.g
STAGES = ['admitted','journal-durable','payload-durable','finalizer-selected','finalizer-durable','published','clean']

def spans(old, full):
 payload=[];writes=[]
 for n in range(2,9):
  id=g.aid(n); raw=bytes.fromhex(full[id]['hex']);start=len(bytes.fromhex(old['allocations'][id]['hex']));end=start
  if full[id]['arena']=='data':
   while end<len(raw):
    tag=raw[end+8];stop=end+16+struct.unpack('<I',raw[end+12:end+16])[0]
    if tag==0 or json.loads(raw[end+16:stop])['schema']['id']=='photara.storage.allocation-claim':break
    end=stop
  def segment(lo,hi):
   p=lo;count=0
   while p<hi: count+=1;p+=16+struct.unpack('<I',raw[p+12:p+16])[0]
   assert p==hi
   return dict(allocation_id=id,arena=full[id]['arena'],witness=full[id]['witness'],original_end=str(lo),final_end=str(hi),frame_count=str(count),framed_sha256=g.sha(raw[lo:hi]))
  payload.append(segment(start,end));writes.append(segment(end,len(raw)))
 return payload,writes

def build():
 original=a.g.load(HERE.parent/'integrated/linked.json');admitted=a.g.load(HERE.parent/'admission/linked.json');operation=a.g.load(HERE.parent/'route/operation-four.json');dry=a.planned(original,operation)
 initial=a.admission(original,dry,operation);selected_commit=json.loads(initial['bootstrap']['commit']);root=selected_commit['root_set'];pool={k:json.loads(v) for k,v in initial['loose'].items()};env=pool[root['placement']['accounting']['sha256']];holds=pool[env['holds']['sha256']];O=pool[holds['entries'][0]['original']['sha256']]
 assert initial['bootstrap']==admitted['selected']['bootstrap'] and initial['loose']==admitted['selected']['loose']
 payload,writes=spans(original,dry['allocations']);target={**dry['target'], 'retention_evidence':root['retention_evidence'],'base_inventory':dry['base_inventory'],'placement':dict(generation='2',root_placements=dry['root_placements'])}
 target['active_state']=json.loads(dry['records'][dry['target']['active']['sha256']])
 F=g.obj('photara.storage.finalization-control',token=O['token'],original=g.ref(O),prepared_receipt=operation['records']['receipt-4']['input'],generation_plan=None,newborn_binding=None,payload_completion=payload,sealed_allocations=[],recipe_codec=O['payload_recipe']['codec'],writes=writes,target_projection=target)
 assert len(g.enc(F))<=int(O['control_bounds']['finalization_bytes'])
 oldledger=O['old']['ledger'];nextledger=copy.deepcopy(oldledger)
 for tip in nextledger['tips']:
  end=len(bytes.fromhex(dry['allocations'][tip['allocation_id']]['hex']));tip['extent']=tip['registered_charge']=str(end);tip['observation']['measured_extent']=tip['observation']['charged_high_water']=str(end)
 C=sum(int(t['registered_charge'])-int(o['registered_charge']) for t,o in zip(nextledger['tips'],oldledger['tips']));nextledger['total_charge']=str(int(oldledger['total_charge'])+C)
 frame=operation['records']['journal-frame-4']['input'];intent=operation['records']['intent-4']['input'];receipt=operation['records']['receipt-4']['input'];previous=json.loads(original['bootstrap']['commit']);snapshots={}
 for i,stage in enumerate(STAGES):
  if i==0: c=copy.deepcopy(initial)
  else:
   c=copy.deepcopy(initial);published=i>=5;ledger=nextledger if published else oldledger;remaining=0 if i==6 else int(O['reserve'])-C if published else int(O['reserve'])
   P=g.obj('photara.storage.operation-phase',token=O['token'],original=g.ref(O),stage=stage,consumed=str(C if published else 0),remaining=str(remaining),newborn_binding=None,finalization=g.ref(F) if i>=3 else None,cleanup=dict(remaining_roles=[],directory_barrier=True,released=str(int(O['reserve'])-C)) if i==6 else None,journal_completion=dict(frame=frame,barrier=True))
   hold=g.obj('photara.storage.hold-leaf',count='1',reserved=O['reserve'],consumed=P['consumed'],remaining=P['remaining'],entries=[dict(token=O['token'],original=g.ref(O),phase=g.ref(P))]);env=g.obj('photara.storage.accounting-envelope',ledger=g.ref(ledger),holds=g.ref(hold));controls=[ledger,env,O,P,hold,intent]+([] if published else [receipt])+([F] if i>=3 else [])
   base=dry['base_inventory'] if published else O['old']['overlay']['base'];basecount=int(dry['global_count']) if published else int(O['old']['overlay']['count'])-len(O['old']['overlay']['controls']);overlay=g.obj('photara.package.inventory-overlay',base=base,controls=sorted([g.ref(v) for v in controls],key=g.key),count=str(basecount+len(controls)))
   commit=copy.deepcopy(O['old']['commit']);r=commit['root_set'];r['inventory']=g.ref(overlay);r['placement']['accounting']=g.ref(env)
   if published:
    r.update(dry['target']);r['placement'].update(generation='2',root_placements=dry['root_placements']);state=json.loads(dry['records'][dry['target']['active']['sha256']]);commit.update(authored=state['authored'],history=state['history'],inventory=state['inventory'])
   commit.update(commit_id=g.uid(9200+i*2),write_id=g.uid(9201+i*2),package_revision=str(int(previous['package_revision'])+1),parent=dict(commit_id=previous['commit_id'],commit_sha256=g.sha(g.enc(previous))))
   head=copy.deepcopy(O['old']['head']);head.update(commit_id=commit['commit_id'],commit_sha256=g.sha(g.enc(commit)));c['bootstrap']['head']=g.enc(head).decode();c['bootstrap']['commit']=g.enc(commit).decode();c['loose']={g.ref(v)['sha256']:g.enc(v).decode() for v in controls+[overlay]}
  ends={k:str(len(bytes.fromhex(v['hex']))) for k,v in original['allocations'].items()}
  if i>=2:ends.update({v['allocation_id']:v['final_end'] for v in payload})
  if i>=4:ends.update({v['allocation_id']:v['final_end'] for v in writes})
  c['journal']=original['journal']+([g.enc(frame).decode()] if i>=1 else []);c['ends']=ends
  for k in ['allocations','source_files','source_witnesses','records','expected']:c.pop(k,None)
  snapshots[stage]=c;previous=json.loads(c['bootstrap']['commit'])
 cuts={}
 for before,after in zip(STAGES,STAGES[1:]):
  oldhead=json.loads(snapshots[before]['bootstrap']['head']);nexthead=json.loads(snapshots[after]['bootstrap']['head']);intent=g.obj('photara.storage.admission-selector',token=O['token'],original=g.ref(O),old_head_sha256=g.sha(g.enc(oldhead)),next_head_sha256=g.sha(g.enc(nexthead)))
  cuts[before]={'selector.intent':g.enc(intent).decode(),'candidate.commit':snapshots[after]['bootstrap']['commit'],'HEAD.next':snapshots[after]['bootstrap']['head'],**{'control/'+k:v for k,v in snapshots[after]['loose'].items()}}
 return dict(status='unfrozen-same-tip-selected-phase-model',qualification=False,snapshots=snapshots,selector_cuts=cuts,allocations=dry['allocations'],source_files=original['source_files'],source_witnesses=original['source_witnesses'],expected=dict(reserve=O['reserve'],growth=str(C),remaining=str(int(O['reserve'])-C),payload_bytes=sum(int(x['final_end'])-int(x['original_end']) for x in payload),finalizer_bytes=sum(int(x['final_end'])-int(x['original_end']) for x in writes)))
if __name__=='__main__':
 c=build();(HERE/'linked.json').write_text(json.dumps(c,sort_keys=True,indent=2)+'\n');print(c['expected'])
