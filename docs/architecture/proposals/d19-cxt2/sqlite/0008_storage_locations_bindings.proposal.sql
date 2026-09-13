-- INERT PROPOSAL ONLY — CXT2, accepted design 2026-09-12.
-- NOT AN INSTALLED, APPLIED, OR RUNNABLE MIGRATION. DO NOT EXECUTE.
-- Review text outside every runtime migration directory; no runner references it.
-- Implementation/adaptation and disposable execution require separate CXT3 scope.
-- Read ../README.md and ../RESPONSIBILITIES.md before reviewing this SQL.
-- Reservations map to D19_STATIC_SCHEMA_DELTA.md; baseline bytes remain intact.

-- S1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE storage_location_specs (
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  storage_root_id BLOB NOT NULL CHECK (length(storage_root_id)=16 AND storage_root_id<>zeroblob(16)),
  storage_kind TEXT NOT NULL,
  provider_id TEXT,
  supported_rights_json TEXT NOT NULL CHECK (CASE WHEN json_valid(supported_rights_json) THEN json_type(supported_rights_json)='array' ELSE 0 END),
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (library_id,storage_root_id),
  FOREIGN KEY (library_id,storage_root_id) REFERENCES storage_roots (library_id,storage_root_id) ON DELETE RESTRICT,
  CHECK (storage_kind IN ('filesystem','provider')),
  CHECK ((storage_kind='provider')=(provider_id IS NOT NULL)),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

-- S2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE storage_slots (
  slot_id BLOB NOT NULL CHECK (length(slot_id)=16 AND slot_id<>zeroblob(16)),
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  current_name TEXT NOT NULL,
  display_name TEXT NOT NULL,
  storage_root_id BLOB NOT NULL CHECK (length(storage_root_id)=16 AND storage_root_id<>zeroblob(16)),
  state TEXT NOT NULL,
  retired_at INTEGER,
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (slot_id),
  UNIQUE (library_id,slot_id),
  FOREIGN KEY (library_id,storage_root_id) REFERENCES storage_roots (library_id,storage_root_id) ON DELETE RESTRICT,
  CHECK (state IN ('active','tombstoned')),
  CHECK ((state='tombstoned')=(retired_at IS NOT NULL)),
  CHECK (length(current_name) BETWEEN 1 AND 64 AND substr(current_name,1,1) GLOB '[a-z]' AND current_name NOT GLOB '*[^a-z0-9_]*'),
  FOREIGN KEY (library_id,slot_id,current_name) REFERENCES storage_slot_names (library_id,slot_id,name) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

CREATE INDEX d19_storage_slots_root ON storage_slots (library_id,storage_root_id,state);

-- S3: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE storage_slot_names (
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  name TEXT NOT NULL,
  slot_id BLOB NOT NULL CHECK (length(slot_id)=16 AND slot_id<>zeroblob(16)),
  claimed_at INTEGER NOT NULL,
  PRIMARY KEY (library_id,name),
  UNIQUE (library_id,slot_id,name),
  FOREIGN KEY (library_id,slot_id) REFERENCES storage_slots (library_id,slot_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
  CHECK (length(name) BETWEEN 1 AND 64 AND substr(name,1,1) GLOB '[a-z]' AND name NOT GLOB '*[^a-z0-9_]*'),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT
) STRICT;

-- H1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE host_bindings (
  binding_id BLOB NOT NULL CHECK (length(binding_id)=16 AND binding_id<>zeroblob(16)),
  device_id BLOB NOT NULL CHECK (length(device_id)=16 AND device_id<>zeroblob(16)),
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  storage_root_id BLOB NOT NULL CHECK (length(storage_root_id)=16 AND storage_root_id<>zeroblob(16)),
  host_kind TEXT NOT NULL,
  binding_kind TEXT NOT NULL,
  host_path TEXT,
  secure_handle_ref BLOB CHECK (secure_handle_ref IS NULL OR (length(secure_handle_ref)=16 AND secure_handle_ref<>zeroblob(16))),
  state TEXT NOT NULL,
  generation INTEGER NOT NULL CHECK (generation>=1),
  availability TEXT NOT NULL,
  verification_json TEXT CHECK (verification_json IS NULL OR (CASE WHEN json_valid(verification_json) THEN json_type(verification_json)='object' ELSE 0 END)),
  verified_at INTEGER,
  diagnostic_code TEXT,
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (binding_id),
  UNIQUE (device_id,library_id,storage_root_id,binding_id),
  FOREIGN KEY (device_id) REFERENCES local_device (device_id) ON DELETE RESTRICT,
  FOREIGN KEY (library_id,storage_root_id) REFERENCES storage_roots (library_id,storage_root_id) ON DELETE RESTRICT,
  CHECK (host_kind IN ('macos','windows','linux')),
  CHECK (binding_kind IN ('path','bookmark','provider')),
  CHECK (state IN ('candidate','verified','retired')),
  CHECK (availability IN ('unknown','available','unavailable','denied','stale','ambiguous','unsupported')),
  CHECK ((binding_kind='path' AND host_path IS NOT NULL AND secure_handle_ref IS NULL) OR (binding_kind IN ('bookmark','provider') AND host_path IS NULL AND secure_handle_ref IS NOT NULL)),
  CHECK (state<>'verified' OR (verification_json IS NOT NULL AND verified_at IS NOT NULL)),
  CHECK (availability<>'available' OR state='verified'),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

CREATE INDEX d19_host_bindings_device_root ON host_bindings (device_id,library_id,storage_root_id,state);

-- H2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE host_binding_selections (
  device_id BLOB NOT NULL CHECK (length(device_id)=16 AND device_id<>zeroblob(16)),
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  storage_root_id BLOB NOT NULL CHECK (length(storage_root_id)=16 AND storage_root_id<>zeroblob(16)),
  binding_id BLOB NOT NULL CHECK (length(binding_id)=16 AND binding_id<>zeroblob(16)),
  selection_revision INTEGER NOT NULL CHECK (selection_revision>=1),
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (device_id,library_id,storage_root_id),
  FOREIGN KEY (device_id,library_id,storage_root_id,binding_id) REFERENCES host_bindings (device_id,library_id,storage_root_id,binding_id) ON DELETE RESTRICT,
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT
) STRICT;

-- H3: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE legacy_external_resource_resolutions (
  device_id BLOB NOT NULL CHECK (length(device_id)=16 AND device_id<>zeroblob(16)),
  project_id BLOB NOT NULL CHECK (length(project_id)=16 AND project_id<>zeroblob(16)),
  legacy_binding_id BLOB NOT NULL CHECK (length(legacy_binding_id)=16 AND legacy_binding_id<>zeroblob(16)),
  source_object_sha256 BLOB NOT NULL CHECK (length(source_object_sha256)=32),
  external_ref_id BLOB NOT NULL CHECK (length(external_ref_id)=16 AND external_ref_id<>zeroblob(16)),
  external_revision_id BLOB NOT NULL CHECK (length(external_revision_id)=16 AND external_revision_id<>zeroblob(16)),
  mapping_commit_id BLOB NOT NULL CHECK (length(mapping_commit_id)=16 AND mapping_commit_id<>zeroblob(16)),
  mapping_commit_sha256 BLOB NOT NULL CHECK (length(mapping_commit_sha256)=32),
  created_at INTEGER NOT NULL,
  PRIMARY KEY (device_id,project_id,legacy_binding_id,source_object_sha256),
  FOREIGN KEY (device_id) REFERENCES local_device (device_id) ON DELETE RESTRICT
) STRICT;

CREATE INDEX d19_storage_slots_fk6 ON storage_slots (library_id,slot_id,current_name);

CREATE INDEX d19_host_bindings_fk3 ON host_bindings (library_id,storage_root_id);

CREATE INDEX d19_host_binding_selections_fk1 ON host_binding_selections (device_id,library_id,storage_root_id,binding_id);

CREATE INDEX d19_host_binding_selections_fk2 ON host_binding_selections (library_id);
