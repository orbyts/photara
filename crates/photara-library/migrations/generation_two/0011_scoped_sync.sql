-- CXT3b executable local migration, promoted from accepted D19 CXT2.
-- P1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE project_media_links (
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  project_id BLOB NOT NULL CHECK (length(project_id)=16 AND project_id<>zeroblob(16)),
  sha256 BLOB NOT NULL CHECK (length(sha256)=32),
  purpose TEXT NOT NULL,
  source_commit_id BLOB NOT NULL CHECK (length(source_commit_id)=16 AND source_commit_id<>zeroblob(16)),
  source_commit_sha256 BLOB NOT NULL CHECK (length(source_commit_sha256)=32),
  consent_projection_sha256 BLOB NOT NULL CHECK (length(consent_projection_sha256)=32),
  state TEXT NOT NULL,
  retired_at INTEGER,
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (library_id,project_id,sha256,purpose),
  FOREIGN KEY (library_id,project_id) REFERENCES project_ownership (library_id,project_id) ON DELETE RESTRICT,
  FOREIGN KEY (library_id,sha256) REFERENCES library_media (library_id,sha256) ON DELETE RESTRICT,
  CHECK (purpose IN ('cover','assigned-snapshot')),
  CHECK (state IN ('active','tombstoned')),
  CHECK ((state='tombstoned')=(retired_at IS NOT NULL)),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

CREATE INDEX d19_project_media_links_media ON project_media_links (library_id,sha256,state,project_id);

-- Q1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE scoped_sync_channels (
  channel_id BLOB NOT NULL CHECK (length(channel_id)=16 AND channel_id<>zeroblob(16)),
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  account_id BLOB NOT NULL CHECK (length(account_id)=16 AND account_id<>zeroblob(16)),
  environment_id TEXT NOT NULL,
  scope_kind TEXT NOT NULL,
  project_id BLOB CHECK (project_id IS NULL OR (length(project_id)=16 AND project_id<>zeroblob(16))),
  authorization_generation INTEGER NOT NULL CHECK (authorization_generation>=1),
  epoch BLOB NOT NULL CHECK (length(epoch)=16 AND epoch<>zeroblob(16)),
  applied_cursor TEXT,
  state TEXT NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (channel_id),
  FOREIGN KEY (account_id) REFERENCES account_cache (account_id) ON DELETE RESTRICT,
  CHECK (scope_kind IN ('library','project')),
  CHECK ((scope_kind='project')=(project_id IS NOT NULL)),
  FOREIGN KEY (library_id,project_id) REFERENCES project_ownership (library_id,project_id) ON DELETE RESTRICT,
  CHECK (state IN ('active','paused','access-lost','snapshot-required')),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT
) STRICT;

CREATE UNIQUE INDEX d19_scoped_sync_channels_library ON scoped_sync_channels (library_id,account_id,environment_id) WHERE scope_kind='library';

CREATE UNIQUE INDEX d19_scoped_sync_channels_project ON scoped_sync_channels (library_id,project_id,account_id,environment_id) WHERE scope_kind='project';

-- Q2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE scoped_sync_operations (
  operation_id BLOB NOT NULL CHECK (length(operation_id)=16 AND operation_id<>zeroblob(16)),
  channel_id BLOB NOT NULL CHECK (length(channel_id)=16 AND channel_id<>zeroblob(16)),
  command_kind TEXT NOT NULL,
  local_intent_canonical BLOB NOT NULL,
  local_intent_sha256 BLOB NOT NULL CHECK (length(local_intent_sha256)=32),
  sealed_request BLOB,
  request_sha256 BLOB CHECK (request_sha256 IS NULL OR (length(request_sha256)=32)),
  receipt_canonical BLOB,
  receipt_sha256 BLOB CHECK (receipt_sha256 IS NULL OR (length(receipt_sha256)=32)),
  state TEXT NOT NULL,
  replacement_operation_id BLOB CHECK (replacement_operation_id IS NULL OR (length(replacement_operation_id)=16 AND replacement_operation_id<>zeroblob(16))),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (operation_id),
  FOREIGN KEY (channel_id) REFERENCES scoped_sync_channels (channel_id) ON DELETE RESTRICT,
  UNIQUE (channel_id,operation_id),
  FOREIGN KEY (channel_id,replacement_operation_id) REFERENCES scoped_sync_operations (channel_id,operation_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
  CHECK (state IN ('queued','sealed','acknowledged','rejected','conflict','unknown','superseded','discarded')),
  CHECK ((sealed_request IS NULL)=(request_sha256 IS NULL)),
  CHECK ((receipt_canonical IS NULL)=(receipt_sha256 IS NULL)),
  CHECK (state<>'acknowledged' OR receipt_canonical IS NOT NULL),
  CHECK (state NOT IN ('sealed','acknowledged','rejected','conflict','unknown') OR sealed_request IS NOT NULL),
  CHECK (state<>'queued' OR sealed_request IS NULL),
  CHECK ((state='superseded')=(replacement_operation_id IS NOT NULL)),
  CHECK (replacement_operation_id IS NULL OR replacement_operation_id<>operation_id),
  CHECK (length(local_intent_canonical)<=1048576),
  CHECK (length(sealed_request)<=1048576),
  CHECK (length(receipt_canonical)<=4194304)
) STRICT;

CREATE INDEX d19_scoped_sync_operations_queue ON scoped_sync_operations (channel_id,state,created_at,operation_id);

-- Q3: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE scoped_sync_operation_roots (
  channel_id BLOB NOT NULL CHECK (length(channel_id)=16 AND channel_id<>zeroblob(16)),
  operation_id BLOB NOT NULL CHECK (length(operation_id)=16 AND operation_id<>zeroblob(16)),
  entity_kind TEXT NOT NULL,
  entity_id BLOB NOT NULL CHECK (length(entity_id)=16 AND entity_id<>zeroblob(16)),
  expected_server_revision TEXT,
  predecessor_operation_id BLOB CHECK (predecessor_operation_id IS NULL OR (length(predecessor_operation_id)=16 AND predecessor_operation_id<>zeroblob(16))),
  local_poststate BLOB NOT NULL,
  local_poststate_sha256 BLOB NOT NULL CHECK (length(local_poststate_sha256)=32),
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  PRIMARY KEY (operation_id,entity_kind,entity_id),
  FOREIGN KEY (channel_id,operation_id) REFERENCES scoped_sync_operations (channel_id,operation_id) ON DELETE RESTRICT,
  FOREIGN KEY (channel_id,predecessor_operation_id) REFERENCES scoped_sync_operations (channel_id,operation_id) ON DELETE RESTRICT,
  CHECK (expected_server_revision IS NULL OR predecessor_operation_id IS NULL),
  CHECK (predecessor_operation_id IS NULL OR predecessor_operation_id<>operation_id),
  CHECK (length(local_poststate)<=1048576)
) STRICT;

CREATE INDEX d19_scoped_sync_operation_roots_entity ON scoped_sync_operation_roots (channel_id,entity_kind,entity_id,operation_id);

-- Q4: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE scoped_sync_base (
  channel_id BLOB NOT NULL CHECK (length(channel_id)=16 AND channel_id<>zeroblob(16)),
  entity_kind TEXT NOT NULL,
  entity_id BLOB NOT NULL CHECK (length(entity_id)=16 AND entity_id<>zeroblob(16)),
  server_revision TEXT NOT NULL,
  poststate_canonical BLOB NOT NULL,
  poststate_sha256 BLOB NOT NULL CHECK (length(poststate_sha256)=32),
  observed_local_revision INTEGER NOT NULL CHECK (observed_local_revision>=1),
  PRIMARY KEY (channel_id,entity_kind,entity_id),
  FOREIGN KEY (channel_id) REFERENCES scoped_sync_channels (channel_id) ON DELETE RESTRICT,
  CHECK (length(poststate_canonical)<=1048576)
) STRICT;

-- Q5: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE scoped_sync_inbox (
  inbox_id BLOB NOT NULL CHECK (length(inbox_id)=16 AND inbox_id<>zeroblob(16)),
  channel_id BLOB NOT NULL CHECK (length(channel_id)=16 AND channel_id<>zeroblob(16)),
  authorization_generation INTEGER NOT NULL CHECK (authorization_generation>=1),
  epoch BLOB NOT NULL CHECK (length(epoch)=16 AND epoch<>zeroblob(16)),
  batch_sequence INTEGER NOT NULL CHECK (batch_sequence>=0),
  cursor_before TEXT NOT NULL,
  cursor_after TEXT NOT NULL,
  batch_canonical BLOB NOT NULL,
  batch_sha256 BLOB NOT NULL CHECK (length(batch_sha256)=32),
  state TEXT NOT NULL,
  received_at INTEGER NOT NULL,
  applied_at INTEGER,
  PRIMARY KEY (inbox_id),
  FOREIGN KEY (channel_id) REFERENCES scoped_sync_channels (channel_id) ON DELETE RESTRICT,
  UNIQUE (channel_id,authorization_generation,epoch,batch_sequence),
  CHECK (batch_sequence>0),
  CHECK (length(batch_canonical)<=4194304),
  CHECK (state IN ('received','applied','conflict','unsupported','access-lost')),
  CHECK ((state='applied')=(applied_at IS NOT NULL))
) STRICT;

CREATE UNIQUE INDEX d19_scoped_sync_inbox_pending ON scoped_sync_inbox (channel_id) WHERE state IN ('received','conflict','unsupported');

-- Q6: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE scoped_sync_snapshot_installs (
  installation_id BLOB NOT NULL CHECK (length(installation_id)=16 AND installation_id<>zeroblob(16)),
  channel_id BLOB NOT NULL CHECK (length(channel_id)=16 AND channel_id<>zeroblob(16)),
  snapshot_id BLOB NOT NULL CHECK (length(snapshot_id)=16 AND snapshot_id<>zeroblob(16)),
  authorization_generation INTEGER NOT NULL CHECK (authorization_generation>=1),
  epoch BLOB NOT NULL CHECK (length(epoch)=16 AND epoch<>zeroblob(16)),
  expected_cursor TEXT,
  high_water_cursor TEXT NOT NULL,
  payload_canonical BLOB NOT NULL,
  payload_sha256 BLOB NOT NULL CHECK (length(payload_sha256)=32),
  state TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  installed_at INTEGER,
  PRIMARY KEY (installation_id),
  FOREIGN KEY (channel_id) REFERENCES scoped_sync_channels (channel_id) ON DELETE RESTRICT,
  UNIQUE (channel_id,snapshot_id),
  CHECK (length(payload_canonical)<=16777216),
  CHECK (state IN ('staged','installed','conflict','unsupported','superseded')),
  CHECK ((state='installed')=(installed_at IS NOT NULL))
) STRICT;

CREATE UNIQUE INDEX d19_scoped_sync_snapshot_installs_pending ON scoped_sync_snapshot_installs (channel_id) WHERE state IN ('staged','conflict','unsupported');

CREATE INDEX d19_scoped_sync_channels_fk1 ON scoped_sync_channels (account_id);

CREATE INDEX d19_scoped_sync_channels_fk4 ON scoped_sync_channels (library_id,project_id);

CREATE INDEX d19_scoped_sync_operations_fk3 ON scoped_sync_operations (channel_id,replacement_operation_id);

CREATE INDEX d19_scoped_sync_operation_roots_fk1 ON scoped_sync_operation_roots (channel_id,operation_id);

CREATE INDEX d19_scoped_sync_operation_roots_fk2 ON scoped_sync_operation_roots (channel_id,predecessor_operation_id);
