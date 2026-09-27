#!/usr/bin/env python3
"""UNFROZEN additive field/byte proposal, private metadata only; no package writes."""
import copy,hashlib,json
from pathlib import Path
HERE=Path(__file__).resolve().parent
BASE=HERE.parent/'resource-conversion/linked.json'
C=json.loads(BASE.read_text())['scenarios']['valid']
P='10000000-0000-4000-8000-000000000001';L='10000000-0000-4000-8000-000000000002'
FEATURE='photara.resource-state-trees.v1'
def enc(v):return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
def sha(b):return hashlib.sha256(b).hexdigest()
def ref(v):return dict(kind='json',sha256=sha(enc(v)),byte_length=str(len(enc(v))))
def key(r):return r['sha256'],int(r['byte_length'])
def obj(name,version=1,**kw):return dict(schema=dict(id='photara.resource.'+name,version=version),project_id=P,extensions={},**kw)
def value(r):return C['records'][r['sha256']]['input']
oldroot=value(C['roots']['active']);oldstate=value(oldroot['resource_state'])
templates={name:value(oldstate[name][0][field]) for name,field in [('identities','identity'),('working_bindings','binding'),('versions','version'),('backings','backing'),('requirements','obligation')] if name!='requirements'}
templates['requirements']=value(oldstate['obligations'][0]['obligation'])
templates['capture']=value(templates['versions']['capture_evidence']);templates['publication']=value(templates['backings']['publication_evidence'])
IDS={'identities':'resource_id','working_bindings':'binding_id','versions':'version_id','backings':'backing_id','requirements':'requirement_id'}
def build(n,mode='valid'):
 records={}
 def add(v):
  r=ref(v);records[r['sha256']]=dict(input=v,canonical=enc(v).decode(),sha256=r['sha256'],byte_length=len(enc(v)));return r
 def tree(kind,entries):
  entries=sorted(entries,key=lambda e:e['id']);levels=[]
  for start in range(0,max(1,len(entries)),4):
   part=entries[start:start+4];r=add(obj('selection-leaf',selection_kind=kind,count=str(len(part)),entries=part));levels.append(dict(first=part[0]['id'] if part else None,last=part[-1]['id'] if part else None,count=str(len(part)),child=r))
  while len(levels)>1:
   out=[]
   for start in range(0,len(levels),4):
    part=levels[start:start+4]
    if len(part)==1:out.extend(part);continue
    r=add(obj('selection-branch',selection_kind=kind,count=str(sum(int(x['count']) for x in part)),children=part));out.append(dict(first=part[0]['first'],last=part[-1]['last'],count=str(sum(int(x['count']) for x in part)),child=r))
   levels=out
  return levels[0]['child']
 entries={k:[] for k in IDS}
 for i in range(n):
  vs=copy.deepcopy(templates)
  if i:
   names=['resource_id','version_id','backing_id','binding_id','obligation_id','evidence_id']
   mapping={v[k]:f'92000000-0000-4000-8000-{i*100+j+1:012d}' for j,(v,k) in enumerate((v,k) for v in templates.values() for k in names if k in v)}
   def rewrite(v):
    if isinstance(v,str):return mapping.get(v,v)
    if isinstance(v,list):return [rewrite(x) for x in v]
    if isinstance(v,dict):return {k:rewrite(x) for k,x in v.items()}
    return v
   vs=rewrite(vs)
  vs['versions']['capture_evidence']=add(vs['capture']);vs['backings']['publication_evidence']=add(vs['publication'])
  req=vs['requirements'];req['schema']['id']='photara.resource.retention-requirement';req['requirement_id']=req.pop('obligation_id');req.pop('origin')
  if i==0:
   if mode=='two-copies':req['minimum_qualified_copies']='2'
   if mode=='zero-copies':req['minimum_qualified_copies']='0'
   if mode=='wrong-qualification':req['required_qualification']['failure_model_sha256']='d'*64
   if mode=='missing-backing':req['required_backing_ids']=['93000000-0000-4000-8000-000000000001']
   if mode=='unknown-field':req['origin']=dict(kind='authored',source_id='ignored')
   if mode=='old-requirement-version':req['schema']['version']=2
   if mode=='capture-policy-pointer':vs['versions']['retention']=add(req)
  for kind in IDS:
   v=vs[kind];r=add(v);entries[kind].append(dict(id=v[IDS[kind]],record=r))
 tree_roots={k:tree(k,v) for k,v in entries.items()}
 if mode=='wrong-tree-kind':tree_roots['versions']=tree_roots['backings']
 if mode=='unknown-tree-field':
  v=copy.deepcopy(records[tree_roots['versions']['sha256']]['input']);v['unknown']=True;tree_roots['versions']=add(v)
 groups=[[e] for e in entries['requirements']] if mode=='many-sources' else [entries['requirements'][::2],entries['requirements'][1::2]];groups=[g for g in groups if g]
 subsets=[tree('requirements',g) for g in groups]
 if mode=='conflicting-source-requirement':
  group=copy.deepcopy(groups[0]);changed=copy.deepcopy(records[group[0]['record']['sha256']]['input']);changed['minimum_qualified_copies']='2';group[0]['record']=add(changed);subsets[0]=tree('requirements',group)
 if mode=='wrong-tree-range':
  v=copy.deepcopy(records[tree_roots['versions']['sha256']]['input']);v['children'][0]['first']='93000000-0000-4000-8000-000000000001';tree_roots['versions']=add(v)
 roots={}
 for role,root_id in [('a','94000000-0000-4000-8000-000000000001'),('b','94000000-0000-4000-8000-000000000002')]:
  sources=[]
  for j,r in enumerate(subsets):
   association_id=f'95000000-0000-4000-8000-{(100 if role=="a" or mode=="reused-source-id" else 200)+j:012d}'
   if mode=='duplicate-source-id':association_id=f'95000000-0000-4000-8000-{100 if role=="a" else 200:012d}'
   origin=dict(kind='authored',source_id=root_id)
   if role=='b' and mode=='wrong-root':origin['source_id']='94000000-0000-4000-8000-000000000001'
   if role=='b' and mode=='unresolved-origin':origin['kind']='pending'
   v=obj('retention-source',association_id=association_id,origin=origin,requirements=r)
   if role=='b' and mode=='changed-source-id':v['association_id']='95000000-0000-4000-8000-000000009999'
   sources.append(dict(id=association_id,record=add(v)))
  if mode=='uncovered-requirement' and role=='b':sources=[]
  source_root=tree('retention_sources',sources)
  if mode=='source-alias':
   child=dict(first=sources[0]['id'],last=sources[-1]['id'],count=str(len(sources)),child=source_root);source_root=add(obj('selection-branch',selection_kind='retention_sources',count=str(2*len(sources)),children=[child,child]))
  state=obj('state',version=2,library_id=L,**tree_roots,retention_sources=source_root);state['extensions']=copy.deepcopy(oldstate['extensions'])
  if mode=='old-state-version':state['schema']['version']=1
  roots[role]=dict(root_id=root_id,resource_state=add(state),required_features=sorted(['photara.resource-backings.v1',FEATURE]))
 rep=oldroot['representation'];add(value(rep))
 return dict(records=records,roots=roots,representation=rep,fixture_profile=C['fixture_profile'],resource_count=n)
if __name__=='__main__':
 modes=['valid','wrong-root','unresolved-origin','uncovered-requirement','missing-backing','two-copies','zero-copies','wrong-qualification','unknown-field','old-requirement-version','old-state-version','wrong-tree-kind','unknown-tree-field','capture-policy-pointer','conflicting-source-requirement','wrong-tree-range','duplicate-source-id','changed-source-id','source-alias','reused-source-id']
 out=dict(status='unfrozen-proposed-additive-resource-factoring',production=False,base_sha256=sha(BASE.read_bytes()),scenarios={m:build(8,m) for m in modes},scales={str(n):build(n) for n in [1,16,64]},source_scale=build(16,'many-sources'))
 (HERE/'linked.json').write_text(json.dumps(out,sort_keys=True,indent=2)+'\n');print('factored resource corpus:',len(modes),'scenarios; scales 1,16,64')
