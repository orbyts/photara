-- CXT3c executable service migration; promoted from S4.
CREATE FUNCTION photara_private.request_library() RETURNS uuid
LANGUAGE sql STABLE SET search_path=pg_catalog,pg_temp AS $$
  SELECT nullif(current_setting('photara.library_id',true),'')::uuid;
$$;

CREATE FUNCTION photara_private.authorize_library(p_library uuid,p_action text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE actor_id uuid; identity_id uuid; roles text[]; library_state text;
BEGIN
  IF p_library IS DISTINCT FROM photara_private.request_library()
  THEN RAISE EXCEPTION 'library_scope_mismatch' USING ERRCODE='42501'; END IF;
  actor_id:=nullif(current_setting('photara.account_id',true),'')::uuid;
  identity_id:=nullif(current_setting('photara.identity_id',true),'')::uuid;
  CASE p_action
    WHEN 'read' THEN roles:=ARRAY['owner','admin','editor','viewer'];
    WHEN 'write' THEN roles:=ARRAY['owner','admin','editor'];
    WHEN 'admin' THEN roles:=ARRAY['owner','admin'];
    WHEN 'owner' THEN roles:=ARRAY['owner'];
    ELSE RAISE EXCEPTION 'unknown_authorization_action' USING ERRCODE='42501';
  END CASE;
  PERFORM 1 FROM photara_identity.accounts a
    WHERE a.account_id=actor_id AND a.state='active' FOR SHARE;
  IF NOT FOUND THEN RAISE EXCEPTION 'account_unavailable' USING ERRCODE='42501'; END IF;
  PERFORM 1 FROM photara_identity.account_identities i
    WHERE i.identity_id=identity_id AND i.account_id=actor_id AND i.state='active' FOR SHARE;
  IF NOT FOUND THEN RAISE EXCEPTION 'identity_unavailable' USING ERRCODE='42501'; END IF;
  IF p_action='read' THEN
    SELECT state INTO library_state FROM photara.libraries WHERE library_id=p_library FOR SHARE;
  ELSE
    SELECT state INTO library_state FROM photara.libraries WHERE library_id=p_library FOR UPDATE;
  END IF;
  IF NOT FOUND OR (p_action<>'read' AND library_state<>'active')
    OR NOT EXISTS(SELECT 1 FROM photara_identity.memberships m
      WHERE m.library_id=p_library AND m.account_id=actor_id
        AND m.state='active' AND m.role=ANY(roles))
  THEN RAISE EXCEPTION 'library_access_denied' USING ERRCODE='42501'; END IF;
END;
$$;

CREATE FUNCTION photara_private.library_capability(p_key text)
RETURNS TABLE(enabled boolean,quota_limit bigint)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
  SELECT coalesce(bool_or(g.enabled),false),
         max(g.quota_limit) FILTER(WHERE g.enabled)
  FROM photara_private.library_entitlement_grants g
  WHERE g.library_id=photara_private.request_library()
    AND g.capability_key=p_key AND g.state='active'
    AND g.valid_from<=transaction_timestamp()
    AND (g.valid_until IS NULL OR g.valid_until>transaction_timestamp());
$$;

CREATE FUNCTION photara_private.developer_capability(p_key text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
  SELECT EXISTS(SELECT 1 FROM photara_private.account_developer_grants g
    WHERE g.account_id=nullif(current_setting('photara.account_id',true),'')::uuid
      AND g.capability_key=p_key AND g.state='active'
      AND g.valid_from<=transaction_timestamp()
      AND (g.valid_until IS NULL OR g.valid_until>transaction_timestamp()));
$$;

REVOKE ALL ON ALL TABLES IN SCHEMA photara,photara_identity,photara_private FROM PUBLIC;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA photara,photara_identity,photara_private FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE photara_owner IN SCHEMA photara,photara_identity,photara_private
  REVOKE EXECUTE ON FUNCTIONS FROM PUBLIC;

GRANT USAGE ON SCHEMA photara,photara_private TO photara_api,photara_control;
GRANT USAGE ON SCHEMA photara_identity TO photara_control,photara_auth_read;
GRANT SELECT ON photara_identity.accounts,photara_identity.account_identities,
  photara_identity.memberships,photara_identity.devices TO photara_auth_read;
GRANT SELECT,INSERT,UPDATE ON ALL TABLES IN SCHEMA photara_identity TO photara_control;

GRANT SELECT ON photara.schema_metadata,photara.normalization_policies,photara.libraries TO photara_api;
GRANT SELECT,INSERT,UPDATE ON photara.people,photara.organizations,photara.social_profiles,
  photara.person_organization_relationships,photara.location_kinds,photara.locations,
  photara.storage_roots,photara.project_catalog,photara.project_locators TO photara_api;
GRANT SELECT,INSERT,UPDATE,DELETE ON photara.person_capabilities,photara.person_labels,
  photara.organization_labels TO photara_api;
GRANT SELECT,INSERT,UPDATE ON photara.location_kind_terms TO photara_api;
GRANT SELECT,INSERT ON photara.library_media,photara.package_observations,
  photara.library_change_batches,photara.library_changes TO photara_api;
GRANT SELECT,UPDATE ON photara_private.library_streams TO photara_api;
GRANT SELECT,INSERT ON photara_private.mutation_receipts TO photara_api;
GRANT SELECT,INSERT,UPDATE ON photara_private.sync_clients TO photara_api;

GRANT SELECT,INSERT,UPDATE ON ALL TABLES IN SCHEMA photara,photara_private TO photara_control;
GRANT DELETE ON photara.person_capabilities,photara.person_labels,photara.organization_labels TO photara_control;
GRANT EXECUTE ON FUNCTION photara_private.request_library() TO photara_api,photara_control;
GRANT EXECUTE ON FUNCTION photara_private.authorize_library(uuid,text) TO photara_api,photara_control;
GRANT EXECUTE ON FUNCTION photara_private.library_capability(text) TO photara_api,photara_control;
GRANT EXECUTE ON FUNCTION photara_private.developer_capability(text) TO photara_api,photara_control;
ALTER TABLE photara.libraries ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.libraries FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.libraries
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara_private.library_subscriptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara_private.library_subscriptions FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara_private.library_subscriptions
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara_private.library_entitlement_grants ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara_private.library_entitlement_grants FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara_private.library_entitlement_grants
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.people ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.people FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.people
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.organizations FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.organizations
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.social_profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.social_profiles FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.social_profiles
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.person_capabilities ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.person_capabilities FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.person_capabilities
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.person_labels ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.person_labels FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.person_labels
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.organization_labels ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.organization_labels FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.organization_labels
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.person_organization_relationships ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.person_organization_relationships FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.person_organization_relationships
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.location_kinds ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.location_kinds FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.location_kinds
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.location_kind_terms ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.location_kind_terms FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.location_kind_terms
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.locations ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.locations FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.locations
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.storage_roots ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.storage_roots FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.storage_roots
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.project_catalog ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.project_catalog FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.project_catalog
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.project_locators ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.project_locators FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.project_locators
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.package_observations ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.package_observations FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.package_observations
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.library_media ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.library_media FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.library_media
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.library_change_batches ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.library_change_batches FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.library_change_batches
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara.library_changes ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara.library_changes FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara.library_changes
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara_private.media_objects ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara_private.media_objects FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara_private.media_objects
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara_private.library_streams ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara_private.library_streams FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara_private.library_streams
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara_private.mutation_receipts ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara_private.mutation_receipts FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara_private.mutation_receipts
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara_private.sync_clients ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara_private.sync_clients FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara_private.sync_clients
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
ALTER TABLE photara_private.media_upload_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE photara_private.media_upload_sessions FORCE ROW LEVEL SECURITY;
CREATE POLICY library_scope ON photara_private.media_upload_sessions
  TO photara_api,photara_control,photara_owner
  USING (library_id=photara_private.request_library())
  WITH CHECK (library_id=photara_private.request_library());
