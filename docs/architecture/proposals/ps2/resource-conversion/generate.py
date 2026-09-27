#!/usr/bin/env python3
"""Independent unfrozen metadata bytes. No package/media IO or conversion."""
import copy
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
BASE = json.loads((HERE.parent / 'operations/linked-operations.json').read_text())['base_files']
GOLD = {v['name']: v['value'] for v in json.loads((HERE.parent / 'sealed-wire-golden.json').read_text())['vectors']}
PROJECT = json.loads(BASE['manifest.json'])['project_id']
LIBRARY = '10000000-0000-4000-8000-000000000002'

def canon(v):
    return json.dumps(v, sort_keys=True, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()
def sha(b):
    return hashlib.sha256(b).hexdigest()
def ident(n):
    return f'71000000-0000-4000-8000-{n:012d}'
def template(name):
    v = copy.deepcopy(GOLD[name]); v['project_id'] = PROJECT
    return v

def build(change=None):
    records = {}
    def put(v):
        b = canon(v); h = sha(b)
        records[h] = dict(input=v, canonical=b.decode(), sha256=h, byte_length=len(b))
        return dict(kind='json', sha256=h, byte_length=str(len(b)))
    rid, vid, bid = ident(1), ident(2), ident(3)
    version_hash = sha(b'synthetic captured media commitment; no media opened')
    profile = dict(profile_id=ident(4), profile_revision='1', failure_model_sha256=sha(b'fixture-only single-copy profile; no actual qualification'))
    identity = template('resource-identity');identity['resource_id'] = rid
    working = template('portable-working-binding');working['resource_id'] = rid;working['binding_id'] = ident(5);working['location']['library_id'] = LIBRARY
    capture = template('publication-evidence');capture.update(evidence_id=ident(6),operation_id=ident(7),resource_id=rid,version_id=vid,sha256=version_hash,byte_length='8000000000',evidence_kind='capture',destination=None,qualification=None)
    version = template('captured-version');version.update(resource_id=rid,version_id=vid,media_type='image/raw',byte_length='8000000000',sha256=version_hash,capture_operation_id=capture['operation_id'])
    if change == 'capture-qualified': capture['qualification'] = profile
    version['capture_evidence'] = put(capture)
    if change == 'version-policy-pointer': version['backing'] = bid
    version_ref = put(version)
    rep = template('representation-v3');rep['binding'] = dict(kind='managed-captured',resource_id=rid,version_id=vid,version=version_ref);rep['media'] = dict(media_type='image/raw',byte_length='8000000000');rep['fingerprint']['value'] = version_hash
    if change == 'binding-version': rep['binding']['version_id'] = ident(999)
    if change == 'binding-length': rep['media']['byte_length'] = '7'
    if change == 'binding-digest': rep['fingerprint']['value'] = '0'*64
    if change == 'binding-v2': rep['schema']['version'] = 2
    rep_ref = put(rep)
    roots = {}
    for role, rev, root_id in [('active','2',ident(20)),('recovery','1',ident(21))]:
        evidence = template('publication-evidence');evidence.update(evidence_id=ident(8+int(rev)),operation_id=ident(12+int(rev)),resource_id=rid,version_id=vid,sha256=version_hash,byte_length='8000000000',qualification=profile)
        location = dict(library_id=LIBRARY,storage_location_id=ident(30+int(rev)),coordinate=dict(kind='filesystem',components=['retained',f'copy-{rev}.raw']))
        evidence['destination'] = dict(backing_id=bid,backing_revision=rev,location=location)
        if role == 'active':
            if change == 'capture-as-publication': evidence.update(evidence_kind='capture',destination=None,qualification=None)
            if change == 'wrong-profile': evidence['qualification'] = dict(profile,profile_id=ident(999))
            if change == 'old-profile': evidence['qualification'] = dict(profile,profile_revision='0')
            if change == 'wrong-failure-model': evidence['qualification'] = dict(profile,failure_model_sha256='0'*64)
            if change == 'wrong-destination': evidence['destination'] = dict(evidence['destination'],backing_revision='99')
        backing = template('retained-backing');backing.update(resource_id=rid,version_id=vid,backing_id=bid,revision=rev,sha256=version_hash,byte_length='8000000000',location=location,publication_evidence=put(evidence))
        if role == 'active' and change == 'backing-version': backing['version_id'] = ident(999)
        obligation = template('retention-obligation');obligation.update(obligation_id=ident(40),resource_id=rid,version_id=vid,origin=dict(kind='authored',source_id=root_id),required_backing_ids=[bid],required_qualification=dict(profile_id=profile['profile_id'],minimum_profile_revision='1',failure_model_sha256=profile['failure_model_sha256']))
        if role == 'active':
            if change == 'missing-pin': obligation['origin']['source_id'] = ident(999)
            if change == 'unknown-pin': obligation['origin']['kind'] = 'lease'
            if change == 'zero-copies': obligation['minimum_qualified_copies'] = '0'
            if change == 'two-copies': obligation['minimum_qualified_copies'] = '2'
            if change == 'missing-required-backing': obligation['required_backing_ids'] = [ident(999)]
        state = template('resource-state');state['library_id'] = LIBRARY
        for field,key,idkey,record in [('identities','identity','resource_id',identity),('working_bindings','binding','binding_id',working),('versions','version','version_id',version),('backings','backing','backing_id',backing),('obligations','obligation','obligation_id',obligation)]:
            state[field] = [{idkey:record[idkey],key:put(record)}]
        if role == 'active':
            if change == 'selection-id': state['versions'][0]['version_id'] = ident(999)
            if change == 'duplicate-selection': state['backings'] *= 2
            if change == 'duplicate-working':
                extra = dict(working,binding_id=ident(998));state['working_bindings'].append(dict(binding_id=extra['binding_id'],binding=put(extra)))
            if change == 'host-observation':
                bad = dict(working,inode='1');state['working_bindings'][0]['binding'] = put(bad)
            if change == 'unsafe-coordinate':
                bad = copy.deepcopy(working);bad['location']['coordinate']['components'] = ['..','secret'];state['working_bindings'][0]['binding'] = put(bad)
        state['extensions'] = {'example.opaque':{'kind':'json','sha256':'0'*64,'byte_length':'999','unknown':['preserve',2]}}
        state_ref = put(state)
        # Exact typed resource projection, deliberately NOT a production StateRoot.
        root = dict(schema={'id':'example.ps2.resource-closure-projection','version':1},project_id=PROJECT,library_id=LIBRARY,root_id=root_id,resource_state=state_ref,representation=rep_ref,features=['photara.resource-backings.v1'],pin_sources=[dict(kind='authored',source_id=root_id)],extensions={})
        if role == 'active' and change == 'missing-feature': root['features'] = []
        refs = [state_ref,rep_ref]
        for field,key in [('identities','identity'),('working_bindings','binding'),('versions','version'),('backings','backing'),('obligations','obligation')]: refs += [x[key] for x in state[field]]
        refs += [version['capture_evidence'],backing['publication_evidence']]
        unique = {(r['sha256'],int(r['byte_length'])):r for r in refs}
        root['inventory'] = [unique[k] for k in sorted(unique)]
        if role == 'active' and change == 'surplus-inventory': root['inventory'].append(dict(kind='json',sha256='f'*64,byte_length='1'))
        if role == 'active' and change == 'missing-inventory': root['inventory'].pop()
        roots[role] = put(root)
    source = {k:v.encode() for k,v in BASE.items()}
    source.update({'optional/opaque.bin':b'\x00\xffunknown optional bytes\n','staging/other-attempt/keep.bin':b'not this attempt','conversion-sources-not-reserved/keep.bin':b'prefix is not exclusion'})
    conversion = template('conversion-source');conversion.update(conversion_id=ident(50),source_format_version=json.loads(BASE['manifest.json'])['format_version'],source_bootstrap_sha256=sha(source['manifest.json']),source_head_sha256=sha(source['HEAD.json']),source_commit_id=json.loads(BASE['HEAD.json'])['commit_id'],snapshot_directory=['conversion-sources',ident(50),'package'])
    conversion['files'] = [dict(components=k.split('/'),sha256=sha(v),byte_length=str(len(v))) for k,v in sorted(source.items())]
    conversion_ref = put(conversion)
    return dict(records=records,roots=roots,conversion=conversion_ref,source_files={k:v.hex() for k,v in source.items()},fixture_profile=profile)

names = ['capture-qualified','version-policy-pointer','binding-version','binding-length','binding-digest','binding-v2','capture-as-publication','wrong-profile','old-profile','wrong-failure-model','wrong-destination','backing-version','missing-pin','unknown-pin','zero-copies','two-copies','missing-required-backing','selection-id','duplicate-selection','duplicate-working','host-observation','unsafe-coordinate','missing-feature','surplus-inventory','missing-inventory']
out = {'status':'unfrozen-linked-resource-conversion-projection','base_file_count':len(BASE),'scenarios':{'valid':build(),**{name:build(name) for name in names}}}
HERE.joinpath('linked.json').write_text(json.dumps(out,indent=2,ensure_ascii=False)+'\n')
print(f'{len(out["scenarios"])} scenarios; {len(BASE)} legacy files; no native package effects')
