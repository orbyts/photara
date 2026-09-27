#!/usr/bin/env python3
"""Unfrozen fixed phase bytes, independently assembled without the Rust verifier."""
import copy
import hashlib
import json
import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent
OPS_PATH = HERE.parent / 'operations' / 'linked-operations.json'
ops = json.loads(OPS_PATH.read_text())
PROJECT = '10000000-0000-4000-8000-000000000001'
PROFILE = 'example.ps2.synthetic-local-profile-v1'
INCARNATION = '71000000-0000-4000-8000-000000000001'
IDS = {n: f'71000000-0000-4000-8000-{i:012}' for i,n in enumerate(['source','old-meta','data','meta'],10)}
IDS['old-meta']=IDS['meta']

def canonical(v): return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()
def sha(b): return hashlib.sha256(b).hexdigest()
def ref(v):
    b=canonical(v);return {'kind':'json','sha256':sha(b),'byte_length':str(len(b))}
def record(kind,**kwargs): return dict(schema={'id':'example.ps2.phase-'+kind,'version':1},project_id=PROJECT,extensions={},**kwargs)
def storage(suffix,**kwargs): return dict(schema={'id':'example.ps2.'+suffix,'version':1},project_id=PROJECT,extensions={},**kwargs)
def frame(v,tag):
    b=canonical(v);return b'PS2PKD01'+bytes([tag,0,0,0])+struct.pack('<I',len(b))+b
pool={}
def save(name,v,tag=None):
    b=canonical(v);e={'input':v,'canonical':b.decode(),'sha256':sha(b),'byte_length':len(b)}
    if tag is not None:
        f=frame(v,tag);e.update(frame_hex=f.hex(),frame_sha256=sha(f),frame_length=len(f),frame_tag=tag)
    pool[name]=e;return ref(v)
def physical(allocation,arena,offset,f): return dict(allocation_id=allocation,arena=arena,offset=str(offset),byte_length=str(len(f)),record_sha256=sha(f))
objects={r['sha256']:r['input'] for r in ops['records'].values()}
def closure(roots):
    found={}
    def visit(v):
        if isinstance(v,dict):
            if set(v)=={'kind','sha256','byte_length'} and v['kind']=='json':
                if v['sha256'] in found:return
                found[v['sha256']]=v
                visit(objects[v['sha256']])
            else:
                for x in v.values():visit(x)
        elif isinstance(v,list):
            for x in v:visit(x)
    for key in ['authored','history','operation_index']:visit(roots[key])
    return sorted(found.values(),key=lambda r:(r['sha256'],int(r['byte_length'])))
keys={r:closure(ops['selected_roots'][r]) for r in ['active','recovery']}
original_locations={e['object']['sha256']:e['physical'] for e in ops['records']['locator']['input']['entries']}
raw_pack=bytes.fromhex(ops['pack_hex'])
raw_recovery=raw_pack[:ops['recovery_pack_end']]

def witness(allocation):
    index=list(IDS.values()).index(allocation)
    return dict(profile=PROFILE,incarnation=INCARNATION,allocation_id=allocation,device='17',inode=str(100+index))
def charge(b):return ((len(b)+4095)//4096)*4096

# Each builder produces actual semantic, inventory, ownership and locator frames.
# Extent-only self coverage is solved without hashing enclosing final bytes.
def build(name,data_id,meta_id,data_prefix,meta_prefix,roles,mappings,retained,extra_active=False,authenticated_data=None):
    extents={data_id:len(data_prefix),meta_id:len(meta_prefix)}
    for _ in range(20):
        data=bytearray(data_prefix);meta=bytearray(meta_prefix);owned=[];semantics={};states={};inventories={};local={}
        def put(label,v,tag,arena='data'):
            target=data if arena=='data' else meta;alloc=data_id if arena=='data' else meta_id
            f=frame(v,tag);p=physical(alloc,arena,len(target),f);target.extend(f)
            r=ref(v);local[label]=(v,tag);return r,p
        for role in ['active','recovery']:
            root=ops['selected_roots'][roles[role]]
            refs=keys[roles[role]]
            inv=record('inventory-leaf',count=str(len(refs)),entries=refs)
            ir,ip=put(role+'-inventory',inv,1);inventories[role]=ir
            state=storage('state-root',library_id=ops['records']['receipt-1']['input']['library_id'],bootstrap_sha256=ops['records']['manifest']['sha256'],root_id=f"72000000-0000-4000-8000-{int(root['journal_inclusion']['resulting_authored_revision']):012}",authored_revision=root['journal_inclusion']['resulting_authored_revision'],authored=root['authored'],history=root['history'],resource_state=None,operation_index=root['operation_index'],accepted=root['accepted'],journal_inclusion=root['journal_inclusion'],predecessor=({'root_sha256':BASE_STATE['sha256'],'authored_revision':'2'} if root['journal_inclusion']['resulting_authored_revision']=='3' else None),inventory=ir)
            sr,sp=put(role+'-state',state,1);states[role]=sr
            semantic=[]
            for r in refs:
                p=copy.deepcopy(mappings[role][r['sha256']]);semantic.append({'object':r,'membership':'semantic','physical':p})
            semantic.extend([{'object':ir,'membership':'semantic','physical':ip},{'object':sr,'membership':'semantic','physical':sp}])
            semantics[role]=semantic
        claims=[]
        for alloc,entry in sorted(retained.items()):
            claims.append(dict(allocation_id=alloc,arena=entry['arena'],extent=str(len(entry['bytes'])),prefix_end=str(len(entry['bytes'])),prefix_sha256=sha(entry['bytes']),sealed=True,registered_charge=str(entry['charge'])))
        for alloc,arena,prefix in [(data_id,'data',data_prefix if authenticated_data is None else authenticated_data),(meta_id,'metadata',meta_prefix)]:
            claims.append(dict(allocation_id=alloc,arena=arena,extent=str(extents[alloc]),prefix_end=str(len(prefix)),prefix_sha256=sha(prefix),sealed=False,registered_charge=None))
        owner_roots={};locator_roots={}
        for role in ['active','recovery']:
            needed={e['physical']['allocation_id'] for e in semantics[role]}|{data_id,meta_id}
            if extra_active and role=='active':needed.add(IDS['source'])
            children=[];owned=[]
            for i,c in enumerate(sorted([c for c in claims if c['allocation_id'] in needed],key=lambda c:c['allocation_id'])):
                value=record('ownership-leaf',claim=c)
                r,p=put(role+'-ownership-'+str(i),value,2);owned.append({'object':r,'membership':'ownership','physical':p});children.append({'object':r,'physical':p,'allocation_id':c['allocation_id']})
            tree=record('ownership-branch',children=children)
            owner,owner_p=put(role+'-ownership-root',tree,2);owner_roots[role]=owner_p;owned.append({'object':owner,'membership':'ownership','physical':owner_p})
            entries=sorted(semantics[role]+owned,key=lambda e:(e['object']['sha256'],int(e['object']['byte_length'])))
            loc=record('locator-leaf',count=str(len(entries)),entries=entries)
            _,locator_roots[role]=put(role+'-locator',loc,3,'metadata')
        changed={data_id:len(data),meta_id:len(meta)}
        if changed==extents:break
        extents=changed
    else:raise AssertionError('fixed point')
    for label,(value,tag) in local.items():save(name+'-'+label,value,tag)
    tips=[dict(witness=witness(alloc),arena=arena,extent=str(len(b)),registered_charge=str(charge(b))) for alloc,arena,b in [(data_id,'data',data),(meta_id,'metadata',meta)]]
    accounting=record('accounting',profile=PROFILE,incarnation=INCARNATION,standing_control='131072',tips=tips,sealed=[dict(allocation_id=a,registered_charge=str(e['charge'])) for a,e in sorted(retained.items())],tickets=[])
    accounting['total_charge']=str(131072+sum(int(t['registered_charge']) for t in tips)+sum(e['charge'] for e in retained.values()))
    ar=save(name+'-accounting',accounting)
    union={r['sha256']:r for role in ['active','recovery'] for r in keys[roles[role]]+[states[role],inventories[role]]}
    union[ar['sha256']]=ar
    ur=save(name+'-union',record('inventory-leaf',count=str(len(union)),entries=sorted(union.values(),key=lambda r:(r['sha256'],int(r['byte_length'])))))
    root=storage('root-set',library_id=ops['records']['receipt-1']['input']['library_id'],bootstrap_sha256=ops['records']['manifest']['sha256'],kind='scalable-draft',active=states['active'],recovery=states['recovery'],pinned_roots=[],operation_index=ops['selected_roots'][roles['active']]['operation_index'],conversion_source=None,inventory=ur,placement={'generation':'1','accounting':ar,'active_locator':locator_roots['active'],'recovery_locator':locator_roots['recovery'],'active_ownership':owner_roots['active'],'recovery_ownership':owner_roots['recovery']})
    rr=save(name+'-root',root)
    files={a:{'arena':e['arena'],'hex':e['bytes'].hex(),'witness':witness(a)} for a,e in retained.items()}
    files[data_id]={'arena':'data','hex':bytes(data).hex(),'witness':witness(data_id)}
    files[meta_id]={'arena':'metadata','hex':bytes(meta).hex(),'witness':witness(meta_id)}
    return {'root':rr,'files':files,'roles':roles,'expected_closures':{role:keys[roles[role]] for role in ['active','recovery']}},bytes(data),bytes(meta)

def mapping(allocation,offset=0):
    return {h:{**p,'allocation_id':allocation,'offset':str(int(p['offset'])+offset)} for h,p in original_locations.items()}
old,old_data,old_meta=build('old',IDS['source'],IDS['old-meta'],raw_recovery,b'',{'active':'recovery','recovery':'recovery'},{'active':mapping(IDS['source']),'recovery':mapping(IDS['source'])},{})
BASE_STATE=pool['old-root']['input']['active']
retained={IDS['source']:{'arena':'data','bytes':old_data,'charge':charge(old_data)}}
published,new_data,new_meta=build('published',IDS['data'],IDS['meta'],raw_pack,old_meta,{'active':'active','recovery':'recovery'},{'active':mapping(IDS['data']),'recovery':mapping(IDS['source'])},retained)
extra_owner,_,_=build('extra-owner',IDS['data'],IDS['meta'],raw_pack,old_meta,{'active':'active','recovery':'recovery'},{'active':mapping(IDS['data']),'recovery':mapping(IDS['source'])},retained,True)
# Original raw suffix includes a deliberately unreachable typed ownership frame.
dead=record('unreachable-intermediate',allocation_id=IDS['source'],reason='covered-by-suffix-not-selected-closure')
dead_frame=frame(dead,2);save('unreachable',dead,2)
copy_start=len(new_data)
release_prefix=new_data+old_data+dead_frame
released,final_data,final_meta=build('released',IDS['data'],IDS['meta'],release_prefix,new_meta,{'active':'active','recovery':'recovery'},{'active':mapping(IDS['data']),'recovery':mapping(IDS['data'],copy_start)},{},authenticated_data=new_data)
# Tickets retain charge without being physical content-liveness edges.
accounting=pool['released-accounting']['input'];accounting['tips']=copy.deepcopy(pool['published-accounting']['input']['tips']);accounting['tickets']=[{'token':'102','allocation_id':IDS['source'],'registered_charge':str(charge(old_data))}]
accounting['total_charge']=pool['published-accounting']['input']['total_charge']
ar=save('released-accounting',accounting)
union=pool['released-union']['input'];union['entries']=[r for r in union['entries'] if r['sha256']!=pool['released-root']['input']['placement']['accounting']['sha256']]+[ar];union['entries'].sort(key=lambda r:(r['sha256'],int(r['byte_length'])))
ur=save('released-union',union)
r=pool['released-root']['input'];r['placement']['accounting']=ar;r['inventory']=ur;released['root']=save('released-root',r)
# Released source stays physically present and fully charged by the ticket.
released['files'][IDS['source']]=copy.deepcopy(published['files'][IDS['source']])
unlinked=copy.deepcopy(released);del unlinked['files'][IDS['source']]
credited=copy.deepcopy(unlinked)
credit_accounting=copy.deepcopy(accounting);credit_accounting['tickets']=[]
for tip in credit_accounting['tips']:
    b=final_data if tip['arena']=='data' else final_meta
    tip['extent']=str(len(b));tip['registered_charge']=str(charge(b))
credit_accounting['total_charge']=str(131072+sum(int(t['registered_charge']) for t in credit_accounting['tips'])+sum(int(t['registered_charge']) for t in credit_accounting['sealed']))
car=save('credited-accounting',credit_accounting)
cunion=copy.deepcopy(union);cunion['entries']=[x for x in cunion['entries'] if x!=ar]+[car];cunion['entries'].sort(key=lambda r:(r['sha256'],int(r['byte_length'])))
cur=save('credited-union',cunion)
croot=copy.deepcopy(r);croot['placement']['accounting']=car;croot['inventory']=cur;credited['root']=save('credited-root',croot)
# Unchanged manifest/HEAD/outer commit are the only selected bootstrap path.
bootstrap_cache={}
previous_commit=None
for number,(name,snapshot) in enumerate([('old',old),('published',published),('released',released),('unlinked',unlinked),('credited',credited),('extra-owner',extra_owner)],1):
    root_value=next(e['input'] for e in pool.values() if e['sha256']==snapshot['root']['sha256'])
    if snapshot['root']['sha256'] not in bootstrap_cache:
        number=len(bootstrap_cache)+1
        state=next(e['input'] for e in pool.values() if ref(e['input'])==root_value['active'])
        commit={'schema':{'id':'photara.package.commit','version':1},'project_id':PROJECT,'commit_id':f'73000000-0000-4000-8000-{number:012}','package_revision':str(number),'parent':({'commit_id':previous_commit['commit_id'],'sha256':sha(canonical(previous_commit))} if previous_commit else None),'bootstrap_sha256':ops['records']['manifest']['sha256'],'write_id':f'74000000-0000-4000-8000-{number:012}','created_at':'2026-09-27T00:00:00.000Z','minimum_reader':{'major':1,'minor':3},'required_features':sorted(ops['records']['manifest']['input']['required_features']+['example.ps2.scalable-storage.draft-v1','photara.sealed-roots.v1']),'authored':state['authored'],'history':state['history'],'inventory':state['inventory'],'root_set':root_value,'extensions':{}}
        previous_commit=commit
        head={'schema':{'id':'photara.package.head','version':1},'project_id':PROJECT,'commit_id':commit['commit_id'],'commit_sha256':sha(canonical(commit))}
        save(name+'-commit',commit);save(name+'-head',head)
        bootstrap_cache[snapshot['root']['sha256']]={'manifest':ops['records']['manifest']['canonical'],'commit':canonical(commit).decode(),'head':canonical(head).decode()}
    snapshot['bootstrap']=bootstrap_cache[snapshot['root']['sha256']]
# Proposed local-only bindings and immutable codec dispatch are outside intents.
scope=save('scope',record('local-scope',profile=PROFILE,incarnation=INCARNATION,qualification=False,codec='example.ps2.phase-canonical-v1',project_binding_sha256=ops['records']['manifest']['sha256']))

def manifest(alloc,arena,original,final):
    suffix=final[len(original):]
    frames=0;cursor=0
    while cursor<len(suffix):cursor+=16+struct.unpack('<I',suffix[cursor+12:cursor+16])[0];frames+=1
    assert cursor==len(suffix)
    return dict(witness=witness(alloc),arena=arena,original_end=str(len(original)),original_sha256=sha(original),final_end=str(len(final)),frame_count=str(frames),framed_bytes=str(len(suffix)),framed_sha256=sha(suffix))
compact=record('compact-plan',mode='compact',full_data=None,full_metadata=None,data=manifest(IDS['data'],'data',new_data,final_data),metadata=manifest(IDS['meta'],'metadata',new_meta,final_meta),target=released['root'])
cr=save('compact',compact)
initial_charge=131072+charge(old_data)+charge(old_meta)
consumed=charge(new_data)+charge(new_meta)-charge(old_meta)
original=record('original-admission',token='101',scope=scope,request=ref(ops['records']['intent-3']['input']),receipt=ref(ops['records']['receipt-3']['input']),original_selection=old['root'],target_selection=published['root'],original_head_sha256=sha(old['bootstrap']['head'].encode()),target_head_sha256=sha(published['bootstrap']['head'].encode()),reserve='1048576',cleanup_bound='65536',maximum_control_frame='16384',maximum_simultaneous_controls='4',standing_control='131072',original_charge=str(initial_charge),source_witnesses=[witness(IDS['source']),witness(IDS['old-meta'])],payload_sha256=sha(raw_pack),births=[{'allocation_id':IDS['data'],'arena':'data','nonce':sha(b'birth-data'),'maximum_end':'262144'}],existing_tip={'witness':witness(IDS['meta']),'arena':'metadata','original_end':str(len(old_meta)),'original_sha256':sha(old_meta),'maximum_end':'262144'},codec='example.ps2.phase-canonical-v1')
orig=save('admission',original)
marker=save('birth-marker',record('birth-marker',token='101',scope=scope,allocation_id=IDS['data'],nonce=original['births'][0]['nonce']))
birth=save('birth-proof',record('birth-proof',original=orig,marker=marker,witness=witness(IDS['data']),events=['exclusive-marker-create','marker-barrier','binding-selected','truncate-bound-stage','link-no-replace','directory-barrier','unlink-known-stage','directory-barrier'],promotion_pair={'stage':{'witness':witness(IDS['data']),'links':'2'},'final':{'witness':witness(IDS['data']),'links':'2'}},final_links='1',stage_absent=True,namespace_observation='synthetic-profile-only'))
retire_original=record('retirement-admission',token='102',scope=scope,original_selection=published['root'],target_selection=released['root'],original_head_sha256=sha(published['bootstrap']['head'].encode()),target_head_sha256=sha(released['bootstrap']['head'].encode()),source={'witness':witness(IDS['source']),'extent':str(len(old_data)),'sha256':sha(old_data),'registered_charge':str(charge(old_data))},reserve='524288',cleanup_bound='65536',maximum_control_frame='16384',maximum_simultaneous_controls='4',standing_control='131072',compact=cr,codec='example.ps2.phase-canonical-v1')
ret=save('retirement-admission',retire_original)
phases=[]
def phase(name,kind,original_ref,selection,consumed='0',held='1048576',**extra):
    head_sha=next(sha(s['bootstrap']['head'].encode()) for s in [old,published,released,credited] if s['root']==selection)
    v=record('selected-phase',previous=phases[-1] if phases else None,phase=kind,original=original_ref,selection=selection,head_sha256=head_sha,consumed=consumed,held=held,**extra)
    phases.append(save(name,v));return v
phase('admitted','admitted',orig,old['root'],births=[])
phase('witnessed','witnessed',orig,old['root'],births=[witness(IDS['data'])],birth_proof=birth)
phase('payload','payload',orig,old['root'],births=[witness(IDS['data'])],payload_sha256=sha(raw_pack),barrier=True)
phase('finalized','finalized',orig,old['root'],births=[witness(IDS['data'])],payload_sha256=sha(raw_pack),barrier=True,observed_charge=str(consumed),finalizer_selection=published['root'])
phase('published-phase','published',orig,published['root'],str(consumed),str(1048576-consumed),cleanup_pending=True)
phase('clean','clean',orig,published['root'],str(consumed),'0',cleanup_barrier=True)
phase('retirement-held','retirement-held',ret,published['root'],held='524288',source_charge_retained=True)
phase('released-phase','released',ret,released['root'],held='524288',source_charge_retained=True)
phase('unlink-authorized','unlink-authorized',ret,released['root'],held='524288',source_charge_retained=True,authorization_sha256=sha(canonical(retire_original)))
phase('unlinked','unlinked',ret,released['root'],held='524288',source_charge_retained=True,authorization_sha256=sha(canonical(retire_original)),source_absent=True,directory_barrier=False)
phase('absence','absence',ret,released['root'],held='524288',source_charge_retained=True,authorization_sha256=sha(canonical(retire_original)),source_absent=True,directory_barrier=True)
growth=(charge(final_data)-charge(new_data))+(charge(final_meta)-charge(new_meta))
phase('credited','credited',ret,credited['root'],str(growth),'0',source_charge_retained=False,authorization_sha256=sha(canonical(retire_original)),source_absent=True,directory_barrier=True,project_credit=str(charge(old_data)),filesystem_credit='0')
# Worst-width control is sizing-only, never an observed charge or selected state.
worst=copy.deepcopy(pool['finalized']['input'])
worst.update(consumed=str(2**64-1),held=str(2**64-1),observed_charge=str(2**64-1))
for w in worst['births']:w['device']=str(2**64-1);w['inode']=str(2**64-1)
save('worst-width-sizing-only',worst)
output={'status':'unfrozen-linked-phase-candidate','operation_corpus_sha256':sha(OPS_PATH.read_bytes()),'scope':scope,'records':pool,'snapshots':{'old':old,'published':published,'released':released,'unlinked':unlinked,'credited':credited,'extra-owner':extra_owner},'phase_order':phases,'original_data_prefix_hex':new_data.hex(),'original_metadata_prefix_hex':new_meta.hex(),'complete_data_hex':final_data.hex(),'complete_metadata_hex':final_meta.hex(),'original_source_hex':old_data.hex(),'unreachable_frame_offset':len(new_data)+len(old_data),'expected_initial_charge':str(initial_charge),'expected_consumed':str(consumed),'expected_retirement_growth':str(growth),'expected_project_credit':str(charge(old_data)),'qualification':False}
(HERE/'linked-phases.json').write_text(json.dumps(output,indent=2,ensure_ascii=False)+'\n')
print('phase bytes',len(canonical(output)),'worst control',pool['worst-width-sizing-only']['byte_length'])
