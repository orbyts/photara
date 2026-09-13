-- CXT3b executable local migration, promoted from accepted D19 CXT2.
CREATE TRIGGER d19_library_contract_state_no_delete
BEFORE DELETE ON library_contract_state
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_library_contract_state_update
BEFORE UPDATE ON library_contract_state
WHEN NEW.library_id IS NOT OLD.library_id OR NEW.contract_version IS NOT OLD.contract_version OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_project_ownership_no_delete
BEFORE DELETE ON project_ownership
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_project_ownership_update
BEFORE UPDATE ON project_ownership
WHEN NEW.project_id IS NOT OLD.project_id OR NEW.library_id IS NOT OLD.library_id OR NEW.source_origin_library_id IS NOT OLD.source_origin_library_id OR NEW.source_format IS NOT OLD.source_format OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_project_access_policies_no_delete
BEFORE DELETE ON project_access_policies
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_project_access_policies_update
BEFORE UPDATE ON project_access_policies
WHEN NEW.library_id IS NOT OLD.library_id OR NEW.project_id IS NOT OLD.project_id OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_project_access_grants_no_delete
BEFORE DELETE ON project_access_grants
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_project_access_grants_update
BEFORE UPDATE ON project_access_grants
WHEN NEW.grant_id IS NOT OLD.grant_id OR NEW.library_id IS NOT OLD.library_id OR NEW.project_id IS NOT OLD.project_id OR NEW.account_id IS NOT OLD.account_id OR NEW.local_principal_id IS NOT OLD.local_principal_id OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_project_invitations_no_delete
BEFORE DELETE ON project_invitations
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_project_invitations_update
BEFORE UPDATE ON project_invitations
WHEN NEW.invitation_id IS NOT OLD.invitation_id OR NEW.library_id IS NOT OLD.library_id OR NEW.project_id IS NOT OLD.project_id OR NEW.inviter_account_id IS NOT OLD.inviter_account_id OR NEW.target_account_id IS NOT OLD.target_account_id OR NEW.action_mask IS NOT OLD.action_mask OR NEW.expected_policy_revision IS NOT OLD.expected_policy_revision OR NEW.expires_at IS NOT OLD.expires_at OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_storage_location_specs_no_delete
BEFORE DELETE ON storage_location_specs
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_storage_location_specs_update
BEFORE UPDATE ON storage_location_specs
WHEN NEW.library_id IS NOT OLD.library_id OR NEW.storage_root_id IS NOT OLD.storage_root_id OR NEW.storage_kind IS NOT OLD.storage_kind OR NEW.provider_id IS NOT OLD.provider_id OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_storage_slots_no_delete
BEFORE DELETE ON storage_slots
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_storage_slots_update
BEFORE UPDATE ON storage_slots
WHEN NEW.slot_id IS NOT OLD.slot_id OR NEW.library_id IS NOT OLD.library_id OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_storage_slots_no_resurrection
BEFORE UPDATE ON storage_slots
WHEN OLD.state='tombstoned' AND NEW.state<>'tombstoned'
BEGIN
  SELECT RAISE(ABORT,'d19_no_resurrection');
END;

CREATE TRIGGER d19_storage_slot_names_no_delete
BEFORE DELETE ON storage_slot_names
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_storage_slot_names_update
BEFORE UPDATE ON storage_slot_names
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_host_bindings_no_delete
BEFORE DELETE ON host_bindings
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_host_bindings_update
BEFORE UPDATE ON host_bindings
WHEN NEW.binding_id IS NOT OLD.binding_id OR NEW.device_id IS NOT OLD.device_id OR NEW.library_id IS NOT OLD.library_id OR NEW.storage_root_id IS NOT OLD.storage_root_id OR NEW.host_kind IS NOT OLD.host_kind OR NEW.binding_kind IS NOT OLD.binding_kind OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_host_binding_selections_update
BEFORE UPDATE ON host_binding_selections
WHEN NEW.device_id IS NOT OLD.device_id OR NEW.library_id IS NOT OLD.library_id OR NEW.storage_root_id IS NOT OLD.storage_root_id
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_legacy_external_resource_resolutions_no_delete
BEFORE DELETE ON legacy_external_resource_resolutions
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_legacy_external_resource_resolutions_update
BEFORE UPDATE ON legacy_external_resource_resolutions
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_library_variables_no_delete
BEFORE DELETE ON library_variables
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_library_variables_update
BEFORE UPDATE ON library_variables
WHEN NEW.variable_id IS NOT OLD.variable_id OR NEW.library_id IS NOT OLD.library_id OR NEW.namespace IS NOT OLD.namespace OR NEW.value_type_id IS NOT OLD.value_type_id OR NEW.value_type_version IS NOT OLD.value_type_version OR NEW.schema_id IS NOT OLD.schema_id OR NEW.schema_version IS NOT OLD.schema_version OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_library_variables_no_resurrection
BEFORE UPDATE ON library_variables
WHEN OLD.state='tombstoned' AND NEW.state<>'tombstoned'
BEGIN
  SELECT RAISE(ABORT,'d19_no_resurrection');
END;

CREATE TRIGGER d19_library_variable_values_no_delete
BEFORE DELETE ON library_variable_values
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_library_variable_values_update
BEFORE UPDATE ON library_variable_values
WHEN NEW.library_id IS NOT OLD.library_id OR NEW.variable_id IS NOT OLD.variable_id OR NEW.value_id IS NOT OLD.value_id
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_library_variable_names_no_delete
BEFORE DELETE ON library_variable_names
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_library_variable_names_update
BEFORE UPDATE ON library_variable_names
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_library_expressions_no_delete
BEFORE DELETE ON library_expressions
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_library_expressions_update
BEFORE UPDATE ON library_expressions
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_library_expression_dependencies_no_delete
BEFORE DELETE ON library_expression_dependencies
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_library_expression_dependencies_update
BEFORE UPDATE ON library_expression_dependencies
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_device_context_snapshots_no_delete
BEFORE DELETE ON device_context_snapshots
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_device_context_snapshots_update
BEFORE UPDATE ON device_context_snapshots
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_context_apply_intents_no_delete
BEFORE DELETE ON context_apply_intents
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_context_apply_intents_update
BEFORE UPDATE ON context_apply_intents
WHEN NEW.operation_id IS NOT OLD.operation_id OR NEW.library_id IS NOT OLD.library_id OR NEW.target_project_id IS NOT OLD.target_project_id OR NEW.proposal_id IS NOT OLD.proposal_id OR NEW.request_canonical IS NOT OLD.request_canonical OR NEW.request_sha256 IS NOT OLD.request_sha256 OR NEW.source_run_id IS NOT OLD.source_run_id OR NEW.source_attempt_id IS NOT OLD.source_attempt_id OR NEW.target_authority IS NOT OLD.target_authority OR NEW.expected_commit_id IS NOT OLD.expected_commit_id OR NEW.expected_commit_sha256 IS NOT OLD.expected_commit_sha256 OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_context_apply_receipts_no_delete
BEFORE DELETE ON context_apply_receipts
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_context_apply_receipts_update
BEFORE UPDATE ON context_apply_receipts
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_project_media_links_no_delete
BEFORE DELETE ON project_media_links
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_project_media_links_update
BEFORE UPDATE ON project_media_links
WHEN NEW.library_id IS NOT OLD.library_id OR NEW.project_id IS NOT OLD.project_id OR NEW.sha256 IS NOT OLD.sha256 OR NEW.purpose IS NOT OLD.purpose OR NEW.record_schema IS NOT OLD.record_schema OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_project_media_links_no_resurrection
BEFORE UPDATE ON project_media_links
WHEN OLD.state='tombstoned' AND NEW.state<>'tombstoned'
BEGIN
  SELECT RAISE(ABORT,'d19_no_resurrection');
END;

CREATE TRIGGER d19_scoped_sync_channels_no_delete
BEFORE DELETE ON scoped_sync_channels
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_scoped_sync_channels_update
BEFORE UPDATE ON scoped_sync_channels
WHEN NEW.channel_id IS NOT OLD.channel_id OR NEW.library_id IS NOT OLD.library_id OR NEW.account_id IS NOT OLD.account_id OR NEW.environment_id IS NOT OLD.environment_id OR NEW.scope_kind IS NOT OLD.scope_kind OR NEW.project_id IS NOT OLD.project_id
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_scoped_sync_operations_no_delete
BEFORE DELETE ON scoped_sync_operations
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_scoped_sync_operations_update
BEFORE UPDATE ON scoped_sync_operations
WHEN NEW.operation_id IS NOT OLD.operation_id OR NEW.channel_id IS NOT OLD.channel_id OR NEW.command_kind IS NOT OLD.command_kind OR NEW.local_intent_canonical IS NOT OLD.local_intent_canonical OR NEW.local_intent_sha256 IS NOT OLD.local_intent_sha256 OR NEW.created_at IS NOT OLD.created_at
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_scoped_sync_operation_roots_no_delete
BEFORE DELETE ON scoped_sync_operation_roots
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_scoped_sync_operation_roots_update
BEFORE UPDATE ON scoped_sync_operation_roots
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_scoped_sync_base_no_delete
BEFORE DELETE ON scoped_sync_base
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_scoped_sync_base_update
BEFORE UPDATE ON scoped_sync_base
WHEN NEW.channel_id IS NOT OLD.channel_id OR NEW.entity_kind IS NOT OLD.entity_kind OR NEW.entity_id IS NOT OLD.entity_id
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_scoped_sync_inbox_no_delete
BEFORE DELETE ON scoped_sync_inbox
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_scoped_sync_inbox_update
BEFORE UPDATE ON scoped_sync_inbox
WHEN NEW.inbox_id IS NOT OLD.inbox_id OR NEW.channel_id IS NOT OLD.channel_id OR NEW.authorization_generation IS NOT OLD.authorization_generation OR NEW.epoch IS NOT OLD.epoch OR NEW.batch_sequence IS NOT OLD.batch_sequence OR NEW.cursor_before IS NOT OLD.cursor_before OR NEW.cursor_after IS NOT OLD.cursor_after OR NEW.batch_canonical IS NOT OLD.batch_canonical OR NEW.batch_sha256 IS NOT OLD.batch_sha256 OR NEW.received_at IS NOT OLD.received_at
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_scoped_sync_snapshot_installs_no_delete
BEFORE DELETE ON scoped_sync_snapshot_installs
WHEN 1
BEGIN
  SELECT RAISE(ABORT,'d19_retained_record');
END;

CREATE TRIGGER d19_scoped_sync_snapshot_installs_update
BEFORE UPDATE ON scoped_sync_snapshot_installs
WHEN NEW.installation_id IS NOT OLD.installation_id OR NEW.channel_id IS NOT OLD.channel_id OR NEW.snapshot_id IS NOT OLD.snapshot_id OR NEW.authorization_generation IS NOT OLD.authorization_generation OR NEW.epoch IS NOT OLD.epoch OR NEW.expected_cursor IS NOT OLD.expected_cursor OR NEW.high_water_cursor IS NOT OLD.high_water_cursor OR NEW.payload_canonical IS NOT OLD.payload_canonical OR NEW.payload_sha256 IS NOT OLD.payload_sha256 OR NEW.created_at IS NOT OLD.created_at
BEGIN
  SELECT RAISE(ABORT,'d19_immutable_or_cas');
END;

CREATE TRIGGER d19_project_ownership_lifecycle
BEFORE UPDATE ON project_ownership
WHEN (OLD.registration_state='closed' AND NEW.registration_state<>'closed') OR (OLD.registration_state='active' AND NEW.registration_state='pending')
BEGIN
  SELECT RAISE(ABORT,'d19_registration_lifecycle');
END;

CREATE TRIGGER d19_project_invitations_terminal
BEFORE UPDATE ON project_invitations
WHEN OLD.state<>'pending'
BEGIN
  SELECT RAISE(ABORT,'d19_invitation_terminal');
END;

CREATE TRIGGER d19_library_contract_state_generation
BEFORE UPDATE ON library_contract_state
WHEN NEW.authorization_generation<OLD.authorization_generation OR NEW.authorization_generation>OLD.authorization_generation+1
BEGIN
  SELECT RAISE(ABORT,'d19_generation_step');
END;

CREATE TRIGGER d19_project_access_policies_generation
BEFORE UPDATE ON project_access_policies
WHEN NEW.authorization_generation<OLD.authorization_generation OR NEW.authorization_generation>OLD.authorization_generation+1 OR NEW.authorization_generation<>OLD.authorization_generation+1
BEGIN
  SELECT RAISE(ABORT,'d19_generation_step');
END;

CREATE TRIGGER d19_project_access_grants_principal_insert
BEFORE INSERT ON project_access_grants
WHEN NEW.local_principal_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM library_contract_state c WHERE c.library_id=NEW.library_id AND c.authority_mode='local-only' AND c.local_principal_id=NEW.local_principal_id)
BEGIN
  SELECT RAISE(ABORT,'d19_local_principal');
END;

CREATE TRIGGER d19_host_binding_selections_verified_insert
BEFORE INSERT ON host_binding_selections
WHEN NOT EXISTS (SELECT 1 FROM host_bindings b WHERE b.device_id=NEW.device_id AND b.library_id=NEW.library_id AND b.storage_root_id=NEW.storage_root_id AND b.binding_id=NEW.binding_id AND b.state='verified')
BEGIN
  SELECT RAISE(ABORT,'d19_selection_unverified');
END;

CREATE TRIGGER d19_project_access_grants_principal_update
BEFORE UPDATE ON project_access_grants
WHEN NEW.local_principal_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM library_contract_state c WHERE c.library_id=NEW.library_id AND c.authority_mode='local-only' AND c.local_principal_id=NEW.local_principal_id)
BEGIN
  SELECT RAISE(ABORT,'d19_local_principal');
END;

CREATE TRIGGER d19_host_binding_selections_verified_update
BEFORE UPDATE ON host_binding_selections
WHEN NOT EXISTS (SELECT 1 FROM host_bindings b WHERE b.device_id=NEW.device_id AND b.library_id=NEW.library_id AND b.storage_root_id=NEW.storage_root_id AND b.binding_id=NEW.binding_id AND b.state='verified')
BEGIN
  SELECT RAISE(ABORT,'d19_selection_unverified');
END;

CREATE TRIGGER d19_host_binding_selections_cas
BEFORE UPDATE ON host_binding_selections
WHEN NEW.selection_revision<>OLD.selection_revision+1
BEGIN
  SELECT RAISE(ABORT,'d19_selection_cas');
END;

CREATE TRIGGER d19_host_bindings_selected
BEFORE UPDATE ON host_bindings
WHEN NEW.state<>'verified' AND EXISTS (SELECT 1 FROM host_binding_selections s WHERE s.device_id=OLD.device_id AND s.library_id=OLD.library_id AND s.storage_root_id=OLD.storage_root_id AND s.binding_id=OLD.binding_id)
BEGIN
  SELECT RAISE(ABORT,'d19_selected_binding_retired');
END;

CREATE TRIGGER d19_host_bindings_generation
BEFORE UPDATE ON host_bindings
WHEN NEW.generation<OLD.generation OR NEW.generation>OLD.generation+1 OR ((NEW.host_path IS NOT OLD.host_path OR NEW.secure_handle_ref IS NOT OLD.secure_handle_ref OR NEW.availability<>OLD.availability OR NEW.state<>OLD.state) AND NEW.generation<>OLD.generation+1) OR (OLD.state='retired' AND NEW.state<>'retired')
BEGIN
  SELECT RAISE(ABORT,'d19_binding_generation');
END;

CREATE TRIGGER d19_scoped_sync_channels_cas
BEFORE UPDATE ON scoped_sync_channels
WHEN NEW.local_revision<>OLD.local_revision+1 OR NEW.authorization_generation<OLD.authorization_generation
BEGIN
  SELECT RAISE(ABORT,'d19_channel_cas');
END;

CREATE TRIGGER d19_scoped_sync_operations_seal
BEFORE UPDATE ON scoped_sync_operations
WHEN (OLD.sealed_request IS NOT NULL AND (NEW.sealed_request IS NOT OLD.sealed_request OR NEW.request_sha256 IS NOT OLD.request_sha256)) OR (OLD.receipt_canonical IS NOT NULL AND (NEW.receipt_canonical IS NOT OLD.receipt_canonical OR NEW.receipt_sha256 IS NOT OLD.receipt_sha256)) OR (NEW.state IN ('superseded','discarded') AND OLD.sealed_request IS NOT NULL AND OLD.state NOT IN ('rejected','conflict')) OR (OLD.state IN ('acknowledged','superseded','discarded') AND NEW.state<>OLD.state) OR (OLD.sealed_request IS NOT NULL AND NEW.state='queued')
BEGIN
  SELECT RAISE(ABORT,'d19_sealed_history');
END;

CREATE TRIGGER d19_scoped_sync_inbox_terminal
BEFORE UPDATE ON scoped_sync_inbox
WHEN OLD.state IN ('applied','access-lost')
BEGIN
  SELECT RAISE(ABORT,'d19_inbox_terminal');
END;

CREATE TRIGGER d19_scoped_sync_snapshot_installs_terminal
BEFORE UPDATE ON scoped_sync_snapshot_installs
WHEN OLD.state IN ('installed','superseded')
BEGIN
  SELECT RAISE(ABORT,'d19_snapshot_terminal');
END;

CREATE TRIGGER d19_context_apply_intents_terminal
BEFORE UPDATE ON context_apply_intents
WHEN OLD.state='settled' AND NEW.state<>'settled'
BEGIN
  SELECT RAISE(ABORT,'d19_apply_terminal');
END;

CREATE TRIGGER d19_context_apply_receipts_authority
BEFORE INSERT ON context_apply_receipts
WHEN (NEW.observation_kind='local-applied' AND NOT EXISTS (SELECT 1 FROM context_apply_intents i WHERE i.library_id=NEW.library_id AND i.operation_id=NEW.operation_id AND i.target_authority='library')) OR (NEW.observation_kind='package-published' AND NOT EXISTS (SELECT 1 FROM context_apply_intents i WHERE i.library_id=NEW.library_id AND i.operation_id=NEW.operation_id AND i.target_authority='project'))
BEGIN
  SELECT RAISE(ABORT,'d19_receipt_authority');
END;

CREATE TRIGGER d19_storage_slots_target_insert
BEFORE INSERT ON storage_slots
WHEN NEW.state='active' AND NOT EXISTS (SELECT 1 FROM storage_roots r JOIN storage_location_specs s ON s.library_id=r.library_id AND s.storage_root_id=r.storage_root_id WHERE r.library_id=NEW.library_id AND r.storage_root_id=NEW.storage_root_id AND r.state='active')
BEGIN
  SELECT RAISE(ABORT,'d19_slot_unclassified');
END;

CREATE TRIGGER d19_storage_slots_target_update
BEFORE UPDATE ON storage_slots
WHEN NEW.state='active' AND NOT EXISTS (SELECT 1 FROM storage_roots r JOIN storage_location_specs s ON s.library_id=r.library_id AND s.storage_root_id=r.storage_root_id WHERE r.library_id=NEW.library_id AND r.storage_root_id=NEW.storage_root_id AND r.state='active')
BEGIN
  SELECT RAISE(ABORT,'d19_slot_unclassified');
END;

CREATE TRIGGER d19_storage_roots_live_dependents
BEFORE UPDATE ON storage_roots
WHEN NEW.state='tombstoned' AND (EXISTS (SELECT 1 FROM storage_slots s WHERE s.library_id=NEW.library_id AND s.storage_root_id=NEW.storage_root_id AND s.state='active') OR EXISTS (SELECT 1 FROM project_locators l WHERE l.library_id=NEW.library_id AND l.storage_root_id=NEW.storage_root_id AND l.state='active'))
BEGIN
  SELECT RAISE(ABORT,'d19_root_in_use');
END;

-- Repository activation preflight: foreign_keys=ON, BEGIN IMMEDIATE, exact
-- baseline checksums and metadata, sealed-v1 settlement, explicit mappings.
-- SQLite cannot defer arbitrary triggers to commit: verify last manager,
-- parent CAS, access generations, payload/aggregate agreement, and ordered
-- sync install in the repository immediately before COMMIT. See RESPONSIBILITIES.md.
-- Check exactly one affected row. Do not change user_version, family, epoch or ledger.
UPDATE schema_metadata SET minimum_reader=2,minimum_writer=2
WHERE singleton=1 AND schema_family='photara.local.g2' AND schema_epoch=1
  AND minimum_reader=1 AND minimum_writer=1;
