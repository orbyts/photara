#!/usr/bin/env python3
"""Unfrozen whole-Blob storage choice; existing portable v1.1 bytes unchanged."""
import hashlib,json
from pathlib import Path
HERE=Path(__file__).resolve().parent
SOURCE=HERE.parents[3]/'fixtures/generation-two/d19-package-specimen.json'
def canonical(v):return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False,allow_nan=False).encode()
def sha(b):return hashlib.sha256(b).hexdigest()
def ref(v):return dict(kind='json',sha256=sha(canonical(v)),byte_length=str(len(canonical(v))))
def blobref(b):return dict(kind='blob',sha256=sha(b),byte_length=str(len(b)))
archive=json.loads(SOURCE.read_text());files={f['path']:f['utf8'].encode() for f in archive['files']}
manifest=json.loads(files['manifest.json']);PROJECT=manifest['project_id']
values={p:json.loads(raw) for p,raw in files.items() if p.startswith('objects/json/')}
managed=next(v for v in values.values() if v.get('schema',{}).get('id')=='photara.project.managed-resource')
representation=next(v for v in values.values() if v.get('schema',{}).get('id')=='photara.project.representation-content' and v['binding']['kind']=='managed')
raw=files['objects/blobs/sha256/'+managed['blob']['sha256']]
assert managed['blob']==blobref(raw) and representation['binding']['version']==ref(managed)
PROFILE='example.ps2.synthetic-local-profile-v1';INCARNATION='95000000-0000-4000-8000-000000000001'
def obj(name,**fields):return dict(schema=dict(id=name,version=1),project_id=PROJECT,extensions={},**fields)
def uid(n):return f'95000000-0000-4000-8000-{n:012d}'
records={}
def save(v):
 b=canonical(v);records[sha(b)]=dict(input=v,canonical=b.decode(),sha256=sha(b),byte_length=len(b));return ref(v)
managedref=save(managed);representationref=save(representation)
allocations={};entries=[];claims=[];charges=[];observations=[]
for i,data in enumerate([raw,b'']):
 allocation=uid(10+i);observation=obj('photara.storage.local-observation',observation_id=uid(20+i),profile=PROFILE,incarnation=INCARNATION,subject=dict(kind='whole-blob',allocation_id=allocation,arena='data'),physical=dict(device='7',inode=str(900+i)),measured_extent=str(len(data)),charged_high_water=str((len(data)+4095)//4096*4096));observationref=save(observation)
 charge=obj('photara.storage.sealed-charge',allocation_id=allocation,arena='data',domain_incarnation=INCARNATION,measured_extent=str(len(data)),charged_high_water=observation['charged_high_water'],observation=observationref);chargeref=save(charge)
 claim=obj('photara.storage.allocation-claim',allocation_id=allocation,arena='data',layout='whole-blob',owned_extent=str(len(data)),authenticated_prefix=dict(byte_length=str(len(data)),sha256=sha(data)),sealed=True,sealed_charge=chargeref);claimref=save(claim)
 physical=dict(kind='whole-blob',allocation_id=allocation,byte_length=str(len(data)),sha256=sha(data))
 entries.append(dict(object=blobref(data),membership='blob',physical=physical));claims.append(claimref);charges.append(chargeref);observations.append(observationref)
 allocations[allocation]=dict(layout='whole-blob',hex=data.hex(),observation_id=observation['observation_id'],profile=PROFILE,incarnation=INCARNATION,physical=observation['physical'])
entries.sort(key=lambda e:(0 if e['object']['kind']=='blob' else 1,e['object']['sha256'],int(e['object']['byte_length'])))
locator=save(obj('photara.storage.locator-leaf',count=str(len(entries)),entries=entries))
# This bounded projection does not pretend to be a complete selected RootSet.
selection=obj('example.ps2.whole-blob-selection',managed=managedref,representation=representationref,blobs=[e['object'] for e in entries],locator=locator,claims=claims,charges=charges,observations=observations,scope=dict(profile=PROFILE,incarnation=INCARNATION),required_features=['photara.whole-blob-storage.v1'],namespace_allowance='8192',directory_allowance='4096',standing_control='65536',registered_data_charge='4096',total_charge='81920')
selectionref=save(selection)
recordfiles=sum((r['byte_length']+4095)//4096*4096 for r in records.values());assert recordfiles<=65536
out=dict(status='unfrozen-whole-blob-placement-candidate',qualification=False,original_package_sha256=sha(SOURCE.read_bytes()),original_files={p:b.hex() for p,b in files.items()},records=records,selection=selectionref,allocations=allocations,expected=dict(raw_blob_bytes=[len(raw),0],records=len(records),rounded_control_bytes=recordfiles,total_charge=81920,empty_blob_namespace_allowance=4096),same_digest_kind_order=[dict(kind='blob',sha256=sha(raw),byte_length=str(len(raw))),dict(kind='json',sha256=sha(raw),byte_length=str(len(raw)))])
(HERE/'linked.json').write_text(json.dumps(out,sort_keys=True,indent=2)+'\n');print(out['expected'])
