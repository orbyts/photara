-- Approved fresh-only LL2a delta. Never applied to an existing database.
CREATE TABLE lifecycle_authorities (
 authority_id BLOB PRIMARY KEY CHECK(length(authority_id)=16 AND authority_id<>zeroblob(16)),
 kind TEXT NOT NULL CHECK(kind='local'), epoch BLOB NOT NULL CHECK(length(epoch)=16 AND epoch<>zeroblob(16)),
 database_id BLOB NOT NULL CHECK(length(database_id)=16 AND database_id<>zeroblob(16)),
 environment_id TEXT CHECK(environment_id IS NULL), UNIQUE(database_id,epoch)
) STRICT;
CREATE TABLE lifecycle_intents (
 authority_id BLOB NOT NULL, principal_kind TEXT NOT NULL CHECK(principal_kind='local'),
 principal_id BLOB NOT NULL CHECK(length(principal_id)=16 AND principal_id<>zeroblob(16)),
 operation_id BLOB NOT NULL CHECK(length(operation_id)=16 AND operation_id<>zeroblob(16)),
 target_library_id BLOB NOT NULL CHECK(length(target_library_id)=16 AND target_library_id<>zeroblob(16)),
 action TEXT NOT NULL CHECK(action IN('create','rename')),
 request_canonical BLOB NOT NULL CHECK(length(request_canonical) BETWEEN 1 AND 65536),
 request_sha256 BLOB NOT NULL CHECK(length(request_sha256)=32),
 initiating_device_id BLOB NOT NULL CHECK(length(initiating_device_id)=16 AND initiating_device_id<>zeroblob(16)),
 state TEXT NOT NULL CHECK(state IN('prepared','dispatching','unknown','terminal')),
 created_at INTEGER NOT NULL CHECK(created_at>=0), updated_at INTEGER NOT NULL CHECK(updated_at>=created_at),
 PRIMARY KEY(authority_id,principal_kind,principal_id,operation_id),
 FOREIGN KEY(authority_id) REFERENCES lifecycle_authorities(authority_id) ON DELETE RESTRICT
) STRICT;
CREATE TABLE lifecycle_receipts (
 authority_id BLOB NOT NULL, principal_kind TEXT NOT NULL CHECK(principal_kind='local'),
 principal_id BLOB NOT NULL CHECK(length(principal_id)=16 AND principal_id<>zeroblob(16)),
 operation_id BLOB NOT NULL CHECK(length(operation_id)=16 AND operation_id<>zeroblob(16)),
 historical_library_id BLOB NOT NULL CHECK(length(historical_library_id)=16 AND historical_library_id<>zeroblob(16)),
 action TEXT NOT NULL CHECK(action IN('create','rename')),
 result TEXT NOT NULL CHECK(result IN('created','renamed','rejected')),
 request_sha256 BLOB NOT NULL CHECK(length(request_sha256)=32),
 result_canonical BLOB NOT NULL CHECK(length(result_canonical) BETWEEN 1 AND 65536),
 result_sha256 BLOB NOT NULL CHECK(length(result_sha256)=32),
 initiating_device_id BLOB NOT NULL CHECK(length(initiating_device_id)=16 AND initiating_device_id<>zeroblob(16)),
 committed_at INTEGER NOT NULL CHECK(committed_at>=0), reviewed_revision INTEGER CHECK(reviewed_revision>=1),
 PRIMARY KEY(authority_id,principal_kind,principal_id,operation_id),
 UNIQUE(authority_id,principal_kind,principal_id,operation_id,historical_library_id,result),
 FOREIGN KEY(authority_id) REFERENCES lifecycle_authorities(authority_id) ON DELETE RESTRICT
) STRICT;
CREATE TRIGGER lifecycle_authorities_immutable BEFORE UPDATE ON lifecycle_authorities BEGIN SELECT RAISE(ABORT,'immutable_authority'); END;
CREATE TRIGGER lifecycle_authorities_retained BEFORE DELETE ON lifecycle_authorities BEGIN SELECT RAISE(ABORT,'retained_authority'); END;
CREATE TRIGGER lifecycle_intents_immutable BEFORE UPDATE ON lifecycle_intents
WHEN OLD.authority_id IS NOT NEW.authority_id OR OLD.principal_kind<>NEW.principal_kind OR OLD.principal_id IS NOT NEW.principal_id OR OLD.operation_id IS NOT NEW.operation_id OR OLD.target_library_id IS NOT NEW.target_library_id OR OLD.action<>NEW.action OR OLD.request_canonical IS NOT NEW.request_canonical OR OLD.request_sha256 IS NOT NEW.request_sha256 OR OLD.initiating_device_id IS NOT NEW.initiating_device_id OR OLD.created_at<>NEW.created_at OR OLD.state='terminal'
BEGIN SELECT RAISE(ABORT,'immutable_intent'); END;
CREATE TRIGGER lifecycle_intents_retained BEFORE DELETE ON lifecycle_intents BEGIN SELECT RAISE(ABORT,'retained_intent'); END;
CREATE TRIGGER lifecycle_receipts_immutable BEFORE UPDATE ON lifecycle_receipts BEGIN SELECT RAISE(ABORT,'immutable_receipt'); END;
CREATE TRIGGER lifecycle_receipts_retained BEFORE DELETE ON lifecycle_receipts BEGIN SELECT RAISE(ABORT,'retained_receipt'); END;
CREATE TABLE local_activation_slots (
 device_id BLOB NOT NULL CHECK(length(device_id)=16 AND device_id<>zeroblob(16)),
 workspace_slot_id BLOB NOT NULL CHECK(length(workspace_slot_id)=16 AND workspace_slot_id<>zeroblob(16)),
 slot_scope_sha256 BLOB NOT NULL CHECK(length(slot_scope_sha256)=32),
 revision INTEGER NOT NULL CHECK(revision>=0), request_generation INTEGER NOT NULL CHECK(request_generation>=0),
 committed_generation INTEGER NOT NULL CHECK(committed_generation>=0 AND committed_generation<=request_generation),
 snapshot_canonical BLOB NOT NULL, snapshot_sha256 BLOB NOT NULL CHECK(length(snapshot_sha256)=32),
 active_library_id BLOB CHECK(active_library_id IS NULL OR(length(active_library_id)=16 AND active_library_id<>zeroblob(16))),
 active_project_id BLOB CHECK(active_project_id IS NULL OR(length(active_project_id)=16 AND active_project_id<>zeroblob(16))),
 CHECK(active_project_id IS NULL OR active_library_id IS NOT NULL),
 PRIMARY KEY(device_id,workspace_slot_id),
 FOREIGN KEY(device_id) REFERENCES local_device(device_id) ON DELETE RESTRICT,
 FOREIGN KEY(active_library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT,
 FOREIGN KEY(active_library_id,active_project_id) REFERENCES project_ownership(library_id,project_id) ON DELETE RESTRICT
) STRICT;
CREATE TRIGGER local_activation_slots_identity BEFORE UPDATE ON local_activation_slots
WHEN OLD.device_id IS NOT NEW.device_id OR OLD.workspace_slot_id IS NOT NEW.workspace_slot_id OR OLD.slot_scope_sha256 IS NOT NEW.slot_scope_sha256 OR NEW.revision<>OLD.revision+1
BEGIN SELECT RAISE(ABORT,'slot_identity_or_revision'); END;
CREATE TRIGGER local_activation_slots_retained BEFORE DELETE ON local_activation_slots BEGIN SELECT RAISE(ABORT,'retained_slot'); END;
UPDATE schema_metadata SET minimum_reader=6,minimum_writer=6 WHERE singleton=1;
