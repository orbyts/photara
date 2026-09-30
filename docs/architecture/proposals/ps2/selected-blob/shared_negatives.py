#!/usr/bin/env python3
"""Coherent negative rebuilds using the unchanged D19 preparation builder."""
import importlib.util,json,sys
from pathlib import Path
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('blob_base',HERE/'generate.py');g=importlib.util.module_from_spec(spec);spec.loader.exec_module(g)
base=g.build();original=g.obj
CASES={'state-version':'photara.package.state-root','state-unknown':'photara.package.state-root','empty-prefix':'photara.package.operation-index','raw-claim-layout':'photara.storage.allocation-claim','raw-claim-not-sealed':'photara.storage.allocation-claim','raw-charge-understated':'photara.storage.sealed-charge'}
out={}
for name,schema in CASES.items():
 def obj(schema_id,**kw):
  v=original(schema_id,**kw)
  if schema_id==schema:
   if name=='state-version':v['schema']['version']=3
   elif name=='state-unknown':v['unknown_mandatory']=True
   elif name=='empty-prefix':v['accepted']={**v['accepted'],'prefix_sha256':'a'*64}
   elif name=='raw-claim-layout' and v['layout']=='whole-blob':v['layout']='framed-json'
   elif name=='raw-claim-not-sealed' and v['layout']=='whole-blob':v['sealed']=False
   elif name=='raw-charge-understated':v['charged_high_water']='0'
  return v
 g.obj=obj
 altered=g.build();delta={k:altered[k] for k in ['bootstrap','loose']};delta['patches']={}
 for allocation,a in altered['allocations'].items():
  before=bytes.fromhex(base['allocations'][allocation]['hex']);after=bytes.fromhex(a['hex']);assert len(before)==len(after)
  patches=[];at=0
  while at<len(after):
   if before[at]==after[at]:at+=1;continue
   start=at
   while at<len(after) and before[at]!=after[at]:at+=1
   patches.append({'offset':start,'hex':after[start:at].hex()})
  if patches:delta['patches'][allocation]=patches
 out[name]=delta
 g.obj=original
(HERE/'shared-negatives.json').write_text(json.dumps(out,sort_keys=True,indent=2)+'\n')
# A second actual selected role has a fully reframed locator naming a different Blob
# in the same original allocation. Its metadata frames and outer hashes are rebuilt.
import copy
pack=g.g.Builder.pack
second={}
def alias_pack(b,n,arena,items):
 for v,tag in items:
  if v['schema']['id']=='photara.storage.root-placement-leaf':
   state=copy.deepcopy(next(x for x in b.values.values() if x['schema']['id']=='photara.package.state-root'))
   state['root_id']=g.uid(102);sr=g.ref(state);sloc=pack(b,1,'data',[(state,1)])[g.key(sr)]
   entries=[copy.deepcopy(e) for x in b.values.values() if x['schema']['id']=='photara.storage.locator-leaf' for e in x['entries']]
   old=v['entries'][0]['root']
   for e in entries:
    if e['object']==old:e['object']=sr;e['physical']=sloc
    if e['object']['kind']=='blob':e['object']['sha256']='a'*64;e['physical']['sha256']='a'*64
   entries.sort(key=lambda e:g.key(e['object']));children=[]
   for at in range(0,len(entries),64):
    part=entries[at:at+64];leaf=original('photara.storage.locator-leaf',count=str(len(part)),entries=part)
    pr=pack(b,2,'metadata',[(leaf,3)])[g.key(g.ref(leaf))]
    children.append(dict(first=part[0]['object'],last=part[-1]['object'],count=str(len(part)),child=pr))
   locator=children[0]['child']
   if len(children)>1:
    branch=original('photara.storage.locator-branch',count=str(len(entries)),children=children)
    locator=pack(b,2,'metadata',[(branch,3)])[g.key(g.ref(branch))]
   e=copy.deepcopy(v['entries'][0]);e.update(root=sr,root_id=state['root_id'],locator=locator)
   v['entries'].append(e);v['entries'].sort(key=lambda e:g.key(e['root']));v['count']='2';second['root']=sr
 return pack(b,n,arena,items)
def alias_obj(schema_id,**kw):
 v=original(schema_id,**kw)
 if schema_id=='photara.package.root-set':v['recovery']=second['root']
 return v
g.g.Builder.pack=alias_pack;g.obj=alias_obj
altered=g.build();g.g.Builder.pack=pack;g.obj=original
delta={k:altered[k] for k in ['bootstrap','loose']};delta['patches']={}
for allocation,a in altered['allocations'].items():
 before=bytes.fromhex(base['allocations'][allocation]['hex']);after=bytes.fromhex(a['hex']);assert len(before)==len(after)
 patches=[];at=0
 while at<len(after):
  if before[at]==after[at]:at+=1;continue
  start=at
  while at<len(after) and before[at]!=after[at]:at+=1
  patches.append({'offset':start,'hex':after[start:at].hex()})
 if patches:delta['patches'][allocation]=patches
out['cross-role-raw-alias']=delta
(HERE/'shared-negatives.json').write_text(json.dumps(out,sort_keys=True,indent=2)+'\n')
