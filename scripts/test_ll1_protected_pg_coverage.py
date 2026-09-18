#!/usr/bin/env python3
"""Disposable full-schema row coverage for the protected PostgreSQL executor."""
import hashlib
import json
import sys
import tempfile

sys.dont_write_bytecode = True
import test_ll1_protected_authorization as authority
from test_ll1_protected_postgres_executor import base, eligible_seed, overlay


def dense_seed(pg):
    """Explicit valid rows, with all normal triggers and FKs enabled."""
    u = lambda n: "'" + base.uid(n) + "'"
    lib, person, project, root = map(u, (1, 410, 510, 810))
    digest = "sha256('{}'::bytea)"
    statements = []
    def insert(table, columns, values):
        statements.append(f'INSERT INTO {table} ({columns}) VALUES ({values});')
    common = 'library_id,record_schema,revision,created_at,updated_at,state,retired_at'
    values = f"{lib},1,1,now(),now(),'tombstoned',now()"
    insert('photara.organizations', 'organization_id,display_name,sort_key,'+common, f"{u(420)},'Org','org',"+values)
    insert('photara.social_profiles', 'social_profile_id,person_id,provider_id,handle,account_kind,verification_kind,provenance,fetch_state,avatar_policy,'+common, f"{u(421)},{person},'fixture','handle','unknown','unverified','{{}}','never','none',"+values)
    insert('photara.person_capabilities','library_id,person_id,capability_id',f"{lib},{person},'fixture'")
    insert('photara.person_labels','library_id,person_id,label_key,display_label',f"{lib},{person},'label','Label'")
    insert('photara.organization_labels','library_id,organization_id,label_key,display_label',f"{lib},{u(420)},'label','Label'")
    insert('photara.person_organization_relationships','relationship_id,person_id,organization_id,relationship_type,'+common,f"{u(422)},{person},{u(420)},'fixture',"+values)
    insert('photara.location_kinds','location_kind_id,canonical_key,canonical_display,retirement_terms,'+common,f"{u(430)},'place','Place','[]',"+values)
    insert('photara.location_kind_terms','library_id,term_key,location_kind_id,policy_version,spellings',f"{lib},'place',{u(430)},1,'[\"Place\"]'")
    insert('photara.locations','location_id,location_kind_id,display_name,sort_key,'+common,f"{u(431)},{u(430)},'Place','place',"+values)
    insert('photara.package_observations','observation_id,library_id,project_id,locator_id,commit_id,commit_sha256,package_revision,index_schema,title,project_lifecycle,asset_count,graph_count,reported_by_account_id,reported_by_device_id,observed_at,received_at,projection_canonical,projection_sha256',f"{u(520)},{lib},{project},{u(511)},{u(521)},{digest},1,1,'Archived','archived',0,0,{u(10)},{u(210)},now(),now(),'{{}}'::bytea,{digest}")
    insert('photara.project_ownership','project_id,library_id,registration_state,source_format,record_schema,revision,created_at,updated_at',f"{project},{lib},'closed','fixture',1,1,now(),now()")
    insert('photara.project_access_policies','library_id,project_id,visibility_policy,owner_mask,admin_mask,editor_mask,viewer_mask,authorization_generation,record_schema,revision,created_at,updated_at',f"{lib},{project},'restricted',0,0,0,0,1,1,1,now(),now()")
    insert('photara_identity.project_access_grants','grant_id,library_id,project_id,account_id,action_mask,state,revoked_at,record_schema,revision,created_at,updated_at',f"{u(530)},{lib},{project},{u(10)},1,'revoked',now(),1,1,now(),now()")
    insert('photara_identity.project_invitations','invitation_id,library_id,project_id,inviter_account_id,target_account_id,action_mask,expected_policy_revision,expires_at,state,completed_at,record_schema,revision,created_at,updated_at',f"{u(531)},{lib},{project},{u(10)},{u(30)},1,1,now()+interval '1 day','revoked',now(),1,1,now(),now()")
    insert('photara_private.project_invitation_secrets','invitation_id,token_verifier,verifier_version,consumed_at',f"{u(531)},{digest},1,now()")
    insert('photara.storage_location_specs','library_id,storage_root_id,storage_kind,supported_rights_json,record_schema,revision,created_at,updated_at',f"{lib},{root},'filesystem','[]',1,1,now(),now()")
    insert('photara.library_variable_values','library_id,variable_id,value_id,is_present,origin,updated_at',f"{lib},{u(910)},{u(920)},false,'manual',now()")
    insert('photara.library_expressions','expression_id,library_id,owner_variable_id,language,container_mode,compiler_version,source_utf8,source_sha256,ast_canonical,ast_sha256,result_type_id,result_type_version,created_at',f"{u(921)},{lib},{u(910)},'photara.expression.v1','expression','1','{{}}'::bytea,{digest},'{{}}'::bytea,{digest},'fixture',1,now()")
    insert('photara.library_expression_dependencies','library_id,expression_id,ordinal,dependency_kind,slot_id,expected_type_id,expected_type_version',f"{lib},{u(921)},0,'storage-slot',{u(811)},'fixture',1")
    insert('photara.library_media','library_id,sha256,media_type,byte_length,created_at',f"{lib},{digest},'image/png',0,now()")
    insert('photara.project_media_links','project_id,sha256,purpose,source_commit_id,source_commit_sha256,consent_projection_sha256,'+common,f"{project},{digest},'cover',{u(521)},{digest},{digest},"+values)
    insert('photara_private.media_objects','library_id,sha256,object_key,state,created_at',f"{lib},{digest},'fixture','quarantined',now()")
    insert('photara_private.media_upload_sessions','upload_id,library_id,sha256,staging_object_key,state,revision,created_at,updated_at,expires_at',f"{u(540)},{lib},{digest},'staging-fixture','expired',1,now(),now(),now()+interval '1 day'")
    insert('photara_private.library_streams','library_id,epoch,last_sequence',f"{lib},{u(720)},0")
    statements.append(f'UPDATE photara_private.library_streams SET last_sequence=1 WHERE library_id={lib};')
    insert('photara_private.mutation_receipts','library_id,mutation_id,actor_account_id,device_id,command_schema,command_kind,request_canonical,request_sha256,outcome,response_canonical,response_sha256,accepted_epoch,accepted_sequence,received_at,completed_at',f"{lib},{u(721)},{u(10)},{u(210)},1,'fixture','{{}}'::bytea,{digest},'accepted','{{}}'::bytea,{digest},{u(720)},1,now(),now()")
    insert('photara.library_change_batches','library_id,epoch,sequence,mutation_id,committed_at,change_count,batch_canonical,batch_sha256',f"{lib},{u(720)},1,{u(721)},now(),1,'{{}}'::bytea,{digest}")
    insert('photara.library_changes','library_id,epoch,sequence,ordinal,entity_kind,entity_id,entity_revision,change_kind,record_schema,post_state,post_state_canonical,post_state_sha256',f"{lib},{u(720)},1,0,'person',{person},1,'tombstone',1,'{{}}','{{}}'::bytea,{digest}")
    insert('photara_private.sync_clients','library_id,account_id,device_id,stream_epoch,acknowledged_sequence,last_seen_at',f"{lib},{u(10)},{u(210)},{u(720)},1,now()")
    insert('photara_private.scoped_sync_clients','stream_id,account_id,device_id,authorization_generation,epoch,acknowledged_sequence,last_seen_at',f"{u(710)},{u(10)},{u(210)},1,{u(711)},1,now()")
    insert('photara_private.library_claim_receipts','account_id,claim_id,requested_library_id,claimed_library_id,request_canonical,request_sha256,outcome,response_canonical,response_sha256,completed_at',f"{u(10)},{u(730)},{lib},{lib},'{{}}'::bytea,{digest},'claimed','{{}}'::bytea,{digest},now()")
    insert('photara_private.security_audit','audit_id,actor_account_id,library_id,action_code,target_kind,occurred_at',f"{u(740)},{u(10)},{lib},'fixture','library',now()")
    insert('photara_private.library_entitlement_grants','grant_id,library_id,capability_key,source,subscription_id,enabled,valid_from,state,revision,created_at,updated_at',f"{u(750)},{lib},'fixture','subscription',{u(610)},false,now(),'revoked',1,now(),now()")
    base.require(pg,'BEGIN; SET LOCAL ROLE photara_owner;'+base.ctx()+''.join(statements)+'COMMIT;')
    base.require(pg,f"""BEGIN; SET LOCAL ROLE photara_owner; {base.ctx(2,20)}
INSERT INTO photara.location_kinds(location_kind_id,library_id,canonical_key,canonical_display,record_schema,revision,created_at,updated_at,state)
VALUES({u(1430)},{u(2)},'place','Other place',1,1,now(),now(),'active');
INSERT INTO photara.location_kind_terms VALUES({u(2)},'place',{u(1430)},1,'["Place"]'); COMMIT;""")


def target_rows(snapshot, owned):
    """Independent expected closure; never infer indirect ownership after deletion."""
    invitations = {r['invitation_id'] for r in snapshot['photara_identity.project_invitations'] if r['library_id']==base.uid(1)}
    streams = {(r['stream_id'],r['epoch']) for r in snapshot['photara_private.scoped_streams'] if r['library_id']==base.uid(1)}
    return {table:[r for r in snapshot[table] if r.get('library_id')==base.uid(1)
                  or (table.endswith('project_invitation_secrets') and r['invitation_id'] in invitations)
                  or (table.endswith('scoped_sync_clients') and (r['stream_id'],r['epoch']) in streams)
                  or (table.endswith('library_claim_receipts') and r['claimed_library_id']==base.uid(1))]
            for table in owned}


def main():
    if not __debug__:
        raise RuntimeError('Assertions required')
    with tempfile.TemporaryDirectory(prefix='photara-ll1-constraints-',dir='/private/tmp') as root:
        pg=base.Postgres(root)
        try:
            hashes=base.install(pg)
            measured=base.measure(pg)
            eligible_seed(pg)
            dense_seed(pg)
            owned=overlay(pg)
            authority.boundary(pg)
            before=json.loads(base.snapshot(pg))
            closure=target_rows(before,owned)
            gaps=[t for t,rows in closure.items() if not rows]
            assert not gaps,gaps
            args=f"'{base.uid(110)}','{base.uid(10)}','{base.uid(1)}',1,'{base.uid(9600)}',sha256('coverage'::bytea)"
            call=f'BEGIN; SELECT ll1_probe.authorize({args}); SELECT ll1_probe.execute({args});'
            cases=[]
            def check(name,sql,error=None,preserve=True,login='ll1_service'):
                base.require(pg,sql,login=login,error=error)
                if preserve:
                    assert json.loads(base.snapshot(pg))==before,name
                cases.append(name)
                print(json.dumps({'case':name,'result':'pass'}),flush=True)
            check('existing_overlay_missing_term_guard',call+'ROLLBACK;',error='term_reserved')
            definition=pg.scalar("SELECT pg_get_functiondef('photara_private.guard_kind_term()'::regprocedure)")
            insertion="\n IF TG_OP='DELETE' AND ll1_probe.permitted(TG_TABLE_SCHEMA||'.'||TG_TABLE_NAME,to_jsonb(OLD)) THEN RETURN OLD; END IF;\n"
            base.require(pg,'SET ROLE photara_owner;'+definition.replace('BEGIN','BEGIN'+insertion,1))
            guard_tx='BEGIN; SET LOCAL ROLE photara_owner;'+base.ctx()
            permit="INSERT INTO ll1_probe.permit SELECT txid_current(),'photara.location_kind_terms',to_jsonb(t) FROM photara.location_kind_terms t WHERE library_id='"+base.uid(1)+"';"
            delete="DELETE FROM photara.location_kind_terms WHERE library_id='"+base.uid(1)+"';"
            check('ordinary_term_delete_denied',guard_tx+delete,error='term_reserved',login='ll1_fixture')
            check('ordinary_term_identity_update_denied',guard_tx+permit+"UPDATE photara.location_kind_terms SET term_key='changed' WHERE library_id='"+base.uid(1)+"';",error='claim_identity_is_immutable',login='ll1_fixture')
            check('term_stale_transaction_permit_denied',guard_tx+permit.replace('txid_current()','txid_current()-1')+delete,error='term_reserved',login='ll1_fixture')
            check('term_wrong_relation_permit_denied',guard_tx+permit.replace("'photara.location_kind_terms'","'photara.people'")+delete,error='term_reserved',login='ll1_fixture')
            check('term_wrong_row_permit_denied',guard_tx+permit.replace('to_jsonb(t)',"to_jsonb(t)||'{\"term_key\":\"other\"}'::jsonb")+delete,error='term_reserved',login='ll1_fixture')
            check('term_other_library_permit_denied',guard_tx+permit+base.ctx(2,20)+delete.replace(base.uid(1),base.uid(2)),error='term_reserved',login='ll1_fixture')
            check('term_exact_delete_permit_rollback',guard_tx+permit+delete+'ROLLBACK;',login='ll1_fixture')
            check('all_48_tables_delete_rollback',call+'ROLLBACK;')
            base.require(pg,"SET ROLE photara_owner; CREATE FUNCTION ll1_probe.omit_terminal_coverage() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$; CREATE TRIGGER omit_terminal_coverage BEFORE INSERT ON ll1_probe.inventory FOR EACH ROW EXECUTE FUNCTION ll1_probe.omit_terminal_coverage();")
            check('full_closure_terminal_failure_rollback',call+'COMMIT;',error='terminal_closure')
            assert pg.scalar('SELECT count(*) FROM ll1_probe.permit')=='0'
            base.require(pg,'SET ROLE photara_owner; DROP TRIGGER omit_terminal_coverage ON ll1_probe.inventory; DROP FUNCTION ll1_probe.omit_terminal_coverage();')
            for table in ('photara.storage_slot_names','photara.library_variable_names','photara_private.scoped_command_receipts','photara_private.project_invitation_secrets','photara_private.scoped_sync_clients','photara_private.library_claim_receipts'):
                base.require(pg,f"SET ROLE photara_owner; CREATE FUNCTION ll1_probe.omit_coverage() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.relation='{table}' THEN RETURN NULL; END IF; RETURN NEW; END $$; CREATE TRIGGER omit_coverage BEFORE INSERT ON ll1_probe.permit FOR EACH ROW EXECUTE FUNCTION ll1_probe.omit_coverage();")
                check('omitted_'+table,call+'COMMIT;',error='foreign key')
                base.require(pg,'SET ROLE photara_owner; DROP TRIGGER omit_coverage ON ll1_probe.permit; DROP FUNCTION ll1_probe.omit_coverage();')
            check('forged_owner_context_denied','BEGIN;'+base.ctx()+f'SELECT ll1_probe.execute({args});',error='permission denied',login='ll1_api')
            check('all_48_tables_authorized_commit',call+'COMMIT;',preserve=False)
            after=json.loads(base.snapshot(pg))
            for table,rows in before.items():
                assert after[table]==[r for r in rows if r not in closure.get(table,[])],table
            assert not any(target_rows(after,owned).values())
            assert pg.scalar('SELECT count(*) FROM ll1_probe.permit')=='0'
            assert pg.scalar("SELECT count(*) FROM ll1_probe.receipts WHERE result='removed'")=='1'
            assert pg.scalar('SELECT count(*) FROM ll1_probe.markers')=='1'
            assert pg.scalar('SELECT count(*) FROM ll1_probe.inventory')=='1'
            base.require(pg,call+'COMMIT;',login='ll1_service',value='removed')
            assert json.loads(base.snapshot(pg))==after
            for name,digest in hashes.items():
                assert hashlib.sha256((base.ROOT/'crates/photara-service/migrations/postgres'/name).read_bytes()).hexdigest()==digest
            print(json.dumps({'cases':len(cases)+1,'catalog':measured,'seeded_tables':len(closure),'target_rows':sum(map(len,closure.values())),'unseeded_tables':gaps,'coverage':{t:len(rows) for t,rows in closure.items()},'status':'full_table_row_closure_proven'}))
        finally:
            pg.close()


if __name__=='__main__':
    main()
