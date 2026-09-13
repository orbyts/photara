CREATE TRIGGER libraries_revision_insert BEFORE INSERT ON libraries
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER libraries_revision_update BEFORE UPDATE ON libraries
WHEN NEW.library_id IS NOT OLD.library_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER libraries_no_delete BEFORE DELETE ON libraries
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER social_profiles_revision_insert BEFORE INSERT ON social_profiles
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER social_profiles_revision_update BEFORE UPDATE ON social_profiles
WHEN NEW.social_profile_id IS NOT OLD.social_profile_id OR NEW.library_id IS NOT OLD.library_id
 OR NEW.person_id IS NOT OLD.person_id OR NEW.organization_id IS NOT OLD.organization_id
 OR NEW.provider_id<>OLD.provider_id OR NEW.created_at_ms<>OLD.created_at_ms
 OR NEW.local_revision<>OLD.local_revision+1
 OR (OLD.provider_subject_id IS NOT NULL AND
     (NEW.provider_subject_id IS NOT OLD.provider_subject_id OR NEW.subject_namespace IS NOT OLD.subject_namespace))
 OR (OLD.state='tombstoned' AND NEW.state<>'tombstoned')
BEGIN SELECT RAISE(ABORT,'social_identity_or_revision_conflict'); END;
CREATE TRIGGER social_profiles_no_delete BEFORE DELETE ON social_profiles
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER social_profiles_owner_insert BEFORE INSERT ON social_profiles
WHEN NEW.state='active'
BEGIN
 SELECT RAISE(ABORT,'social_owner_not_active') WHERE
  (NEW.person_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM people WHERE library_id=NEW.library_id AND person_id=NEW.person_id AND state='active'))
  OR (NEW.organization_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM organizations WHERE library_id=NEW.library_id AND organization_id=NEW.organization_id AND state='active'));
END;
CREATE TRIGGER social_profiles_owner_update BEFORE UPDATE ON social_profiles
WHEN NEW.state='active'
BEGIN
 SELECT RAISE(ABORT,'social_owner_not_active') WHERE
  (NEW.person_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM people WHERE library_id=NEW.library_id AND person_id=NEW.person_id AND state='active'))
  OR (NEW.organization_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM organizations WHERE library_id=NEW.library_id AND organization_id=NEW.organization_id AND state='active'));
END;
CREATE TRIGGER people_live_social_profiles BEFORE UPDATE OF state ON people
WHEN NEW.state<>'active' AND EXISTS (SELECT 1 FROM social_profiles
 WHERE library_id=OLD.library_id AND person_id=OLD.person_id AND state='active')
BEGIN SELECT RAISE(ABORT,'party_has_live_social_profiles'); END;
CREATE TRIGGER organizations_live_social_profiles BEFORE UPDATE OF state ON organizations
WHEN NEW.state<>'active' AND EXISTS (SELECT 1 FROM social_profiles
 WHERE library_id=OLD.library_id AND organization_id=OLD.organization_id AND state='active')
BEGIN SELECT RAISE(ABORT,'party_has_live_social_profiles'); END;

CREATE TRIGGER people_revision_insert BEFORE INSERT ON people
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER people_revision_update BEFORE UPDATE ON people
WHEN NEW.person_id IS NOT OLD.person_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER people_no_delete BEFORE DELETE ON people
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER organizations_revision_insert BEFORE INSERT ON organizations
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER organizations_revision_update BEFORE UPDATE ON organizations
WHEN NEW.organization_id IS NOT OLD.organization_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER organizations_no_delete BEFORE DELETE ON organizations
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER location_kinds_revision_insert BEFORE INSERT ON location_kinds
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER location_kinds_revision_update BEFORE UPDATE ON location_kinds
WHEN NEW.location_kind_id IS NOT OLD.location_kind_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER location_kinds_no_delete BEFORE DELETE ON location_kinds
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER locations_revision_insert BEFORE INSERT ON locations
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER locations_revision_update BEFORE UPDATE ON locations
WHEN NEW.location_id IS NOT OLD.location_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER locations_no_delete BEFORE DELETE ON locations
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER person_organization_relationships_revision_insert BEFORE INSERT ON person_organization_relationships
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER person_organization_relationships_revision_update BEFORE UPDATE ON person_organization_relationships
WHEN NEW.relationship_id IS NOT OLD.relationship_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER person_organization_relationships_no_delete BEFORE DELETE ON person_organization_relationships
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER storage_roots_revision_insert BEFORE INSERT ON storage_roots
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER storage_roots_revision_update BEFORE UPDATE ON storage_roots
WHEN NEW.storage_root_id IS NOT OLD.storage_root_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER storage_roots_no_delete BEFORE DELETE ON storage_roots
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER project_catalog_revision_insert BEFORE INSERT ON project_catalog
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER project_catalog_revision_update BEFORE UPDATE ON project_catalog
WHEN NEW.project_id IS NOT OLD.project_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER project_catalog_no_delete BEFORE DELETE ON project_catalog
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER project_locators_revision_insert BEFORE INSERT ON project_locators
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER project_locators_revision_update BEFORE UPDATE ON project_locators
WHEN NEW.locator_id IS NOT OLD.locator_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER project_locators_no_delete BEFORE DELETE ON project_locators
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER operation_intents_revision_insert BEFORE INSERT ON operation_intents
WHEN NEW.local_revision<>1
BEGIN SELECT RAISE(ABORT,'revision_must_start_at_one'); END;
CREATE TRIGGER operation_intents_revision_update BEFORE UPDATE ON operation_intents
WHEN NEW.operation_id IS NOT OLD.operation_id OR NEW.library_id IS NOT OLD.library_id
  OR NEW.created_at_ms<>OLD.created_at_ms OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'identity_or_revision_conflict'); END;
CREATE TRIGGER operation_intents_no_delete BEFORE DELETE ON operation_intents
BEGIN SELECT RAISE(ABORT,'hard_delete_not_supported'); END;
CREATE TRIGGER people_merge_guard BEFORE UPDATE OF merged_into_id,state ON people
WHEN NEW.merged_into_id IS NOT NULL AND NEW.merged_into_id IS NOT OLD.merged_into_id
BEGIN
  SELECT RAISE(ABORT,'merge_target_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM people WHERE library_id=NEW.library_id
     AND person_id=NEW.merged_into_id AND state='active');
  SELECT RAISE(ABORT,'merge_cycle') WHERE EXISTS (
    WITH RECURSIVE chain(id) AS (
      SELECT NEW.merged_into_id
      UNION
      SELECT t.merged_into_id FROM people t JOIN chain c ON t.person_id=c.id
      WHERE t.library_id=NEW.library_id AND t.merged_into_id IS NOT NULL
    ) SELECT 1 FROM chain WHERE id=NEW.person_id
  );
END;
CREATE TRIGGER people_terminal_identity BEFORE UPDATE ON people
WHEN (OLD.state='merged' AND
      (NEW.state<>'merged' OR NEW.merged_into_id IS NOT OLD.merged_into_id))
  OR (OLD.state='tombstoned' AND NEW.state<>'tombstoned')
BEGIN SELECT RAISE(ABORT,'retired_identity_requires_explicit_restore'); END;
CREATE TRIGGER organizations_merge_guard BEFORE UPDATE OF merged_into_id,state ON organizations
WHEN NEW.merged_into_id IS NOT NULL AND NEW.merged_into_id IS NOT OLD.merged_into_id
BEGIN
  SELECT RAISE(ABORT,'merge_target_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM organizations WHERE library_id=NEW.library_id
     AND organization_id=NEW.merged_into_id AND state='active');
  SELECT RAISE(ABORT,'merge_cycle') WHERE EXISTS (
    WITH RECURSIVE chain(id) AS (
      SELECT NEW.merged_into_id
      UNION
      SELECT t.merged_into_id FROM organizations t JOIN chain c ON t.organization_id=c.id
      WHERE t.library_id=NEW.library_id AND t.merged_into_id IS NOT NULL
    ) SELECT 1 FROM chain WHERE id=NEW.organization_id
  );
END;
CREATE TRIGGER organizations_terminal_identity BEFORE UPDATE ON organizations
WHEN (OLD.state='merged' AND
      (NEW.state<>'merged' OR NEW.merged_into_id IS NOT OLD.merged_into_id))
  OR (OLD.state='tombstoned' AND NEW.state<>'tombstoned')
BEGIN SELECT RAISE(ABORT,'retired_identity_requires_explicit_restore'); END;
CREATE TRIGGER location_kinds_merge_guard BEFORE UPDATE OF merged_into_id,state ON location_kinds
WHEN NEW.merged_into_id IS NOT NULL AND NEW.merged_into_id IS NOT OLD.merged_into_id
BEGIN
  SELECT RAISE(ABORT,'merge_target_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM location_kinds WHERE library_id=NEW.library_id
     AND location_kind_id=NEW.merged_into_id AND state='active');
  SELECT RAISE(ABORT,'merge_cycle') WHERE EXISTS (
    WITH RECURSIVE chain(id) AS (
      SELECT NEW.merged_into_id
      UNION
      SELECT t.merged_into_id FROM location_kinds t JOIN chain c ON t.location_kind_id=c.id
      WHERE t.library_id=NEW.library_id AND t.merged_into_id IS NOT NULL
    ) SELECT 1 FROM chain WHERE id=NEW.location_kind_id
  );
END;
CREATE TRIGGER location_kinds_terminal_identity BEFORE UPDATE ON location_kinds
WHEN (OLD.state='merged' AND
      (NEW.state<>'merged' OR NEW.merged_into_id IS NOT OLD.merged_into_id))
  OR (OLD.state='tombstoned' AND NEW.state<>'tombstoned')
BEGIN SELECT RAISE(ABORT,'retired_identity_requires_explicit_restore'); END;
CREATE TRIGGER locations_merge_guard BEFORE UPDATE OF merged_into_id,state ON locations
WHEN NEW.merged_into_id IS NOT NULL AND NEW.merged_into_id IS NOT OLD.merged_into_id
BEGIN
  SELECT RAISE(ABORT,'merge_target_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM locations WHERE library_id=NEW.library_id
     AND location_id=NEW.merged_into_id AND state='active');
  SELECT RAISE(ABORT,'merge_cycle') WHERE EXISTS (
    WITH RECURSIVE chain(id) AS (
      SELECT NEW.merged_into_id
      UNION
      SELECT t.merged_into_id FROM locations t JOIN chain c ON t.location_id=c.id
      WHERE t.library_id=NEW.library_id AND t.merged_into_id IS NOT NULL
    ) SELECT 1 FROM chain WHERE id=NEW.location_id
  );
END;
CREATE TRIGGER locations_terminal_identity BEFORE UPDATE ON locations
WHEN (OLD.state='merged' AND
      (NEW.state<>'merged' OR NEW.merged_into_id IS NOT OLD.merged_into_id))
  OR (OLD.state='tombstoned' AND NEW.state<>'tombstoned')
BEGIN SELECT RAISE(ABORT,'retired_identity_requires_explicit_restore'); END;
CREATE TRIGGER location_kind_terms_policy_insert BEFORE INSERT ON location_kind_terms
WHEN NOT EXISTS (SELECT 1 FROM libraries WHERE library_id=NEW.library_id
  AND term_policy_version=NEW.policy_version)
BEGIN SELECT RAISE(ABORT,'term_policy_mismatch'); END;
CREATE TRIGGER location_kind_terms_update_guard BEFORE UPDATE ON location_kind_terms
WHEN NEW.library_id IS NOT OLD.library_id OR NEW.term_key<>OLD.term_key
  OR NEW.policy_version<>OLD.policy_version
BEGIN SELECT RAISE(ABORT,'term_claim_identity_is_immutable'); END;
CREATE TRIGGER location_kind_terms_transfer_guard BEFORE UPDATE OF location_kind_id ON location_kind_terms
WHEN NEW.location_kind_id IS NOT OLD.location_kind_id AND NOT EXISTS
 (SELECT 1 FROM location_kinds source JOIN location_kinds target
   ON target.library_id=source.library_id AND target.location_kind_id=NEW.location_kind_id
  WHERE source.library_id=OLD.library_id AND source.location_kind_id=OLD.location_kind_id
    AND source.state='merged' AND source.merged_into_id=NEW.location_kind_id AND target.state='active')
BEGIN SELECT RAISE(ABORT,'term_transfer_requires_merge'); END;
CREATE TRIGGER location_kinds_retirement_history BEFORE UPDATE ON location_kinds
WHEN OLD.state<>'active' AND
 (NEW.retirement_terms_json IS NOT OLD.retirement_terms_json
  OR NEW.canonical_key<>OLD.canonical_key OR NEW.canonical_display<>OLD.canonical_display)
BEGIN SELECT RAISE(ABORT,'retirement_provenance_is_immutable'); END;
CREATE TRIGGER location_kind_terms_no_delete BEFORE DELETE ON location_kind_terms
BEGIN SELECT RAISE(ABORT,'term_claim_is_reserved'); END;

CREATE TRIGGER location_kinds_live_locations BEFORE UPDATE OF state ON location_kinds
WHEN NEW.state<>'active' AND EXISTS
 (SELECT 1 FROM locations WHERE library_id=OLD.library_id
  AND location_kind_id=OLD.location_kind_id AND state='active')
BEGIN SELECT RAISE(ABORT,'kind_has_live_locations'); END;
CREATE TRIGGER locations_live_children BEFORE UPDATE OF state ON locations
WHEN NEW.state<>'active' AND EXISTS
 (SELECT 1 FROM locations WHERE library_id=OLD.library_id
  AND parent_location_id=OLD.location_id AND state='active')
BEGIN SELECT RAISE(ABORT,'location_has_live_children'); END;
CREATE TRIGGER locations_active_targets_insert BEFORE INSERT ON locations
WHEN NEW.state='active'
BEGIN
  SELECT RAISE(ABORT,'location_kind_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM location_kinds WHERE library_id=NEW.library_id
     AND location_kind_id=NEW.location_kind_id AND state='active');
  SELECT RAISE(ABORT,'location_parent_not_active') WHERE NEW.parent_location_id IS NOT NULL
    AND NOT EXISTS (SELECT 1 FROM locations WHERE library_id=NEW.library_id
     AND location_id=NEW.parent_location_id AND state='active');
END;
CREATE TRIGGER locations_parent_cycle_insert BEFORE INSERT ON locations
WHEN NEW.parent_location_id IS NOT NULL
BEGIN
  SELECT RAISE(ABORT,'location_parent_cycle') WHERE EXISTS (
    WITH RECURSIVE ancestors(id) AS (
      SELECT NEW.parent_location_id
      UNION
      SELECT l.parent_location_id FROM locations l JOIN ancestors a ON l.location_id=a.id
      WHERE l.library_id=NEW.library_id AND l.parent_location_id IS NOT NULL
    ) SELECT 1 FROM ancestors WHERE id=NEW.location_id
  );
END;

CREATE TRIGGER relationships_valid_insert BEFORE INSERT ON person_organization_relationships
WHEN NEW.state='active'
BEGIN
  SELECT RAISE(ABORT,'relationship_party_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM people WHERE library_id=NEW.library_id AND person_id=NEW.person_id AND state='active')
    OR NOT EXISTS
    (SELECT 1 FROM organizations WHERE library_id=NEW.library_id AND organization_id=NEW.organization_id AND state='active');
  SELECT RAISE(ABORT,'relationship_interval_overlap') WHERE EXISTS
    (SELECT 1 FROM person_organization_relationships r
     WHERE r.library_id=NEW.library_id AND r.person_id=NEW.person_id
       AND r.organization_id=NEW.organization_id AND r.relationship_type=NEW.relationship_type
       AND r.relationship_id<>NEW.relationship_id AND r.state='active'
       AND (r.valid_until_ms IS NULL OR NEW.valid_from_ms IS NULL OR r.valid_until_ms>NEW.valid_from_ms)
       AND (NEW.valid_until_ms IS NULL OR r.valid_from_ms IS NULL OR NEW.valid_until_ms>r.valid_from_ms));
END;
CREATE TRIGGER locations_active_targets_update BEFORE UPDATE ON locations
WHEN NEW.state='active'
BEGIN
  SELECT RAISE(ABORT,'location_kind_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM location_kinds WHERE library_id=NEW.library_id
     AND location_kind_id=NEW.location_kind_id AND state='active');
  SELECT RAISE(ABORT,'location_parent_not_active') WHERE NEW.parent_location_id IS NOT NULL
    AND NOT EXISTS (SELECT 1 FROM locations WHERE library_id=NEW.library_id
     AND location_id=NEW.parent_location_id AND state='active');
END;
CREATE TRIGGER locations_parent_cycle_update BEFORE UPDATE ON locations
WHEN NEW.parent_location_id IS NOT NULL
BEGIN
  SELECT RAISE(ABORT,'location_parent_cycle') WHERE EXISTS (
    WITH RECURSIVE ancestors(id) AS (
      SELECT NEW.parent_location_id
      UNION
      SELECT l.parent_location_id FROM locations l JOIN ancestors a ON l.location_id=a.id
      WHERE l.library_id=NEW.library_id AND l.parent_location_id IS NOT NULL
    ) SELECT 1 FROM ancestors WHERE id=NEW.location_id
  );
END;

CREATE TRIGGER relationships_valid_update BEFORE UPDATE ON person_organization_relationships
WHEN NEW.state='active'
BEGIN
  SELECT RAISE(ABORT,'relationship_party_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM people WHERE library_id=NEW.library_id AND person_id=NEW.person_id AND state='active')
    OR NOT EXISTS
    (SELECT 1 FROM organizations WHERE library_id=NEW.library_id AND organization_id=NEW.organization_id AND state='active');
  SELECT RAISE(ABORT,'relationship_interval_overlap') WHERE EXISTS
    (SELECT 1 FROM person_organization_relationships r
     WHERE r.library_id=NEW.library_id AND r.person_id=NEW.person_id
       AND r.organization_id=NEW.organization_id AND r.relationship_type=NEW.relationship_type
       AND r.relationship_id<>NEW.relationship_id AND r.state='active'
       AND (r.valid_until_ms IS NULL OR NEW.valid_from_ms IS NULL OR r.valid_until_ms>NEW.valid_from_ms)
       AND (NEW.valid_until_ms IS NULL OR r.valid_from_ms IS NULL OR NEW.valid_until_ms>r.valid_from_ms));
END;
CREATE TRIGGER people_live_relationships BEFORE UPDATE OF state ON people
WHEN NEW.state<>'active' AND EXISTS
 (SELECT 1 FROM person_organization_relationships WHERE library_id=OLD.library_id
  AND person_id=OLD.person_id AND state='active')
BEGIN SELECT RAISE(ABORT,'party_has_live_relationships'); END;
CREATE TRIGGER organizations_live_relationships BEFORE UPDATE OF state ON organizations
WHEN NEW.state<>'active' AND EXISTS
 (SELECT 1 FROM person_organization_relationships WHERE library_id=OLD.library_id
  AND organization_id=OLD.organization_id AND state='active')
BEGIN SELECT RAISE(ABORT,'party_has_live_relationships'); END;
CREATE TRIGGER storage_roots_live_locators BEFORE UPDATE OF state ON storage_roots
WHEN NEW.state<>'active' AND EXISTS
 (SELECT 1 FROM project_locators WHERE library_id=OLD.library_id
  AND storage_root_id=OLD.storage_root_id AND state='active')
BEGIN SELECT RAISE(ABORT,'root_has_live_locators'); END;
CREATE TRIGGER project_locators_root_insert BEFORE INSERT ON project_locators
WHEN NEW.state='active' AND NEW.storage_root_id IS NOT NULL AND NOT EXISTS
 (SELECT 1 FROM storage_roots WHERE library_id=NEW.library_id
  AND storage_root_id=NEW.storage_root_id AND state='active')
BEGIN SELECT RAISE(ABORT,'locator_root_not_active'); END;
CREATE TRIGGER project_catalog_selection_insert BEFORE INSERT ON project_catalog
WHEN NEW.active_locator_id IS NOT NULL AND NOT EXISTS
 (SELECT 1 FROM project_locators WHERE library_id=NEW.library_id
  AND project_id=NEW.project_id AND locator_id=NEW.active_locator_id AND state='active')
BEGIN SELECT RAISE(ABORT,'catalog_locator_not_active'); END;
CREATE TRIGGER project_locators_root_update BEFORE UPDATE ON project_locators
WHEN NEW.state='active' AND NEW.storage_root_id IS NOT NULL AND NOT EXISTS
 (SELECT 1 FROM storage_roots WHERE library_id=NEW.library_id
  AND storage_root_id=NEW.storage_root_id AND state='active')
BEGIN SELECT RAISE(ABORT,'locator_root_not_active'); END;
CREATE TRIGGER project_catalog_selection_update BEFORE UPDATE ON project_catalog
WHEN NEW.active_locator_id IS NOT NULL AND NOT EXISTS
 (SELECT 1 FROM project_locators WHERE library_id=NEW.library_id
  AND project_id=NEW.project_id AND locator_id=NEW.active_locator_id AND state='active')
BEGIN SELECT RAISE(ABORT,'catalog_locator_not_active'); END;
CREATE TRIGGER project_locators_selected_retirement BEFORE UPDATE OF state ON project_locators
WHEN NEW.state='retired' AND EXISTS
 (SELECT 1 FROM project_catalog WHERE library_id=OLD.library_id
  AND project_id=OLD.project_id AND active_locator_id=OLD.locator_id)
BEGIN SELECT RAISE(ABORT,'clear_catalog_selection_first'); END;

CREATE TRIGGER remote_receipt_request BEFORE INSERT ON remote_mutation_receipts
WHEN NOT EXISTS (SELECT 1 FROM sync_outbox WHERE target_id=NEW.target_id
  AND library_id=NEW.library_id AND mutation_id=NEW.mutation_id AND request_sha256=NEW.request_sha256)
BEGIN SELECT RAISE(ABORT,'receipt_request_mismatch'); END;
CREATE TRIGGER remote_receipt_no_update BEFORE UPDATE ON remote_mutation_receipts
BEGIN SELECT RAISE(ABORT,'append_only_receipt'); END;
CREATE TRIGGER remote_receipt_no_delete BEFORE DELETE ON remote_mutation_receipts
BEGIN SELECT RAISE(ABORT,'append_only_receipt'); END;
CREATE TRIGGER disposition_sealed_guard BEFORE INSERT ON local_mutation_dispositions
WHEN EXISTS (SELECT 1 FROM sync_outbox o WHERE o.library_id=NEW.library_id
  AND o.mutation_id=NEW.mutation_id AND o.request_json IS NOT NULL AND NOT EXISTS
    (SELECT 1 FROM remote_mutation_receipts r WHERE r.target_id=o.target_id
     AND r.mutation_id=o.mutation_id AND r.outcome IN ('rejected','conflict')))
BEGIN SELECT RAISE(ABORT,'settle_sealed_request_before_disposition'); END;
CREATE TRIGGER disposition_no_update BEFORE UPDATE ON local_mutation_dispositions
BEGIN SELECT RAISE(ABORT,'append_only_disposition'); END;
CREATE TRIGGER disposition_no_delete BEFORE DELETE ON local_mutation_dispositions
BEGIN SELECT RAISE(ABORT,'append_only_disposition'); END;
CREATE TRIGGER snapshot_payload_immutable BEFORE UPDATE ON sync_snapshot_installs
WHEN NEW.installation_id IS NOT OLD.installation_id OR NEW.target_id IS NOT OLD.target_id
 OR NEW.library_id IS NOT OLD.library_id OR NEW.server_snapshot_id IS NOT OLD.server_snapshot_id
 OR NEW.stream_epoch IS NOT OLD.stream_epoch OR NEW.high_water_cursor<>OLD.high_water_cursor
 OR NEW.payload_json<>OLD.payload_json
 OR NEW.payload_sha256 IS NOT OLD.payload_sha256 OR NEW.created_at_ms<>OLD.created_at_ms
 OR (OLD.state='installed' AND (NEW.state<>'installed'
   OR NEW.expected_local_cursor IS NOT OLD.expected_local_cursor OR NEW.installed_at_ms IS NOT OLD.installed_at_ms))
BEGIN SELECT RAISE(ABORT,'snapshot_payload_is_immutable'); END;
CREATE TRIGGER sync_outbox_request_immutable BEFORE UPDATE ON sync_outbox
WHEN NEW.target_id IS NOT OLD.target_id OR NEW.library_id IS NOT OLD.library_id
 OR NEW.mutation_id IS NOT OLD.mutation_id OR NEW.sequence<>OLD.sequence
 OR (OLD.request_json IS NOT NULL AND
    (NEW.request_json IS NOT OLD.request_json OR NEW.request_sha256 IS NOT OLD.request_sha256))
 OR (OLD.state='acknowledged' AND NEW.state<>'acknowledged')
 OR (NEW.state IN ('superseded','discarded') AND NEW.request_json IS NOT NULL)
 OR (OLD.state IN ('superseded','discarded') AND NEW.state<>OLD.state)
BEGIN SELECT RAISE(ABORT,'sealed_request_is_immutable'); END;

CREATE TRIGGER sync_inbox_payload_immutable BEFORE UPDATE ON sync_inbox
WHEN NEW.batch_id IS NOT OLD.batch_id OR NEW.target_id IS NOT OLD.target_id
 OR NEW.library_id IS NOT OLD.library_id OR NEW.cursor_before IS NOT OLD.cursor_before
 OR NEW.stream_epoch IS NOT OLD.stream_epoch OR NEW.batch_sequence<>OLD.batch_sequence
 OR NEW.cursor_after<>OLD.cursor_after OR NEW.payload_json<>OLD.payload_json
 OR NEW.payload_sha256 IS NOT OLD.payload_sha256 OR NEW.received_at_ms<>OLD.received_at_ms
 OR (OLD.state='applied' AND NEW.state<>'applied')
BEGIN SELECT RAISE(ABORT,'received_page_is_immutable'); END;

CREATE TRIGGER operation_intents_request_immutable BEFORE UPDATE ON operation_intents
WHEN NEW.operation_kind<>OLD.operation_kind OR NEW.idempotency_key<>OLD.idempotency_key
 OR NEW.project_id IS NOT OLD.project_id OR NEW.request_json<>OLD.request_json
 OR NEW.request_sha256 IS NOT OLD.request_sha256 OR NEW.base_commit_id IS NOT OLD.base_commit_id
 OR NEW.base_commit_sha256 IS NOT OLD.base_commit_sha256
 OR (OLD.proposed_commit_id IS NOT NULL AND
    (NEW.proposed_commit_id IS NOT OLD.proposed_commit_id
     OR NEW.proposed_commit_sha256 IS NOT OLD.proposed_commit_sha256))
 OR (OLD.state='committed' AND NEW.state<>'committed')
BEGIN SELECT RAISE(ABORT,'recovery_request_is_immutable'); END;
CREATE TRIGGER normalization_policies_immutable_update BEFORE UPDATE ON normalization_policies
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER normalization_policies_immutable_delete BEFORE DELETE ON normalization_policies
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER mutations_immutable_update BEFORE UPDATE ON mutations
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER mutations_immutable_delete BEFORE DELETE ON mutations
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER mutation_baselines_immutable_update BEFORE UPDATE ON mutation_baselines
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER mutation_baselines_immutable_delete BEFORE DELETE ON mutation_baselines
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER local_changes_immutable_update BEFORE UPDATE ON local_changes
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER local_changes_immutable_delete BEFORE DELETE ON local_changes
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER operation_events_immutable_update BEFORE UPDATE ON operation_events
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER operation_events_immutable_delete BEFORE DELETE ON operation_events
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER recovery_items_immutable_update BEFORE UPDATE ON recovery_items
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER recovery_items_immutable_delete BEFORE DELETE ON recovery_items
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER recovery_publications_immutable_update BEFORE UPDATE ON recovery_publications
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER recovery_publications_immutable_delete BEFORE DELETE ON recovery_publications
BEGIN SELECT RAISE(ABORT,'append_only_record'); END;
CREATE TRIGGER project_observations_immutable_update BEFORE UPDATE ON project_observations
BEGIN SELECT RAISE(ABORT,'replace_observation_not_contents'); END;
CREATE TRIGGER people_merge_insert BEFORE INSERT ON people
WHEN NEW.merged_into_id IS NOT NULL
BEGIN
  SELECT RAISE(ABORT,'merge_target_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM people WHERE library_id=NEW.library_id
     AND person_id=NEW.merged_into_id AND state='active');
END;
CREATE TRIGGER organizations_merge_insert BEFORE INSERT ON organizations
WHEN NEW.merged_into_id IS NOT NULL
BEGIN
  SELECT RAISE(ABORT,'merge_target_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM organizations WHERE library_id=NEW.library_id
     AND organization_id=NEW.merged_into_id AND state='active');
END;
CREATE TRIGGER location_kinds_merge_insert BEFORE INSERT ON location_kinds
WHEN NEW.merged_into_id IS NOT NULL
BEGIN
  SELECT RAISE(ABORT,'merge_target_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM location_kinds WHERE library_id=NEW.library_id
     AND location_kind_id=NEW.merged_into_id AND state='active');
END;
CREATE TRIGGER locations_merge_insert BEFORE INSERT ON locations
WHEN NEW.merged_into_id IS NOT NULL
BEGIN
  SELECT RAISE(ABORT,'merge_target_not_active') WHERE NOT EXISTS
    (SELECT 1 FROM locations WHERE library_id=NEW.library_id
     AND location_id=NEW.merged_into_id AND state='active');
END;
