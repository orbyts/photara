#!/usr/bin/env python3
"""Unfrozen scaling candidate: independent canonical bytes, no production mutation."""
import json,hashlib,struct
from pathlib import Path
HERE=Path(__file__).resolve().parent
SOURCE=HERE.parent/'joined'/'linked-joined.json'
j=json.loads(SOURCE.read_text());PROJECT='10000000-0000-4000-8000-000000000001'
PROFILE='example.ps2.synthetic-local-profile-v1';INC='71000000-0000-4000-8000-000000000001'
def canon(v):return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()
def sha(b):return hashlib.sha256(b).hexdigest()
def ref(v):return dict(kind='json',sha256=sha(canon(v)),byte_length=str(len(canon(v))))
def obj(name,**fields):return dict(schema=dict(id=name,version=1),project_id=PROJECT,extensions={},**fields)
records={}
def save(v,tag=2):
 b=canon(v);f=b'PS2PKD01'+bytes([tag,0,0,0])+struct.pack('<I',len(b))+b
 records[sha(b)]=dict(input=v,canonical=b.decode(),sha256=sha(b),byte_length=str(len(b)),frame_hex=f.hex(),frame_sha256=sha(f));return ref(v)
def tree(kind,entries,keyfn,width=2):
 prefix='photara.package.retained-file-charge' if kind=='retained' else 'photara.storage.'+('local-observation' if kind=='observation' else 'charge')
 def make_leaf(es):
  fields=dict(count=str(len(es)),entries=es)
  if kind!='observation':fields['charged_high_water']=str(sum(int(e['registered_charge'] if kind=='retained' else e['charged_high_water']) for e in es))
  v=obj(prefix+'-leaf',**fields);r=save(v)
  return dict(first=keyfn(es[0]),last=keyfn(es[-1]),count=v['count'],child=r,**({'charged_high_water':v['charged_high_water']} if kind!='observation' else {}))
 level=[make_leaf(entries[i:i+width]) for i in range(0,len(entries),width)]
 while len(level)>1:
  nxt=[]
  for i in range(0,len(level),2):
   group=level[i:i+2]
   if len(group)==1:nxt+=group;continue
   fields=dict(count=str(sum(int(c['count']) for c in group)),children=group)
   if kind!='observation':fields['charged_high_water']=str(sum(int(c['charged_high_water']) for c in group))
   v=obj(prefix+'-branch',**fields);r=save(v)
   nxt.append(dict(first=group[0]['first'],last=group[-1]['last'],count=v['count'],child=r,**({'charged_high_water':v['charged_high_water']} if kind!='observation' else {})))
  level=nxt
 return level[0]['child']
oldrecords=[v['input'] for v in j['records'].values()]
conversion=next(v for v in oldrecords if v.get('schema',{}).get('id')=='photara.package.conversion-source');conversion_ref=save(conversion,1)
observations=[];charges=[];retained=[]
for old in sorted([v for v in oldrecords if v.get('schema',{}).get('id')=='example.ps2.sealed-charge'],key=lambda v:v['allocation_id']):
 aid=old['allocation_id'];file=j['allocations'][aid];w=file['witness']
 observation=obj('photara.storage.local-observation',observation_id=old['observation']['observation_id'],profile=PROFILE,incarnation=INC,subject=dict(kind='pack',allocation_id=aid,arena=old['arena']),physical=dict(device=w['device'],inode=w['inode']),measured_extent=old['measured_extent'],charged_high_water=old['charged_high_water'])
 obs=save(observation);observations.append(dict(observation_id=observation['observation_id'],observation=obs))
 charge=obj('photara.storage.sealed-charge',**{k:old[k] for k in ['allocation_id','arena','domain_incarnation','measured_extent','charged_high_water']},observation=obs)
 charges.append(dict(allocation_id=aid,charge=save(charge),charged_high_water=charge['charged_high_water']))
for i,f in enumerate(conversion['files']):
 path=f['components'];name='/'.join(path);w=j['source_witnesses'][name];raw=bytes.fromhex(j['source_files'][name]);assert len(raw)==int(f['byte_length']) and sha(raw)==f['sha256']
 charge=((len(raw)+4095)//4096)*4096
 observation=obj('photara.storage.local-observation',observation_id=f'90000000-0000-4000-8000-{i+1:012}',profile=PROFILE,incarnation=INC,subject=dict(kind='retained-file',conversion_id=conversion['conversion_id'],namespace=conversion['snapshot_directory'],path=path),physical=dict(device=w['device'],inode=w['inode']),measured_extent=str(len(raw)),charged_high_water=str(charge))
 obs=save(observation);observations.append(dict(observation_id=observation['observation_id'],observation=obs))
 retained.append(dict(key=dict(conversion_id=conversion['conversion_id'],path=path),conversion_source=conversion_ref,source_file=dict(byte_length=f['byte_length'],sha256=f['sha256']),registered_charge=str(charge),observation=obs))
observations.sort(key=lambda e:e['observation_id']);retained.sort(key=lambda e:(e['key']['conversion_id'],[c.encode() for c in e['key']['path']]))
obsroot=tree('observation',observations,lambda e:e['observation_id'])
retroot=tree('retained',retained,lambda e:e['key'])
chargeroot=tree('charge',charges,lambda e:e['allocation_id'],1)
ledger=obj('example.ps2.accounting-scaling-ledger',profile=PROFILE,incarnation=INC,observation_root=obsroot,retained_file_charge_root=retroot,sealed_charge_root=chargeroot,conversion_source=conversion_ref,registered_charge=str(sum(int(e['registered_charge']) for e in retained)+sum(int(e['charged_high_water']) for e in charges)))
ledger_ref=save(ledger)
source=charges[0];path=[];current=chargeroot
while True:
 v=records[current['sha256']]['input'];path.append(records[current['sha256']])
 if 'entries' in v:break
 current=next(c['child'] for c in v['children'] if c['first']<=source['allocation_id']<=c['last'])
charge=records[source['charge']['sha256']];observation=records[charge['input']['observation']['sha256']]
proof=dict(root=chargeroot,allocation_id=source['allocation_id'],nodes=path,charge=charge,observation=observation)
output=dict(status='unfrozen-accounting-scaling-candidate',qualification=False,joined_sha256=sha(SOURCE.read_bytes()),records=records,ledger=ledger_ref,source_files=j['source_files'],source_witnesses=j['source_witnesses'],sealed_files={e['allocation_id']:j['allocations'][e['allocation_id']] for e in charges},sparse=proof,expected=dict(observations=len(observations),retained_files=len(retained),sealed_charges=len(charges),sparse_nodes=len(path),registered_charge=ledger['registered_charge']))
(HERE/'linked-accounting.json').write_text(json.dumps(output,sort_keys=True,indent=2)+'\n')
print(output['expected'])
