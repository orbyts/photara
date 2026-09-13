-- CXT3c executable service migration; promoted from S4.
CREATE FUNCTION photara_private.lock_library() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE scope_id uuid;
BEGIN
  IF TG_OP='DELETE' THEN scope_id:=OLD.library_id; ELSE scope_id:=NEW.library_id; END IF;
  PERFORM 1 FROM photara.libraries WHERE library_id=scope_id FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'library_unavailable' USING ERRCODE='23514'; END IF;
  IF TG_OP='DELETE' THEN RETURN OLD; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_revision() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
DECLARE field_name text;
BEGIN
  IF TG_OP='DELETE' THEN RAISE EXCEPTION 'hard_delete_not_supported' USING ERRCODE='23514'; END IF;
  IF TG_OP='INSERT' THEN
    IF NEW.revision<>1 THEN RAISE EXCEPTION 'revision_must_start_at_one' USING ERRCODE='23514'; END IF;
  ELSE
    IF NEW.revision<>OLD.revision+1 OR NEW.created_at<>OLD.created_at
      OR (to_jsonb(NEW)->'library_id') IS DISTINCT FROM (to_jsonb(OLD)->'library_id')
    THEN RAISE EXCEPTION 'revision_or_identity_conflict' USING ERRCODE='23514'; END IF;
    FOREACH field_name IN ARRAY TG_ARGV LOOP
      IF (to_jsonb(NEW)->field_name) IS DISTINCT FROM (to_jsonb(OLD)->field_name)
      THEN RAISE EXCEPTION 'immutable_identity' USING ERRCODE='23514'; END IF;
    END LOOP;
    IF (to_jsonb(OLD)->>'state') IN ('merged','tombstoned','deleted')
      AND ((to_jsonb(NEW)->>'state') IS DISTINCT FROM (to_jsonb(OLD)->>'state')
        OR (to_jsonb(NEW)->'merged_into_id') IS DISTINCT FROM (to_jsonb(OLD)->'merged_into_id'))
    THEN RAISE EXCEPTION 'restore_or_unmerge_not_supported' USING ERRCODE='23514'; END IF;
  END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.append_only() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  RAISE EXCEPTION 'append_only_record' USING ERRCODE='23514';
END;
$$;

CREATE FUNCTION photara_private.guard_merge() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
DECLARE target_state text; cyclic boolean; own_id uuid;
BEGIN
  IF NEW.merged_into_id IS NULL THEN RETURN NEW; END IF;
  IF TG_OP='UPDATE' AND NEW.merged_into_id IS NOT DISTINCT FROM OLD.merged_into_id THEN RETURN NEW; END IF;
  own_id:=(to_jsonb(NEW)->>TG_ARGV[0])::uuid;
  EXECUTE format('SELECT state FROM %I.%I WHERE library_id=$1 AND %I=$2',
    TG_TABLE_SCHEMA,TG_TABLE_NAME,TG_ARGV[0])
    INTO target_state USING NEW.library_id,NEW.merged_into_id;
  IF target_state IS DISTINCT FROM 'active'
    THEN RAISE EXCEPTION 'merge_target_not_active' USING ERRCODE='23514'; END IF;
  EXECUTE format(
    'WITH RECURSIVE chain(id) AS
      (SELECT $1::uuid UNION SELECT t.merged_into_id FROM %I.%I t
       JOIN chain c ON t.%I=c.id WHERE t.library_id=$2 AND t.merged_into_id IS NOT NULL)
     SELECT EXISTS(SELECT 1 FROM chain WHERE id=$3)',
    TG_TABLE_SCHEMA,TG_TABLE_NAME,TG_ARGV[0])
    INTO cyclic USING NEW.merged_into_id,NEW.library_id,own_id;
  IF cyclic THEN RAISE EXCEPTION 'merge_cycle' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_kind_term() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF TG_OP='DELETE' THEN RAISE EXCEPTION 'term_reserved' USING ERRCODE='23514'; END IF;
  IF TG_OP='UPDATE' AND
    (NEW.library_id IS DISTINCT FROM OLD.library_id OR NEW.term_key<>OLD.term_key
     OR NEW.policy_version<>OLD.policy_version)
  THEN RAISE EXCEPTION 'claim_identity_is_immutable' USING ERRCODE='23514'; END IF;
  IF TG_OP='UPDATE' AND NEW.location_kind_id<>OLD.location_kind_id AND NOT EXISTS(
    SELECT 1 FROM photara.location_kinds source JOIN photara.location_kinds target
      ON target.library_id=source.library_id AND target.location_kind_id=NEW.location_kind_id
    WHERE source.library_id=OLD.library_id AND source.location_kind_id=OLD.location_kind_id
      AND source.state='merged' AND source.merged_into_id=NEW.location_kind_id AND target.state='active')
  THEN RAISE EXCEPTION 'term_transfer_requires_merge' USING ERRCODE='23514'; END IF;
  IF NOT EXISTS(SELECT 1 FROM photara.libraries
    WHERE library_id=NEW.library_id AND term_policy_version=NEW.policy_version)
  THEN RAISE EXCEPTION 'normalization_policy_mismatch' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_location() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.state='active' THEN
    IF NOT EXISTS(SELECT 1 FROM photara.location_kinds
      WHERE library_id=NEW.library_id AND location_kind_id=NEW.location_kind_id AND state='active')
    THEN RAISE EXCEPTION 'location_kind_not_active' USING ERRCODE='23514'; END IF;
    IF NEW.parent_location_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM photara.locations
      WHERE library_id=NEW.library_id AND location_id=NEW.parent_location_id AND state='active')
    THEN RAISE EXCEPTION 'location_parent_not_active' USING ERRCODE='23514'; END IF;
  END IF;
  IF NEW.parent_location_id IS NOT NULL AND EXISTS(
    WITH RECURSIVE ancestors(id) AS (
      SELECT NEW.parent_location_id
      UNION
      SELECT l.parent_location_id FROM photara.locations l JOIN ancestors a ON l.location_id=a.id
      WHERE l.library_id=NEW.library_id AND l.parent_location_id IS NOT NULL
    ) SELECT 1 FROM ancestors WHERE id=NEW.location_id)
  THEN RAISE EXCEPTION 'location_parent_cycle' USING ERRCODE='23514'; END IF;
  IF TG_OP='UPDATE' AND NEW.state<>'active' AND EXISTS(
    SELECT 1 FROM photara.locations WHERE library_id=OLD.library_id
      AND parent_location_id=OLD.location_id AND state='active')
  THEN RAISE EXCEPTION 'location_has_live_children' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_kind_history() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF OLD.state<>'active' AND (NEW.retirement_terms IS DISTINCT FROM OLD.retirement_terms
    OR NEW.canonical_key<>OLD.canonical_key OR NEW.canonical_display<>OLD.canonical_display)
  THEN RAISE EXCEPTION 'retirement_provenance_is_immutable' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_relationship() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.state<>'active' THEN RETURN NEW; END IF;
  IF NOT EXISTS(SELECT 1 FROM photara.people WHERE library_id=NEW.library_id AND person_id=NEW.person_id AND state='active')
    OR NOT EXISTS(SELECT 1 FROM photara.organizations WHERE library_id=NEW.library_id AND organization_id=NEW.organization_id AND state='active')
  THEN RAISE EXCEPTION 'relationship_party_not_active' USING ERRCODE='23514'; END IF;
  IF EXISTS(SELECT 1 FROM photara.person_organization_relationships r
    WHERE r.library_id=NEW.library_id AND r.person_id=NEW.person_id
      AND r.organization_id=NEW.organization_id AND r.relationship_type=NEW.relationship_type
      AND r.relationship_id<>NEW.relationship_id AND r.state='active'
      AND tstzrange(r.valid_from,r.valid_until,'[)') && tstzrange(NEW.valid_from,NEW.valid_until,'[)'))
  THEN RAISE EXCEPTION 'relationship_interval_overlap' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_social_profile() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF TG_OP='UPDATE' AND OLD.provider_subject_id IS NOT NULL AND
    (NEW.provider_subject_id IS DISTINCT FROM OLD.provider_subject_id
     OR NEW.subject_namespace IS DISTINCT FROM OLD.subject_namespace)
  THEN RAISE EXCEPTION 'social_subject_is_immutable' USING ERRCODE='23514'; END IF;
  IF NEW.state='active' AND
    ((NEW.person_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM photara.people
      WHERE library_id=NEW.library_id AND person_id=NEW.person_id AND state='active'))
     OR (NEW.organization_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM photara.organizations
      WHERE library_id=NEW.library_id AND organization_id=NEW.organization_id AND state='active')))
  THEN RAISE EXCEPTION 'social_owner_not_active' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_retirement() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.state='active' THEN RETURN NEW; END IF;
  IF TG_TABLE_NAME='people' THEN
    IF EXISTS(SELECT 1 FROM photara.social_profiles WHERE library_id=OLD.library_id AND person_id=OLD.person_id AND state='active')
    THEN RAISE EXCEPTION 'party_has_live_social_profiles' USING ERRCODE='23514'; END IF;
    IF EXISTS(SELECT 1 FROM photara.person_organization_relationships WHERE library_id=OLD.library_id AND person_id=OLD.person_id AND state='active')
    THEN RAISE EXCEPTION 'party_has_live_relationships' USING ERRCODE='23514'; END IF;
  ELSIF TG_TABLE_NAME='organizations' THEN
    IF EXISTS(SELECT 1 FROM photara.social_profiles WHERE library_id=OLD.library_id AND organization_id=OLD.organization_id AND state='active')
    THEN RAISE EXCEPTION 'party_has_live_social_profiles' USING ERRCODE='23514'; END IF;
    IF EXISTS(SELECT 1 FROM photara.person_organization_relationships WHERE library_id=OLD.library_id AND organization_id=OLD.organization_id AND state='active')
    THEN RAISE EXCEPTION 'party_has_live_relationships' USING ERRCODE='23514'; END IF;
  ELSIF TG_TABLE_NAME='location_kinds' THEN
    IF EXISTS(SELECT 1 FROM photara.locations WHERE library_id=OLD.library_id AND location_kind_id=OLD.location_kind_id AND state='active')
    THEN RAISE EXCEPTION 'kind_has_live_locations' USING ERRCODE='23514'; END IF;
  ELSIF TG_TABLE_NAME='storage_roots' THEN
    IF EXISTS(SELECT 1 FROM photara.project_locators WHERE library_id=OLD.library_id AND storage_root_id=OLD.storage_root_id AND state='active')
    THEN RAISE EXCEPTION 'root_has_live_locators' USING ERRCODE='23514'; END IF;
  END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_locator() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.state='active' AND NEW.storage_root_id IS NOT NULL AND NOT EXISTS(
    SELECT 1 FROM photara.storage_roots WHERE library_id=NEW.library_id
      AND storage_root_id=NEW.storage_root_id AND state='active')
  THEN RAISE EXCEPTION 'locator_root_not_active' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_owner() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE scope_id uuid;
BEGIN
  IF TG_OP='DELETE' THEN scope_id:=OLD.library_id; ELSE scope_id:=NEW.library_id; END IF;
  IF EXISTS(SELECT 1 FROM photara.libraries WHERE library_id=scope_id AND state='active')
    AND NOT EXISTS(SELECT 1 FROM photara_identity.memberships m
      JOIN photara_identity.accounts a ON a.account_id=m.account_id
      WHERE m.library_id=scope_id AND m.role='owner' AND m.state='active' AND a.state='active')
  THEN RAISE EXCEPTION 'library_requires_active_owner' USING ERRCODE='23514'; END IF;
  RETURN NULL;
END;
$$;

CREATE FUNCTION photara_private.guard_account_disable() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.state<>'active' AND EXISTS(SELECT 1 FROM photara_identity.memberships
    WHERE account_id=OLD.account_id AND state='active' AND role='owner')
  THEN RAISE EXCEPTION 'transfer_or_revoke_owner_memberships_first' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_stream() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF TG_OP='INSERT' THEN
    IF NEW.last_sequence<>0 OR NEW.minimum_retained_sequence<>0
    THEN RAISE EXCEPTION 'new_stream_must_be_empty' USING ERRCODE='23514'; END IF;
  ELSE
    IF NEW.library_id<>OLD.library_id OR NEW.epoch<>OLD.epoch
      OR NEW.last_sequence<>OLD.last_sequence+1
      OR NEW.minimum_retained_sequence<>OLD.minimum_retained_sequence
    THEN RAISE EXCEPTION 'stream_advance_invalid' USING ERRCODE='23514'; END IF;
  END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_batch() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
DECLARE actual_count bigint; first_ordinal integer; last_ordinal integer;
BEGIN
  SELECT count(*),min(ordinal),max(ordinal) INTO actual_count,first_ordinal,last_ordinal
    FROM photara.library_changes
    WHERE library_id=NEW.library_id AND epoch=NEW.epoch AND sequence=NEW.sequence;
  IF actual_count<>NEW.change_count OR first_ordinal<>0 OR last_ordinal<>NEW.change_count-1
  THEN RAISE EXCEPTION 'incomplete_change_batch' USING ERRCODE='23514'; END IF;
  IF NOT EXISTS(SELECT 1 FROM photara_private.mutation_receipts
    WHERE library_id=NEW.library_id AND mutation_id=NEW.mutation_id
      AND outcome='accepted' AND accepted_epoch=NEW.epoch AND accepted_sequence=NEW.sequence)
    OR NOT EXISTS(SELECT 1 FROM photara_private.library_streams
    WHERE library_id=NEW.library_id AND epoch=NEW.epoch AND last_sequence=NEW.sequence)
  THEN RAISE EXCEPTION 'batch_receipt_stream_mismatch' USING ERRCODE='23514'; END IF;
  RETURN NULL;
END;
$$;

CREATE FUNCTION photara_private.guard_stream_commit() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF NEW.last_sequence>0 AND NOT EXISTS(SELECT 1 FROM photara.library_change_batches
    WHERE library_id=NEW.library_id AND epoch=NEW.epoch AND sequence=NEW.last_sequence)
  THEN RAISE EXCEPTION 'stream_advance_requires_batch' USING ERRCODE='23514'; END IF;
  RETURN NULL;
END;
$$;

CREATE FUNCTION photara_private.guard_media_upload() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF OLD.state='complete' AND (NEW.state<>'complete' OR NEW.completed_at IS DISTINCT FROM OLD.completed_at)
  THEN RAISE EXCEPTION 'verified_upload_is_terminal' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;

CREATE FUNCTION photara_private.guard_sync_ack() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,pg_temp AS $$
BEGIN
  IF TG_OP='UPDATE' AND
    (NEW.library_id<>OLD.library_id OR NEW.account_id<>OLD.account_id OR NEW.device_id<>OLD.device_id
      OR NEW.stream_epoch<>OLD.stream_epoch OR NEW.acknowledged_sequence<OLD.acknowledged_sequence)
  THEN RAISE EXCEPTION 'sync_ack_regression' USING ERRCODE='23514'; END IF;
  IF NOT EXISTS(SELECT 1 FROM photara_private.library_streams WHERE library_id=NEW.library_id
    AND epoch=NEW.stream_epoch AND last_sequence>=NEW.acknowledged_sequence)
  THEN RAISE EXCEPTION 'sync_ack_ahead_of_stream' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END;
$$;
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara_private.media_upload_sessions
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('upload_id','sha256','staging_object_key');
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara_private.media_upload_sessions
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER c_upload_terminal BEFORE UPDATE ON photara_private.media_upload_sessions
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_media_upload();
CREATE TRIGGER claim_receipt_immutable BEFORE UPDATE OR DELETE ON photara_private.library_claim_receipts
  FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.libraries
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('library_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara_identity.accounts
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('account_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara_identity.account_identities
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('identity_id','account_id','issuer','subject');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara_identity.memberships
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('membership_id','account_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara_private.library_subscriptions
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('subscription_id','provider','provider_subscription_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara_private.library_entitlement_grants
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('grant_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara_private.account_developer_grants
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('grant_id','account_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.social_profiles
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('social_profile_id','person_id','organization_id','provider_id');
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.social_profiles
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER c_social_profile BEFORE INSERT OR UPDATE ON photara.social_profiles
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_social_profile();
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.people
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('person_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.organizations
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('organization_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.person_organization_relationships
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('relationship_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.location_kinds
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('location_kind_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.locations
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('location_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.storage_roots
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('storage_root_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.project_catalog
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('project_id');
CREATE TRIGGER b_revision BEFORE INSERT OR UPDATE OR DELETE ON photara.project_locators
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_revision('locator_id');
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara_identity.memberships
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara_private.library_subscriptions
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara_private.library_entitlement_grants
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.people
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.organizations
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.person_capabilities
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.person_labels
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.organization_labels
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.person_organization_relationships
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.location_kinds
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.location_kind_terms
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.locations
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.storage_roots
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.project_catalog
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.project_locators
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.package_observations
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.library_media
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.library_change_batches
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara.library_changes
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara_private.media_objects
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara_private.library_streams
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara_private.mutation_receipts
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER a_library_lock BEFORE INSERT OR UPDATE OR DELETE ON photara_private.sync_clients
  FOR EACH ROW EXECUTE FUNCTION photara_private.lock_library();
CREATE TRIGGER c_merge BEFORE INSERT OR UPDATE ON photara.people
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_merge('person_id');
CREATE TRIGGER c_merge BEFORE INSERT OR UPDATE ON photara.organizations
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_merge('organization_id');
CREATE TRIGGER c_merge BEFORE INSERT OR UPDATE ON photara.location_kinds
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_merge('location_kind_id');
CREATE TRIGGER c_merge BEFORE INSERT OR UPDATE ON photara.locations
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_merge('location_id');
CREATE TRIGGER c_retirement BEFORE UPDATE ON photara.people
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_retirement();
CREATE TRIGGER c_retirement BEFORE UPDATE ON photara.organizations
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_retirement();
CREATE TRIGGER c_retirement BEFORE UPDATE ON photara.location_kinds
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_retirement();
CREATE TRIGGER c_retirement BEFORE UPDATE ON photara.storage_roots
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_retirement();
CREATE TRIGGER c_kind_term BEFORE INSERT OR UPDATE OR DELETE ON photara.location_kind_terms
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_kind_term();
CREATE TRIGGER c_kind_history BEFORE UPDATE ON photara.location_kinds
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_kind_history();
CREATE TRIGGER c_location BEFORE INSERT OR UPDATE ON photara.locations
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_location();
CREATE TRIGGER c_relationship BEFORE INSERT OR UPDATE ON photara.person_organization_relationships
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_relationship();
CREATE TRIGGER c_locator BEFORE INSERT OR UPDATE ON photara.project_locators
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_locator();
CREATE CONSTRAINT TRIGGER z_library_owner AFTER INSERT OR UPDATE ON photara.libraries
  DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.guard_owner();
CREATE CONSTRAINT TRIGGER z_membership_owner AFTER INSERT OR UPDATE OR DELETE ON photara_identity.memberships
  DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.guard_owner();
CREATE TRIGGER c_account_disable BEFORE UPDATE ON photara_identity.accounts
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_account_disable();
CREATE TRIGGER c_stream BEFORE INSERT OR UPDATE ON photara_private.library_streams
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_stream();
CREATE CONSTRAINT TRIGGER z_stream_commit AFTER INSERT OR UPDATE ON photara_private.library_streams
  DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.guard_stream_commit();
CREATE CONSTRAINT TRIGGER z_batch_complete AFTER INSERT ON photara.library_change_batches
  DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.guard_batch();
CREATE TRIGGER c_sync_ack BEFORE INSERT OR UPDATE ON photara_private.sync_clients
  FOR EACH ROW EXECUTE FUNCTION photara_private.guard_sync_ack();
CREATE TRIGGER immutable_record BEFORE UPDATE OR DELETE ON photara.normalization_policies
  FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER immutable_record BEFORE UPDATE OR DELETE ON photara.library_media
  FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER immutable_record BEFORE UPDATE OR DELETE ON photara.package_observations
  FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER immutable_record BEFORE UPDATE OR DELETE ON photara.library_change_batches
  FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER immutable_record BEFORE UPDATE OR DELETE ON photara.library_changes
  FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER immutable_record BEFORE UPDATE OR DELETE ON photara_private.billing_events
  FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER immutable_record BEFORE UPDATE OR DELETE ON photara_private.mutation_receipts
  FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER immutable_record BEFORE UPDATE OR DELETE ON photara_private.security_audit
  FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
