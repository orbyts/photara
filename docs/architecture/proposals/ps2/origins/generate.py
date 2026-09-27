#!/usr/bin/env python3
"""Independent selected-origin bytes; standalone worlds, never live transitions."""
import copy
import importlib.util
import json
import sys
from pathlib import Path
sys.dont_write_bytecode = True
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('settled_builder',HERE.parent/'integrated/generate.py');base=importlib.util.module_from_spec(spec);spec.loader.exec_module(base)
FEATURE='photara.resource-retention-evidence.v1'
POLICY=base.uid(9001)

def build(mode='valid',rotation=False):
 role_info={}
 def resource_builder(root_id,slot,state,oldclosure,b):
  pool=dict(oldclosure)
  def add(v):
   r=b.add(v);pool[base.key(r)]=v;return r
  state=copy.deepcopy(state)
  master=state['requirements'];node=pool[base.key(master)];assert len(node['children'])==2
  if mode=='conflicting-requirement' and slot==3:
   node=copy.deepcopy(node);leaf=copy.deepcopy(pool[base.key(node['children'][0]['child'])]);entry=leaf['entries'][0];requirement=copy.deepcopy(pool[base.key(entry['record'])]);requirement['minimum_qualified_copies']='2';entry['record']=add(requirement);node['children'][0]['child']=add(leaf);master=add(node);state['requirements']=master
  first=node['children'][0]['child'];last=node['children'][1]['child']
  if slot==1:specs=[('authored',root_id,master),('history',root_id,first),('explicit',POLICY,first)]
  elif slot==2:specs=[('recovery',root_id,master),('explicit',POLICY,last)]
  else:specs=[('history',root_id,master),('explicit',POLICY,master)]
  sources=[]
  for i,(kind,source_id,requirements) in enumerate(specs):
   if mode=='pending-unsupported' and slot==1 and i==0:kind='pending';source_id=base.uid(9100)
   if mode=='wrong-owning-root' and slot==3 and kind=='history':source_id=base.uid(101)
   association=base.uid(10000+slot*10+i)
   if mode=='conflicting-association' and slot==3 and i==0:association=base.uid(10010)
   v=base.obj('photara.resource.retention-source',association_id=association,origin=dict(kind=kind,source_id=source_id),requirements=requirements)
   sources.append(dict(id=association,record=add(v)))
  source_tree=base.obj('photara.resource.selection-leaf',selection_kind='retention_sources',count=str(len(sources)),entries=sorted(sources,key=lambda e:e['id']))
  state=copy.deepcopy(state);state['retention_sources']=add(source_tree);state_ref=add(state)
  closure={}
  def visit(r):
   k=base.key(r)
   if k in closure:return
   v=pool[k];closure[k]=v;name=v['schema']['id'];edges=[]
   if name=='photara.resource.state':edges=[v[f] for f in ['identities','working_bindings','versions','backings','requirements','retention_sources']]
   elif name=='photara.resource.selection-leaf':edges=[e['record'] for e in v['entries']]
   elif name=='photara.resource.selection-branch':edges=[e['child'] for e in v['children']]
   elif name=='photara.resource.retention-source':edges=[v['requirements']]
   elif name=='photara.resource.captured-version':edges=[v['capture_evidence']]
   elif name=='photara.resource.backing':edges=[v['publication_evidence']]
   for e in edges:visit(e)
  visit(state_ref)
  role_info[slot]=dict(root_id=root_id,master=master,first=first,last=last,pool=pool)
  return state_ref,closure
 def evidence_builder(states,role_records,semantic,pins,b):
  pool={}
  for records in role_records.values():pool.update(records)
  for info in role_info.values():pool.update(info['pool'])
  extras={};entries=[]
  def add(v):
   r=b.add(v);extras[base.key(r)]=v;return r
  def include(r):
   k=base.key(r)
   if k in extras:return
   v=pool[k];add(v)
   if v['schema']['id']=='photara.resource.selection-branch':
    for c in v['children']:include(c['child'])
   elif v['schema']['id']=='photara.resource.selection-leaf':
    for e in v['entries']:include(e['record'])
  def body(name,**kw):return base.obj(name,library_id=base.L,bootstrap_sha256=role_records['active'][base.key(states['active'])]['bootstrap_sha256'],**kw)
  policy=body('photara.resource.retention-policy',policy_id=POLICY,revision='1',requirements=role_info[1]['master'])
  if mode=='wrong-policy-id':policy['policy_id']=base.uid(9999)
  if mode=='wrong-policy-subset':policy['requirements']=role_info[1]['first']
  if mode=='noncanonical-policy-revision':policy['revision']='01'
  if mode=='unknown-policy-field':policy['unknown_required']=True
  entries.append(dict(key=dict(kind='explicit',source_id=POLICY),evidence=add(policy)))
  for role,slot in [('active',1),('retained',3)]:
   state=role_records[role][base.key(states[role])]
   v=body('photara.resource.history-retention-context',source_id=state['root_id'],state_root=states[role],history=state['history'],promise='reconstruct-selected-versions',requirements=role_info[slot]['first' if slot==1 else 'master'])
   if mode=='wrong-history-root' and slot==1:v['state_root']=states['recovery']
   if mode=='wrong-history-projection' and slot==1:v['history']=state['authored']
   if mode=='missing-history-context' and slot==1:continue
   entries.append(dict(key=dict(kind='history',source_id=state['root_id']),evidence=add(v)))
  state=role_records['recovery'][base.key(states['recovery'])]
  selected_pins=copy.deepcopy(pins)
  role=dict(kind='current-recovery')
  if rotation:
   selected_pins['entries'][0]['root']=states['recovery'];selected_pins['entries'][0]['reason']='unresolved-recovery';role=dict(kind='retained-recovery',pin_id=selected_pins['entries'][0]['pin_id'])
   if mode=='wrong-rotation-pin-reason':selected_pins['entries'][0]['reason']='explicit-history'
  if mode=='wrong-recovery-pin':role=dict(kind='retained-recovery',pin_id=base.uid(9876))
  v=body('photara.resource.recovery-retention-context',source_id=state['root_id'],state_root=states['recovery'],role=role,requirements=role_info[2]['master'])
  entries.append(dict(key=dict(kind='recovery',source_id=state['root_id']),evidence=add(v)))
  # Required context metadata and subset tree pages are actual packed globals.
  # These do not make missing foreign authored closures readable in recovery mode.
  for role in ['active','recovery','retained']:
   include(states[role]);include(role_records[role][base.key(states[role])]['history'])
  for info in role_info.values():include(info['master'])
  entries.sort(key=lambda e:(e['key']['kind'],e['key']['source_id']))
  if mode=='duplicate-evidence-key':entries.insert(1,copy.deepcopy(entries[0]))
  children=[]
  for i in range(0,len(entries),2):
   part=entries[i:i+2];v=base.obj('photara.resource.retention-evidence-leaf',count=str(len(part)),entries=part);children.append(dict(first=part[0]['key'],last=part[-1]['key'],count=str(len(part)),child=add(v)))
  evidence=base.obj('photara.resource.retention-evidence-branch',count=str(len(entries)),children=children)
  if mode=='wrong-evidence-count':evidence['count']=str(len(entries)+1)
  if mode=='unknown-evidence-version':evidence['schema']['version']=2
  if mode=='missing-policy-body':extras.pop(base.key(base.ref(policy)))
  # Root is included separately by the shared physical builder.
  result=dict(evidence=evidence,records=list(extras.values()),pins=selected_pins)
  if rotation:result['recovery']=states['retained']
  return result
 baseline_mode='missing-shared-ownership' if mode=='missing-shared-ownership' else 'valid'
 world=base.build(baseline_mode,resource_builder,evidence_builder,[] if mode=='missing-capability' else [FEATURE])
 return world
if __name__=='__main__':
 worlds={'selected':build(),'rotated':build(rotation=True)}
 output=dict(status='unfrozen-selected-origin-candidate',production=False,transition=False,worlds=worlds)
 (HERE/'linked.json').write_text(json.dumps(output,sort_keys=True,indent=2)+'\n')
 modes=['wrong-policy-id','wrong-policy-subset','noncanonical-policy-revision','unknown-policy-field','wrong-history-root','wrong-history-projection','missing-history-context','wrong-recovery-pin','duplicate-evidence-key','wrong-evidence-count','unknown-evidence-version','pending-unsupported','wrong-owning-root','conflicting-association','conflicting-requirement','missing-capability','missing-shared-ownership','missing-policy-body']
 negatives={m:dict(base='selected',delta=base.delta(worlds['selected'],build(m))) for m in modes}
 negatives['wrong-rotation-pin-reason']=dict(base='rotated',delta=base.delta(worlds['rotated'],build('wrong-rotation-pin-reason',rotation=True)))
 (HERE/'negatives.json').write_text(json.dumps(negatives,sort_keys=True,indent=2)+'\n')
 print({name:world['expected'] for name,world in worlds.items()})
