-- INERT PROPOSAL ONLY — CXT2, accepted design 2026-09-12.
-- NOT AN INSTALLED, APPLIED, OR RUNNABLE MIGRATION. DO NOT EXECUTE.
-- Review text outside every runtime migration directory; no runner references it.
-- Implementation/adaptation and disposable execution require separate CXT3 scope.
-- Read ../README.md and ../RESPONSIBILITIES.md before reviewing this SQL.
-- Reservations map to D19_STATIC_SCHEMA_DELTA.md; baseline bytes remain intact.

CREATE FUNCTION photara_private.d19_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN RAISE EXCEPTION 'd19_immutable' USING ERRCODE='23514'; END;
$$;

CREATE FUNCTION photara_private.d19_mutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
DECLARE old_j jsonb:=to_jsonb(OLD); new_j jsonb:=to_jsonb(NEW); k text;
BEGIN
  IF TG_OP='DELETE' THEN RAISE EXCEPTION 'd19_no_delete' USING ERRCODE='23514'; END IF;
  FOREACH k IN ARRAY string_to_array(TG_ARGV[0],',') LOOP
    IF old_j->k IS DISTINCT FROM new_j->k THEN
      RAISE EXCEPTION 'd19_immutable_coordinate' USING ERRCODE='23514';
    END IF;
  END LOOP;
  IF TG_ARGV[1]='revision' THEN
    IF NEW.revision<>OLD.revision+1 THEN
      RAISE EXCEPTION 'd19_revision_step' USING ERRCODE='23514';
    END IF;
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_library_contract_state_retention BEFORE UPDATE OR DELETE ON photara.library_contract_state
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('workspace_id,contract_version,record_schema,created_at','revision');

CREATE TRIGGER d19_project_ownership_retention BEFORE UPDATE OR DELETE ON photara.project_ownership
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('project_id,workspace_id,source_origin_library_id,source_format,record_schema,created_at','revision');

CREATE TRIGGER d19_project_access_policies_retention BEFORE UPDATE OR DELETE ON photara.project_access_policies
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('workspace_id,project_id,record_schema,created_at','revision');

CREATE TRIGGER d19_project_access_grants_retention BEFORE UPDATE OR DELETE ON photara_identity.project_access_grants
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('grant_id,workspace_id,project_id,account_id,local_principal_id,record_schema,created_at','revision');

CREATE TRIGGER d19_project_invitations_retention BEFORE UPDATE OR DELETE ON photara_identity.project_invitations
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('invitation_id,workspace_id,project_id,inviter_account_id,target_account_id,action_mask,expected_policy_revision,expires_at,record_schema,created_at','revision');

CREATE TRIGGER d19_project_invitation_secrets_retention BEFORE UPDATE OR DELETE ON photara_private.project_invitation_secrets
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('invitation_id,token_verifier,verifier_version','');

CREATE TRIGGER d19_storage_location_specs_retention BEFORE UPDATE OR DELETE ON photara.storage_location_specs
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('workspace_id,storage_root_id,storage_kind,provider_id,record_schema,created_at','revision');

CREATE TRIGGER d19_storage_slots_retention BEFORE UPDATE OR DELETE ON photara.storage_slots
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('slot_id,workspace_id,record_schema,created_at','revision');

CREATE FUNCTION photara_private.d19_storage_slots_no_resurrection() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF OLD.state='tombstoned' AND NEW.state<>'tombstoned' THEN RAISE EXCEPTION 'd19_no_resurrection' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_storage_slots_no_resurrection BEFORE UPDATE ON photara.storage_slots
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_storage_slots_no_resurrection();

CREATE TRIGGER d19_storage_slot_names_retention BEFORE UPDATE OR DELETE ON photara.storage_slot_names
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_immutable();

CREATE TRIGGER d19_library_variables_retention BEFORE UPDATE OR DELETE ON photara.library_variables
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('variable_id,workspace_id,namespace,value_type_id,value_type_version,schema_id,schema_version,record_schema,created_at','revision');

CREATE FUNCTION photara_private.d19_library_variables_no_resurrection() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF OLD.state='tombstoned' AND NEW.state<>'tombstoned' THEN RAISE EXCEPTION 'd19_no_resurrection' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_library_variables_no_resurrection BEFORE UPDATE ON photara.library_variables
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_library_variables_no_resurrection();

CREATE TRIGGER d19_library_variable_values_retention BEFORE UPDATE OR DELETE ON photara.library_variable_values
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('workspace_id,variable_id,value_id','');

CREATE TRIGGER d19_library_variable_names_retention BEFORE UPDATE OR DELETE ON photara.library_variable_names
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_immutable();

CREATE TRIGGER d19_library_expressions_retention BEFORE UPDATE OR DELETE ON photara.library_expressions
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_immutable();

CREATE TRIGGER d19_library_expression_dependencies_retention BEFORE UPDATE OR DELETE ON photara.library_expression_dependencies
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_immutable();

CREATE TRIGGER d19_project_media_links_retention BEFORE UPDATE OR DELETE ON photara.project_media_links
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('workspace_id,project_id,sha256,purpose,record_schema,created_at','revision');

CREATE FUNCTION photara_private.d19_project_media_links_no_resurrection() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF OLD.state='tombstoned' AND NEW.state<>'tombstoned' THEN RAISE EXCEPTION 'd19_no_resurrection' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_project_media_links_no_resurrection BEFORE UPDATE ON photara.project_media_links
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_project_media_links_no_resurrection();

CREATE TRIGGER d19_scoped_streams_retention BEFORE UPDATE OR DELETE ON photara_private.scoped_streams
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('stream_id,workspace_id,scope_kind,project_id,epoch,created_at','');

CREATE TRIGGER d19_scoped_change_batches_retention BEFORE UPDATE OR DELETE ON photara_private.scoped_change_batches
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_immutable();

CREATE TRIGGER d19_scoped_changes_retention BEFORE UPDATE OR DELETE ON photara_private.scoped_changes
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_immutable();

CREATE TRIGGER d19_scoped_command_receipts_retention BEFORE UPDATE OR DELETE ON photara_private.scoped_command_receipts
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_immutable();

CREATE TRIGGER d19_scoped_sync_clients_retention BEFORE UPDATE OR DELETE ON photara_private.scoped_sync_clients
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_mutable('stream_id,account_id,device_id','');

CREATE FUNCTION photara_private.d19_project_ownership_lifecycle() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF (OLD.registration_state='closed' AND NEW.registration_state<>'closed') OR (OLD.registration_state='active' AND NEW.registration_state='pending') THEN RAISE EXCEPTION 'd19_registration_lifecycle' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_project_ownership_lifecycle BEFORE UPDATE ON photara.project_ownership
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_project_ownership_lifecycle();

CREATE FUNCTION photara_private.d19_project_invitations_terminal() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF OLD.state<>'pending' THEN RAISE EXCEPTION 'd19_invitation_terminal' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_project_invitations_terminal BEFORE UPDATE ON photara_identity.project_invitations
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_project_invitations_terminal();

CREATE FUNCTION photara_private.d19_library_contract_state_generation() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.authorization_generation<OLD.authorization_generation OR NEW.authorization_generation>OLD.authorization_generation+1 THEN RAISE EXCEPTION 'd19_generation_step' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_library_contract_state_generation BEFORE UPDATE ON photara.library_contract_state
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_library_contract_state_generation();

CREATE FUNCTION photara_private.d19_project_access_policies_generation() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.authorization_generation<OLD.authorization_generation OR NEW.authorization_generation>OLD.authorization_generation+1 OR NEW.authorization_generation<>OLD.authorization_generation+1 THEN RAISE EXCEPTION 'd19_generation_step' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_project_access_policies_generation BEFORE UPDATE ON photara.project_access_policies
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_project_access_policies_generation();

CREATE FUNCTION photara_private.d19_project_invitation_secrets_consume() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF OLD.consumed_at IS NOT NULL OR NEW.consumed_at IS NULL THEN RAISE EXCEPTION 'd19_token_consumed' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_project_invitation_secrets_consume BEFORE UPDATE ON photara_private.project_invitation_secrets
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_project_invitation_secrets_consume();

CREATE FUNCTION photara_private.d19_scoped_streams_sequence() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.last_sequence<>OLD.last_sequence+1 THEN RAISE EXCEPTION 'd19_stream_step' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_scoped_streams_sequence BEFORE UPDATE ON photara_private.scoped_streams
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_scoped_streams_sequence();

CREATE FUNCTION photara_private.d19_storage_slots_target_insert() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.state='active' AND NOT EXISTS (SELECT 1 FROM photara.storage_roots r JOIN photara.storage_location_specs s ON s.workspace_id=r.workspace_id AND s.storage_root_id=r.storage_root_id WHERE r.workspace_id=NEW.workspace_id AND r.storage_root_id=NEW.storage_root_id AND r.state='active') THEN RAISE EXCEPTION 'd19_slot_unclassified' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_storage_slots_target_insert BEFORE INSERT ON photara.storage_slots
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_storage_slots_target_insert();

CREATE FUNCTION photara_private.d19_storage_slots_target_update() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.state='active' AND NOT EXISTS (SELECT 1 FROM photara.storage_roots r JOIN photara.storage_location_specs s ON s.workspace_id=r.workspace_id AND s.storage_root_id=r.storage_root_id WHERE r.workspace_id=NEW.workspace_id AND r.storage_root_id=NEW.storage_root_id AND r.state='active') THEN RAISE EXCEPTION 'd19_slot_unclassified' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_storage_slots_target_update BEFORE UPDATE ON photara.storage_slots
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_storage_slots_target_update();

CREATE FUNCTION photara_private.d19_storage_roots_live_dependents() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.state='tombstoned' AND (EXISTS (SELECT 1 FROM photara.storage_slots s WHERE s.workspace_id=NEW.workspace_id AND s.storage_root_id=NEW.storage_root_id AND s.state='active') OR EXISTS (SELECT 1 FROM photara.project_locators l WHERE l.workspace_id=NEW.workspace_id AND l.storage_root_id=NEW.storage_root_id AND l.state='active')) THEN RAISE EXCEPTION 'd19_root_in_use' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_storage_roots_live_dependents BEFORE UPDATE ON photara.storage_roots
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_storage_roots_live_dependents();

-- Trusted transaction GUCs are populated only after JWT/identity validation.
-- These functions are owned by the migration-only photara_owner role; the
-- owner-only policies below allow narrowly scoped authorization-table reads
-- without recursively evaluating content predicates. No runtime owner login.
CREATE FUNCTION photara_private.d19_actor_valid(p_workspace uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
 SELECT coalesce(p_workspace=photara_private.request_workspace(),false)
 AND EXISTS (SELECT 1 FROM photara_identity.accounts a
   JOIN photara_identity.account_identities i ON i.account_id=a.account_id
   WHERE a.account_id=nullif(current_setting('photara.account_id',true),'')::uuid
     AND i.identity_id=nullif(current_setting('photara.identity_id',true),'')::uuid
     AND a.state='active' AND i.state='active')
 AND EXISTS (SELECT 1 FROM photara.workspaces w
   JOIN photara.library_contract_state c ON c.workspace_id=w.workspace_id
   WHERE w.workspace_id=p_workspace AND w.state='active'
     AND c.authority_mode='cloud-member');
$$;

CREATE FUNCTION photara_private.d19_action_bit(p_action text) RETURNS integer
LANGUAGE sql IMMUTABLE SET search_path=pg_catalog,pg_temp AS $$
 SELECT CASE p_action WHEN 'discover' THEN 1 WHEN 'read' THEN 2
  WHEN 'edit' THEN 4 WHEN 'run' THEN 8 WHEN 'invite' THEN 16
  WHEN 'manage-storage' THEN 32 WHEN 'manage-context' THEN 64
  WHEN 'manage-access' THEN 128 ELSE 0 END;
$$;

CREATE FUNCTION photara_private.d19_can_library(p_workspace uuid,p_action text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
 SELECT photara_private.d19_actor_valid(p_workspace)
 AND current_setting('photara.scope_kind',true)='library'
 AND EXISTS (SELECT 1 FROM photara_identity.memberships m
   WHERE m.workspace_id=p_workspace
     AND m.account_id=nullif(current_setting('photara.account_id',true),'')::uuid
     AND m.state='active' AND CASE p_action
       WHEN 'discover' THEN true WHEN 'read' THEN true
       WHEN 'edit' THEN m.role IN ('owner','admin','editor')
       WHEN 'invite' THEN m.role IN ('owner','admin')
       WHEN 'manage-storage' THEN m.role IN ('owner','admin')
       WHEN 'manage-context' THEN m.role IN ('owner','admin','editor')
       WHEN 'manage-access' THEN m.role IN ('owner','admin') ELSE false END);
$$;

CREATE FUNCTION photara_private.can_read_library(p_workspace uuid,p_sensitivity text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
 SELECT coalesce(photara_private.d19_can_library(p_workspace,'read')
   AND p_sensitivity IN ('ordinary','personal'),false);
$$;

CREATE FUNCTION photara_private.can_project(p_workspace uuid,p_project uuid,p_action text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
 SELECT coalesce(photara_private.d19_actor_valid(p_workspace)
 AND (current_setting('photara.scope_kind',true)='library' OR
     (current_setting('photara.scope_kind',true)='project' AND
      p_project=nullif(current_setting('photara.project_id',true),'')::uuid))
 AND photara_private.d19_action_bit(p_action)<>0
 AND EXISTS (
   SELECT 1 FROM photara.project_ownership o
   JOIN photara.project_access_policies p USING (workspace_id,project_id)
   LEFT JOIN photara_identity.project_access_grants g
     ON g.workspace_id=o.workspace_id AND g.project_id=o.project_id
     AND g.account_id=nullif(current_setting('photara.account_id',true),'')::uuid
   LEFT JOIN photara_identity.memberships m ON m.workspace_id=o.workspace_id
     AND m.account_id=nullif(current_setting('photara.account_id',true),'')::uuid AND m.state='active'
   WHERE o.workspace_id=p_workspace AND o.project_id=p_project AND o.registration_state='active'
     AND (g.state IS NULL OR g.state='active')
     AND ((coalesce(g.action_mask,0) | CASE WHEN p.visibility_policy='library-visible' THEN
       CASE m.role WHEN 'owner' THEN p.owner_mask WHEN 'admin' THEN p.admin_mask
         WHEN 'editor' THEN p.editor_mask WHEN 'viewer' THEN p.viewer_mask ELSE 0 END
       ELSE 0 END) & photara_private.d19_action_bit(p_action))=photara_private.d19_action_bit(p_action)
 ),false);
$$;

CREATE FUNCTION photara_private.can_project_media(p_workspace uuid,p_project uuid,p_digest bytea,p_purpose text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
 SELECT photara_private.can_project(p_workspace,p_project,'read')
 AND EXISTS (SELECT 1 FROM photara.project_media_links l WHERE l.workspace_id=p_workspace
   AND l.project_id=p_project AND l.sha256=p_digest AND l.purpose=p_purpose AND l.state='active');
$$;

CREATE FUNCTION photara_private.d19_lock_actor() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 PERFORM 1 FROM photara_identity.accounts a
  WHERE a.account_id=nullif(current_setting('photara.account_id',true),'')::uuid AND a.state='active' FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'not_found_or_forbidden' USING ERRCODE='42501'; END IF;
 PERFORM 1 FROM photara_identity.account_identities i
  WHERE i.identity_id=nullif(current_setting('photara.identity_id',true),'')::uuid
    AND i.account_id=nullif(current_setting('photara.account_id',true),'')::uuid AND i.state='active' FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'not_found_or_forbidden' USING ERRCODE='42501'; END IF;
END;
$$;

CREATE FUNCTION photara_private.authorize_library(p_workspace uuid,p_action text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF p_workspace IS DISTINCT FROM photara_private.request_workspace() THEN
  RAISE EXCEPTION 'not_found_or_forbidden' USING ERRCODE='42501'; END IF;
 PERFORM photara_private.d19_lock_actor();
 IF p_action IN ('discover','read') THEN
  PERFORM 1 FROM photara.workspaces WHERE workspace_id=p_workspace FOR SHARE;
 ELSE
  PERFORM 1 FROM photara.workspaces WHERE workspace_id=p_workspace FOR UPDATE;
 END IF;

 IF NOT coalesce(photara_private.d19_can_library(p_workspace,p_action),false) THEN
  RAISE EXCEPTION 'not_found_or_forbidden' USING ERRCODE='42501'; END IF;
END;
$$;

CREATE FUNCTION photara_private.authorize_project(p_workspace uuid,p_project uuid,p_action text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF p_workspace IS DISTINCT FROM photara_private.request_workspace() THEN
  RAISE EXCEPTION 'not_found_or_forbidden' USING ERRCODE='42501'; END IF;
 PERFORM photara_private.d19_lock_actor();
 IF p_action IN ('discover','read') THEN
  PERFORM 1 FROM photara.workspaces WHERE workspace_id=p_workspace FOR SHARE;
 ELSE
  PERFORM 1 FROM photara.workspaces WHERE workspace_id=p_workspace FOR UPDATE;
 END IF;
 IF p_action IN ('discover','read') THEN
  PERFORM 1 FROM photara.project_access_policies WHERE workspace_id=p_workspace AND project_id=p_project FOR SHARE;
 ELSE
  PERFORM 1 FROM photara.project_access_policies WHERE workspace_id=p_workspace AND project_id=p_project FOR UPDATE;
 END IF;
 IF NOT coalesce(photara_private.can_project(p_workspace,p_project,p_action),false) THEN
  RAISE EXCEPTION 'not_found_or_forbidden' USING ERRCODE='42501'; END IF;
END;
$$;

CREATE FUNCTION photara_private.d19_managers_at_commit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF EXISTS (SELECT 1 FROM photara.project_ownership o WHERE o.registration_state='active'
   AND (NOT EXISTS (SELECT 1 FROM photara.project_access_policies p WHERE p.workspace_id=o.workspace_id AND p.project_id=o.project_id)
   OR NOT EXISTS (SELECT 1 FROM photara.library_contract_state c WHERE c.workspace_id=o.workspace_id)
   OR NOT EXISTS (SELECT 1 FROM photara_identity.project_access_grants g
     JOIN photara_identity.accounts a ON a.account_id=g.account_id AND a.state='active'
     WHERE g.workspace_id=o.workspace_id AND g.project_id=o.project_id
       AND g.state='active' AND g.action_mask=255
       AND EXISTS (SELECT 1 FROM photara_identity.account_identities i
         WHERE i.account_id=a.account_id AND i.state='active'))))
 THEN RAISE EXCEPTION 'd19_last_project_manager' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER d19_project_ownership_managers AFTER INSERT OR UPDATE OR DELETE ON photara.project_ownership
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.d19_managers_at_commit();

CREATE CONSTRAINT TRIGGER d19_project_access_grants_managers AFTER INSERT OR UPDATE OR DELETE ON photara_identity.project_access_grants
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.d19_managers_at_commit();

CREATE CONSTRAINT TRIGGER d19_accounts_managers AFTER INSERT OR UPDATE OR DELETE ON photara_identity.accounts
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.d19_managers_at_commit();

CREATE CONSTRAINT TRIGGER d19_account_identities_managers AFTER INSERT OR UPDATE OR DELETE ON photara_identity.account_identities
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.d19_managers_at_commit();

CREATE FUNCTION photara_private.d19_feed_at_commit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE lib uuid:=photara_private.request_workspace();
BEGIN
 IF lib IS NULL THEN RAISE EXCEPTION 'd19_scope_required' USING ERRCODE='23514'; END IF;
 IF EXISTS (SELECT 1 FROM photara_private.scoped_change_batches b
  JOIN photara_private.scoped_streams s ON s.stream_id=b.stream_id AND s.epoch=b.epoch
  JOIN photara_private.scoped_command_receipts r ON r.workspace_id=b.workspace_id AND r.operation_id=b.operation_id
  WHERE b.workspace_id=lib AND (
    b.workspace_id<>s.workspace_id OR b.project_id IS DISTINCT FROM s.project_id
    OR r.outcome<>'accepted' OR r.accepted_stream_id IS DISTINCT FROM b.stream_id
    OR r.accepted_epoch IS DISTINCT FROM b.epoch OR r.accepted_sequence IS DISTINCT FROM b.sequence
    OR r.project_id IS DISTINCT FROM b.project_id OR r.scope_kind<>s.scope_kind
    OR b.sequence>s.last_sequence
    OR b.change_count<>(SELECT count(*) FROM photara_private.scoped_changes c
       WHERE c.stream_id=b.stream_id AND c.epoch=b.epoch AND c.sequence=b.sequence)
    OR EXISTS (SELECT 1 FROM photara_private.scoped_changes c
       WHERE c.stream_id=b.stream_id AND c.epoch=b.epoch AND c.sequence=b.sequence
         AND (c.ordinal>=b.change_count OR c.workspace_id<>b.workspace_id OR c.project_id IS DISTINCT FROM b.project_id))))
 OR EXISTS (SELECT 1 FROM photara_private.scoped_command_receipts r
  JOIN photara_private.scoped_change_batches b ON b.stream_id=r.accepted_stream_id
    AND b.epoch=r.accepted_epoch AND b.sequence=r.accepted_sequence
  WHERE r.workspace_id=lib AND r.outcome='accepted'
    AND (b.workspace_id<>r.workspace_id OR b.operation_id<>r.operation_id))
 OR EXISTS (SELECT 1 FROM photara_private.scoped_streams s WHERE s.workspace_id=lib
   AND s.last_sequence<>(SELECT count(*) FROM photara_private.scoped_change_batches b
     WHERE b.stream_id=s.stream_id AND b.epoch=s.epoch))
 THEN RAISE EXCEPTION 'd19_scoped_batch_closure' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER d19_scoped_streams_closure AFTER INSERT OR UPDATE OR DELETE ON photara_private.scoped_streams
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.d19_feed_at_commit();

CREATE CONSTRAINT TRIGGER d19_scoped_change_batches_closure AFTER INSERT OR UPDATE OR DELETE ON photara_private.scoped_change_batches
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.d19_feed_at_commit();

CREATE CONSTRAINT TRIGGER d19_scoped_changes_closure AFTER INSERT OR UPDATE OR DELETE ON photara_private.scoped_changes
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.d19_feed_at_commit();

CREATE CONSTRAINT TRIGGER d19_scoped_command_receipts_closure AFTER INSERT OR UPDATE OR DELETE ON photara_private.scoped_command_receipts
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.d19_feed_at_commit();

CREATE FUNCTION photara_private.d19_scoped_sync_clients_ack_insert() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM photara_private.scoped_streams s JOIN photara.library_contract_state c ON c.workspace_id=s.workspace_id WHERE s.stream_id=NEW.stream_id AND s.epoch=NEW.epoch AND NEW.acknowledged_sequence<=s.last_sequence AND NEW.authorization_generation=c.authorization_generation) THEN RAISE EXCEPTION 'd19_ack_scope' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_scoped_sync_clients_ack_insert BEFORE INSERT ON photara_private.scoped_sync_clients
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_scoped_sync_clients_ack_insert();

CREATE FUNCTION photara_private.d19_scoped_sync_clients_ack_update() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM photara_private.scoped_streams s JOIN photara.library_contract_state c ON c.workspace_id=s.workspace_id WHERE s.stream_id=NEW.stream_id AND s.epoch=NEW.epoch AND NEW.acknowledged_sequence<=s.last_sequence AND NEW.authorization_generation=c.authorization_generation) OR NEW.authorization_generation<OLD.authorization_generation OR (NEW.authorization_generation=OLD.authorization_generation AND (NEW.epoch<>OLD.epoch OR NEW.acknowledged_sequence<OLD.acknowledged_sequence)) THEN RAISE EXCEPTION 'd19_ack_scope' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER d19_scoped_sync_clients_ack_update BEFORE UPDATE ON photara_private.scoped_sync_clients
FOR EACH ROW EXECUTE FUNCTION photara_private.d19_scoped_sync_clients_ack_update();

DROP POLICY workspace_scope ON photara.workspaces;

DROP POLICY workspace_scope ON photara_private.workspace_subscriptions;

DROP POLICY workspace_scope ON photara_private.workspace_entitlement_grants;

DROP POLICY workspace_scope ON photara.people;

DROP POLICY workspace_scope ON photara.organizations;

DROP POLICY workspace_scope ON photara.social_profiles;

DROP POLICY workspace_scope ON photara.person_capabilities;

DROP POLICY workspace_scope ON photara.person_labels;

DROP POLICY workspace_scope ON photara.organization_labels;

DROP POLICY workspace_scope ON photara.person_organization_relationships;

DROP POLICY workspace_scope ON photara.location_kinds;

DROP POLICY workspace_scope ON photara.location_kind_terms;

DROP POLICY workspace_scope ON photara.locations;

DROP POLICY workspace_scope ON photara.storage_roots;

DROP POLICY workspace_scope ON photara.project_catalog;

DROP POLICY workspace_scope ON photara.project_locators;

DROP POLICY workspace_scope ON photara.package_observations;

DROP POLICY workspace_scope ON photara.library_media;

DROP POLICY workspace_scope ON photara.workspace_change_batches;

DROP POLICY workspace_scope ON photara.workspace_changes;

DROP POLICY workspace_scope ON photara_private.media_objects;

DROP POLICY workspace_scope ON photara_private.workspace_streams;

DROP POLICY workspace_scope ON photara_private.mutation_receipts;

DROP POLICY workspace_scope ON photara_private.sync_clients;

DROP POLICY workspace_scope ON photara_private.media_upload_sessions;

REVOKE ALL ON photara.workspace_change_batches,photara.workspace_changes,photara_private.workspace_streams,photara_private.mutation_receipts,photara_private.sync_clients FROM photara_api;

REVOKE EXECUTE ON FUNCTION photara_private.authorize_workspace(uuid,text) FROM photara_api,photara_control;

ALTER TABLE photara.library_contract_state ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.library_contract_state FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.project_ownership ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.project_ownership FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.project_access_policies ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.project_access_policies FORCE ROW LEVEL SECURITY;

ALTER TABLE photara_identity.project_access_grants ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara_identity.project_access_grants FORCE ROW LEVEL SECURITY;

ALTER TABLE photara_identity.project_invitations ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara_identity.project_invitations FORCE ROW LEVEL SECURITY;

ALTER TABLE photara_private.project_invitation_secrets ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara_private.project_invitation_secrets FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.storage_location_specs ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.storage_location_specs FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.storage_slots ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.storage_slots FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.storage_slot_names ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.storage_slot_names FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.library_variables ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.library_variables FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.library_variable_values ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.library_variable_values FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.library_variable_names ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.library_variable_names FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.library_expressions ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.library_expressions FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.library_expression_dependencies ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.library_expression_dependencies FORCE ROW LEVEL SECURITY;

ALTER TABLE photara.project_media_links ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara.project_media_links FORCE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_streams ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_streams FORCE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_change_batches ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_change_batches FORCE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_changes ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_changes FORCE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_command_receipts ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_command_receipts FORCE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_sync_clients ENABLE ROW LEVEL SECURITY;

ALTER TABLE photara_private.scoped_sync_clients FORCE ROW LEVEL SECURITY;

CREATE POLICY d19_owner ON photara.workspaces TO photara_owner USING (true) WITH CHECK (true);

CREATE POLICY d19_read ON photara.workspaces FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.workspaces FOR INSERT TO photara_api WITH CHECK (false);

CREATE POLICY d19_update ON photara.workspaces FOR UPDATE TO photara_api USING (false) WITH CHECK (false);

CREATE POLICY d19_control ON photara.workspaces TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara_private.workspace_subscriptions TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.workspace_subscriptions TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.workspace_subscriptions FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.workspace_subscriptions TO photara_control;

CREATE POLICY d19_owner ON photara_private.workspace_entitlement_grants TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.workspace_entitlement_grants TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.workspace_entitlement_grants FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.workspace_entitlement_grants TO photara_control;

CREATE POLICY d19_owner ON photara.people TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.people FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.people FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.people FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.people TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.organizations TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.organizations FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.organizations FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.organizations FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.organizations TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.social_profiles TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.social_profiles FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.social_profiles FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.social_profiles FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.social_profiles TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.person_capabilities TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.person_capabilities FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.person_capabilities FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.person_capabilities FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_delete ON photara.person_capabilities FOR DELETE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.person_capabilities TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.person_labels TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.person_labels FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.person_labels FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.person_labels FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_delete ON photara.person_labels FOR DELETE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.person_labels TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.organization_labels TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.organization_labels FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.organization_labels FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.organization_labels FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_delete ON photara.organization_labels FOR DELETE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.organization_labels TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.person_organization_relationships TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.person_organization_relationships FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.person_organization_relationships FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.person_organization_relationships FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.person_organization_relationships TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.location_kinds TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.location_kinds FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.location_kinds FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.location_kinds FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.location_kinds TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.location_kind_terms TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.location_kind_terms FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.location_kind_terms FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.location_kind_terms FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.location_kind_terms TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.locations TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.locations FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.locations FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.locations FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.locations TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.storage_roots TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.storage_roots FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.storage_roots FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-storage'));

CREATE POLICY d19_update ON photara.storage_roots FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'manage-storage')) WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-storage'));

CREATE POLICY d19_control ON photara.storage_roots TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.project_catalog TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.project_catalog FOR SELECT TO photara_api,photara_control USING (photara_private.can_project(workspace_id,project_id,'read'));

CREATE POLICY d19_insert ON photara.project_catalog FOR INSERT TO photara_api WITH CHECK (photara_private.can_project(workspace_id,project_id,'edit'));

CREATE POLICY d19_update ON photara.project_catalog FOR UPDATE TO photara_api USING (photara_private.can_project(workspace_id,project_id,'edit')) WITH CHECK (photara_private.can_project(workspace_id,project_id,'edit'));

CREATE POLICY d19_control ON photara.project_catalog TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.project_locators TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.project_locators FOR SELECT TO photara_api,photara_control USING (photara_private.can_project(workspace_id,project_id,'read'));

CREATE POLICY d19_insert ON photara.project_locators FOR INSERT TO photara_api WITH CHECK (photara_private.can_project(workspace_id,project_id,'edit') AND photara_private.can_project(workspace_id,project_id,'manage-storage'));

CREATE POLICY d19_update ON photara.project_locators FOR UPDATE TO photara_api USING (photara_private.can_project(workspace_id,project_id,'edit') AND photara_private.can_project(workspace_id,project_id,'manage-storage')) WITH CHECK (photara_private.can_project(workspace_id,project_id,'edit') AND photara_private.can_project(workspace_id,project_id,'manage-storage'));

CREATE POLICY d19_control ON photara.project_locators TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.package_observations TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.package_observations FOR SELECT TO photara_api,photara_control USING (photara_private.can_project(workspace_id,project_id,'read'));

CREATE POLICY d19_insert ON photara.package_observations FOR INSERT TO photara_api WITH CHECK (photara_private.can_project(workspace_id,project_id,'edit'));

CREATE POLICY d19_update ON photara.package_observations FOR UPDATE TO photara_api USING (photara_private.can_project(workspace_id,project_id,'edit')) WITH CHECK (photara_private.can_project(workspace_id,project_id,'edit'));

CREATE POLICY d19_control ON photara.package_observations TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.library_media TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.library_media FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary') OR photara_private.can_project_media(workspace_id,nullif(current_setting('photara.project_id',true),'')::uuid,sha256,current_setting('photara.media_purpose',true)));

CREATE POLICY d19_insert ON photara.library_media FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_update ON photara.library_media FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'edit')) WITH CHECK (photara_private.d19_can_library(workspace_id,'edit'));

CREATE POLICY d19_control ON photara.library_media TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_owner ON photara.workspace_change_batches TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara.workspace_change_batches TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara.workspace_change_batches FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara.workspace_change_batches TO photara_control;

CREATE POLICY d19_owner ON photara.workspace_changes TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara.workspace_changes TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara.workspace_changes FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara.workspace_changes TO photara_control;

CREATE POLICY d19_owner ON photara_private.media_objects TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.media_objects TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.media_objects FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.media_objects TO photara_control;

CREATE POLICY d19_owner ON photara_private.workspace_streams TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.workspace_streams TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.workspace_streams FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.workspace_streams TO photara_control;

CREATE POLICY d19_owner ON photara_private.mutation_receipts TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.mutation_receipts TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.mutation_receipts FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.mutation_receipts TO photara_control;

CREATE POLICY d19_owner ON photara_private.sync_clients TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.sync_clients TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.sync_clients FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.sync_clients TO photara_control;

CREATE POLICY d19_owner ON photara_private.media_upload_sessions TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.media_upload_sessions TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.media_upload_sessions FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.media_upload_sessions TO photara_control;

CREATE POLICY d19_owner ON photara.library_contract_state TO photara_owner USING (true) WITH CHECK (true);

CREATE POLICY d19_auth_read ON photara.library_contract_state FOR SELECT TO photara_auth_read USING (true);

CREATE POLICY d19_control ON photara.library_contract_state TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara.library_contract_state FROM photara_api;

GRANT SELECT ON photara.library_contract_state TO photara_auth_read;

GRANT SELECT,INSERT,UPDATE ON photara.library_contract_state TO photara_control;

CREATE POLICY d19_owner ON photara.project_ownership TO photara_owner USING (true) WITH CHECK (true);

CREATE POLICY d19_auth_read ON photara.project_ownership FOR SELECT TO photara_auth_read USING (true);

CREATE POLICY d19_control ON photara.project_ownership TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara.project_ownership FROM photara_api;

GRANT SELECT ON photara.project_ownership TO photara_auth_read;

GRANT SELECT,INSERT,UPDATE ON photara.project_ownership TO photara_control;

CREATE POLICY d19_owner ON photara.project_access_policies TO photara_owner USING (true) WITH CHECK (true);

CREATE POLICY d19_auth_read ON photara.project_access_policies FOR SELECT TO photara_auth_read USING (true);

CREATE POLICY d19_control ON photara.project_access_policies TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara.project_access_policies FROM photara_api;

GRANT SELECT ON photara.project_access_policies TO photara_auth_read;

GRANT SELECT,INSERT,UPDATE ON photara.project_access_policies TO photara_control;

CREATE POLICY d19_owner ON photara_identity.project_access_grants TO photara_owner USING (true) WITH CHECK (true);

CREATE POLICY d19_auth_read ON photara_identity.project_access_grants FOR SELECT TO photara_auth_read USING (true);

CREATE POLICY d19_control ON photara_identity.project_access_grants TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_identity.project_access_grants FROM photara_api;

GRANT SELECT ON photara_identity.project_access_grants TO photara_auth_read;

GRANT SELECT,INSERT,UPDATE ON photara_identity.project_access_grants TO photara_control;

CREATE POLICY d19_owner ON photara_identity.project_invitations TO photara_owner USING (true) WITH CHECK (true);

CREATE POLICY d19_auth_read ON photara_identity.project_invitations FOR SELECT TO photara_auth_read USING (true);

CREATE POLICY d19_control ON photara_identity.project_invitations TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_identity.project_invitations FROM photara_api;

GRANT SELECT ON photara_identity.project_invitations TO photara_auth_read;

GRANT SELECT,INSERT,UPDATE ON photara_identity.project_invitations TO photara_control;

CREATE POLICY d19_owner ON photara_private.project_invitation_secrets TO photara_owner USING (EXISTS (SELECT 1 FROM photara_identity.project_invitations i WHERE i.invitation_id=project_invitation_secrets.invitation_id AND i.workspace_id=photara_private.request_workspace())) WITH CHECK (EXISTS (SELECT 1 FROM photara_identity.project_invitations i WHERE i.invitation_id=project_invitation_secrets.invitation_id AND i.workspace_id=photara_private.request_workspace()));

CREATE POLICY d19_control ON photara_private.project_invitation_secrets TO photara_control USING (EXISTS (SELECT 1 FROM photara_identity.project_invitations i WHERE i.invitation_id=project_invitation_secrets.invitation_id AND i.workspace_id=photara_private.request_workspace())) WITH CHECK (EXISTS (SELECT 1 FROM photara_identity.project_invitations i WHERE i.invitation_id=project_invitation_secrets.invitation_id AND i.workspace_id=photara_private.request_workspace()));

REVOKE ALL ON photara_private.project_invitation_secrets FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.project_invitation_secrets TO photara_control;

CREATE POLICY d19_owner ON photara.storage_location_specs TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.storage_location_specs FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.storage_location_specs FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-storage'));

CREATE POLICY d19_update ON photara.storage_location_specs FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'manage-storage')) WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-storage'));

CREATE POLICY d19_control ON photara.storage_location_specs TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

GRANT SELECT,INSERT,UPDATE ON photara.storage_location_specs TO photara_api,photara_control;

CREATE POLICY d19_owner ON photara.storage_slots TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.storage_slots FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.storage_slots FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-storage'));

CREATE POLICY d19_update ON photara.storage_slots FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'manage-storage')) WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-storage'));

CREATE POLICY d19_control ON photara.storage_slots TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

GRANT SELECT,INSERT,UPDATE ON photara.storage_slots TO photara_api,photara_control;

CREATE POLICY d19_owner ON photara.storage_slot_names TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.storage_slot_names FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,'ordinary'));

CREATE POLICY d19_insert ON photara.storage_slot_names FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-storage'));

CREATE POLICY d19_update ON photara.storage_slot_names FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'manage-storage')) WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-storage'));

CREATE POLICY d19_control ON photara.storage_slot_names TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

GRANT SELECT,INSERT,UPDATE ON photara.storage_slot_names TO photara_api,photara_control;

CREATE POLICY d19_owner ON photara.library_variables TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.library_variables FOR SELECT TO photara_api,photara_control USING (photara_private.can_read_library(workspace_id,sensitivity));

CREATE POLICY d19_insert ON photara.library_variables FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_update ON photara.library_variables FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'manage-context')) WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_control ON photara.library_variables TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

GRANT SELECT,INSERT,UPDATE ON photara.library_variables TO photara_api,photara_control;

CREATE POLICY d19_owner ON photara.library_variable_values TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.library_variable_values FOR SELECT TO photara_api,photara_control USING (EXISTS (SELECT 1 FROM photara.library_variables v WHERE v.workspace_id=library_variable_values.workspace_id AND v.variable_id=library_variable_values.variable_id AND photara_private.can_read_library(v.workspace_id,v.sensitivity)));

CREATE POLICY d19_insert ON photara.library_variable_values FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_update ON photara.library_variable_values FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'manage-context')) WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_control ON photara.library_variable_values TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

GRANT SELECT,INSERT,UPDATE ON photara.library_variable_values TO photara_api,photara_control;

CREATE POLICY d19_owner ON photara.library_variable_names TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.library_variable_names FOR SELECT TO photara_api,photara_control USING (EXISTS (SELECT 1 FROM photara.library_variables v WHERE v.workspace_id=library_variable_names.workspace_id AND v.variable_id=library_variable_names.variable_id AND photara_private.can_read_library(v.workspace_id,v.sensitivity)));

CREATE POLICY d19_insert ON photara.library_variable_names FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_update ON photara.library_variable_names FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'manage-context')) WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_control ON photara.library_variable_names TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

GRANT SELECT,INSERT,UPDATE ON photara.library_variable_names TO photara_api,photara_control;

CREATE POLICY d19_owner ON photara.library_expressions TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.library_expressions FOR SELECT TO photara_api,photara_control USING (EXISTS (SELECT 1 FROM photara.library_variables v WHERE v.workspace_id=library_expressions.workspace_id AND v.variable_id=library_expressions.owner_variable_id AND photara_private.can_read_library(v.workspace_id,v.sensitivity)));

CREATE POLICY d19_insert ON photara.library_expressions FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_update ON photara.library_expressions FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'manage-context')) WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_control ON photara.library_expressions TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

GRANT SELECT,INSERT,UPDATE ON photara.library_expressions TO photara_api,photara_control;

CREATE POLICY d19_owner ON photara.library_expression_dependencies TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.library_expression_dependencies FOR SELECT TO photara_api,photara_control USING (EXISTS (SELECT 1 FROM photara.library_expressions e JOIN photara.library_variables v ON v.workspace_id=e.workspace_id AND v.variable_id=e.owner_variable_id WHERE e.workspace_id=library_expression_dependencies.workspace_id AND e.expression_id=library_expression_dependencies.expression_id AND photara_private.can_read_library(v.workspace_id,v.sensitivity)));

CREATE POLICY d19_insert ON photara.library_expression_dependencies FOR INSERT TO photara_api WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_update ON photara.library_expression_dependencies FOR UPDATE TO photara_api USING (photara_private.d19_can_library(workspace_id,'manage-context')) WITH CHECK (photara_private.d19_can_library(workspace_id,'manage-context'));

CREATE POLICY d19_control ON photara.library_expression_dependencies TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

GRANT SELECT,INSERT,UPDATE ON photara.library_expression_dependencies TO photara_api,photara_control;

CREATE POLICY d19_owner ON photara.project_media_links TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_read ON photara.project_media_links FOR SELECT TO photara_api,photara_control USING (photara_private.can_project(workspace_id,project_id,'read'));

CREATE POLICY d19_insert ON photara.project_media_links FOR INSERT TO photara_api WITH CHECK (photara_private.can_project(workspace_id,project_id,'edit'));

CREATE POLICY d19_update ON photara.project_media_links FOR UPDATE TO photara_api USING (photara_private.can_project(workspace_id,project_id,'edit')) WITH CHECK (photara_private.can_project(workspace_id,project_id,'edit'));

CREATE POLICY d19_control ON photara.project_media_links TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

GRANT SELECT,INSERT,UPDATE ON photara.project_media_links TO photara_api,photara_control;

CREATE POLICY d19_owner ON photara_private.scoped_streams TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.scoped_streams TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.scoped_streams FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.scoped_streams TO photara_control;

CREATE POLICY d19_owner ON photara_private.scoped_change_batches TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.scoped_change_batches TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.scoped_change_batches FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.scoped_change_batches TO photara_control;

CREATE POLICY d19_owner ON photara_private.scoped_changes TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.scoped_changes TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.scoped_changes FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.scoped_changes TO photara_control;

CREATE POLICY d19_owner ON photara_private.scoped_command_receipts TO photara_owner USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

CREATE POLICY d19_control ON photara_private.scoped_command_receipts TO photara_control USING (workspace_id=photara_private.request_workspace()) WITH CHECK (workspace_id=photara_private.request_workspace());

REVOKE ALL ON photara_private.scoped_command_receipts FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.scoped_command_receipts TO photara_control;

CREATE POLICY d19_owner ON photara_private.scoped_sync_clients TO photara_owner USING (EXISTS (SELECT 1 FROM photara_private.scoped_streams s WHERE s.stream_id=scoped_sync_clients.stream_id AND s.workspace_id=photara_private.request_workspace())) WITH CHECK (EXISTS (SELECT 1 FROM photara_private.scoped_streams s WHERE s.stream_id=scoped_sync_clients.stream_id AND s.workspace_id=photara_private.request_workspace()));

CREATE POLICY d19_control ON photara_private.scoped_sync_clients TO photara_control USING (EXISTS (SELECT 1 FROM photara_private.scoped_streams s WHERE s.stream_id=scoped_sync_clients.stream_id AND s.workspace_id=photara_private.request_workspace())) WITH CHECK (EXISTS (SELECT 1 FROM photara_private.scoped_streams s WHERE s.stream_id=scoped_sync_clients.stream_id AND s.workspace_id=photara_private.request_workspace()));

REVOKE ALL ON photara_private.scoped_sync_clients FROM photara_api;

GRANT SELECT,INSERT,UPDATE ON photara_private.scoped_sync_clients TO photara_control;

REVOKE INSERT,UPDATE ON photara.project_media_links FROM photara_api;

GRANT SELECT ON photara.project_media_links TO photara_api;

REVOKE ALL ON ALL TABLES IN SCHEMA photara,photara_identity,photara_private FROM PUBLIC;

REVOKE ALL ON ALL FUNCTIONS IN SCHEMA photara,photara_identity,photara_private FROM PUBLIC;

GRANT EXECUTE ON FUNCTION photara_private.authorize_library(uuid,text) TO photara_api,photara_control;

GRANT EXECUTE ON FUNCTION photara_private.authorize_project(uuid,uuid,text) TO photara_api,photara_control;

GRANT EXECUTE ON FUNCTION photara_private.can_read_library(uuid,text) TO photara_api,photara_control;

GRANT EXECUTE ON FUNCTION photara_private.can_project(uuid,uuid,text) TO photara_api,photara_control;

GRANT EXECUTE ON FUNCTION photara_private.can_project_media(uuid,uuid,bytea,text) TO photara_api,photara_control;

GRANT EXECUTE ON FUNCTION photara_private.d19_can_library(uuid,text) TO photara_api,photara_control;

-- Activation runner must first settle every sealed v1 operation, verify 0007
-- privileges, stage explicit associations, and confirm all CXT3 proof gates.
-- Check exactly one affected row; run together with the complete guard cutover.
UPDATE photara.schema_metadata SET minimum_api=2
WHERE singleton=true AND schema_family='photara.service.g2' AND schema_epoch=1 AND minimum_api=1;
