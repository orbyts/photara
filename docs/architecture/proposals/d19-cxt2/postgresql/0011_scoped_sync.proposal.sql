-- INERT PROPOSAL ONLY — CXT2, accepted design 2026-09-12.
-- NOT AN INSTALLED, APPLIED, OR RUNNABLE MIGRATION. DO NOT EXECUTE.
-- Review text outside every runtime migration directory; no runner references it.
-- Implementation/adaptation and disposable execution require separate CXT3 scope.
-- Read ../README.md and ../RESPONSIBILITIES.md before reviewing this SQL.
-- Reservations map to D19_STATIC_SCHEMA_DELTA.md; baseline bytes remain intact.

-- P1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.project_media_links (
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  project_id uuid NOT NULL CHECK (project_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  sha256 bytea NOT NULL CHECK (octet_length(sha256)=32),
  purpose text COLLATE "C" NOT NULL,
  source_commit_id uuid NOT NULL CHECK (source_commit_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  source_commit_sha256 bytea NOT NULL CHECK (octet_length(source_commit_sha256)=32),
  consent_projection_sha256 bytea NOT NULL CHECK (octet_length(consent_projection_sha256)=32),
  state text COLLATE "C" NOT NULL,
  retired_at timestamptz(3),
  record_schema integer NOT NULL,
  revision bigint NOT NULL CHECK (revision>=1),
  created_at timestamptz(3) NOT NULL,
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (workspace_id,project_id,sha256,purpose),
  FOREIGN KEY (workspace_id,project_id) REFERENCES photara.project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  FOREIGN KEY (workspace_id,sha256) REFERENCES photara.library_media (workspace_id,sha256) ON DELETE RESTRICT,
  CHECK (purpose IN ('cover','assigned-snapshot')),
  CHECK (state IN ('active','tombstoned')),
  CHECK ((state='tombstoned')=(retired_at IS NOT NULL)),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
);

CREATE INDEX d19_project_media_links_media ON photara.project_media_links (workspace_id,sha256,state,project_id);

-- F1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara_private.scoped_streams (
  stream_id uuid NOT NULL CHECK (stream_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  scope_kind text COLLATE "C" NOT NULL,
  project_id uuid CHECK (project_id IS NULL OR (project_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  epoch uuid NOT NULL CHECK (epoch<>'00000000-0000-0000-0000-000000000000'::uuid),
  last_sequence bigint NOT NULL CHECK (last_sequence>=0),
  created_at timestamptz(3) NOT NULL,
  PRIMARY KEY (stream_id),
  CHECK (scope_kind IN ('library','project')),
  CHECK ((scope_kind='project')=(project_id IS NOT NULL)),
  FOREIGN KEY (workspace_id,project_id) REFERENCES photara.project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  UNIQUE (stream_id,epoch),
  UNIQUE (stream_id,epoch,workspace_id),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX d19_scoped_streams_library ON photara_private.scoped_streams (workspace_id) WHERE scope_kind='library';

CREATE UNIQUE INDEX d19_scoped_streams_project ON photara_private.scoped_streams (workspace_id,project_id) WHERE scope_kind='project';

-- F2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara_private.scoped_change_batches (
  stream_id uuid NOT NULL CHECK (stream_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  epoch uuid NOT NULL CHECK (epoch<>'00000000-0000-0000-0000-000000000000'::uuid),
  sequence bigint NOT NULL CHECK (sequence>=0),
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  project_id uuid CHECK (project_id IS NULL OR (project_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  operation_id uuid NOT NULL CHECK (operation_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  change_count integer NOT NULL,
  batch_canonical bytea NOT NULL,
  batch_sha256 bytea NOT NULL CHECK (octet_length(batch_sha256)=32),
  committed_at timestamptz(3) NOT NULL,
  PRIMARY KEY (stream_id,epoch,sequence),
  FOREIGN KEY (stream_id,epoch,workspace_id) REFERENCES photara_private.scoped_streams (stream_id,epoch,workspace_id) ON DELETE RESTRICT,
  UNIQUE (stream_id,epoch,sequence,workspace_id),
  UNIQUE (workspace_id,operation_id),
  CHECK (sequence>0 AND change_count BETWEEN 1 AND 1000),
  CHECK (octet_length(batch_canonical)<=3145728),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT
);

-- F3: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara_private.scoped_changes (
  stream_id uuid NOT NULL CHECK (stream_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  epoch uuid NOT NULL CHECK (epoch<>'00000000-0000-0000-0000-000000000000'::uuid),
  sequence bigint NOT NULL CHECK (sequence>=0),
  ordinal integer NOT NULL,
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  project_id uuid CHECK (project_id IS NULL OR (project_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  entity_kind text COLLATE "C" NOT NULL,
  entity_id uuid NOT NULL CHECK (entity_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  entity_revision bigint NOT NULL CHECK (entity_revision>=1),
  change_kind text COLLATE "C" NOT NULL,
  record_schema integer NOT NULL,
  post_state jsonb NOT NULL CHECK (jsonb_typeof(post_state)='object'),
  post_state_canonical bytea NOT NULL,
  post_state_sha256 bytea NOT NULL CHECK (octet_length(post_state_sha256)=32),
  PRIMARY KEY (stream_id,epoch,sequence,ordinal),
  FOREIGN KEY (stream_id,epoch,sequence,workspace_id) REFERENCES photara_private.scoped_change_batches (stream_id,epoch,sequence,workspace_id) ON DELETE RESTRICT,
  UNIQUE (stream_id,entity_kind,entity_id,entity_revision),
  CHECK (sequence>0 AND ordinal BETWEEN 0 AND 999 AND record_schema>=1),
  CHECK (octet_length(post_state_canonical)<=1048576),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT
);

-- F4: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara_private.scoped_command_receipts (
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  operation_id uuid NOT NULL CHECK (operation_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  actor_account_id uuid NOT NULL CHECK (actor_account_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  device_id uuid NOT NULL CHECK (device_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  scope_kind text COLLATE "C" NOT NULL,
  project_id uuid CHECK (project_id IS NULL OR (project_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  command_kind text COLLATE "C" NOT NULL,
  request_canonical bytea NOT NULL,
  request_sha256 bytea NOT NULL CHECK (octet_length(request_sha256)=32),
  outcome text COLLATE "C" NOT NULL,
  response_canonical bytea NOT NULL,
  response_sha256 bytea NOT NULL CHECK (octet_length(response_sha256)=32),
  accepted_stream_id uuid CHECK (accepted_stream_id IS NULL OR (accepted_stream_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  accepted_epoch uuid CHECK (accepted_epoch IS NULL OR (accepted_epoch<>'00000000-0000-0000-0000-000000000000'::uuid)),
  accepted_sequence bigint CHECK (accepted_sequence IS NULL OR (accepted_sequence>=0)),
  completed_at timestamptz(3) NOT NULL,
  PRIMARY KEY (workspace_id,operation_id),
  FOREIGN KEY (actor_account_id,device_id) REFERENCES photara_identity.devices (account_id,device_id) ON DELETE RESTRICT,
  FOREIGN KEY (workspace_id,project_id) REFERENCES photara.project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  FOREIGN KEY (accepted_stream_id,accepted_epoch,accepted_sequence,workspace_id) REFERENCES photara_private.scoped_change_batches (stream_id,epoch,sequence,workspace_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
  CHECK (scope_kind IN ('library','project','control')),
  CHECK (scope_kind='control' OR (scope_kind='project')=(project_id IS NOT NULL)),
  CHECK (outcome IN ('accepted','control-accepted','rejected','conflict')),
  CHECK ((outcome='accepted' AND accepted_stream_id IS NOT NULL AND accepted_epoch IS NOT NULL AND accepted_sequence IS NOT NULL AND accepted_sequence>0) OR (outcome<>'accepted' AND accepted_stream_id IS NULL AND accepted_epoch IS NULL AND accepted_sequence IS NULL)),
  CHECK ((scope_kind='control' AND outcome<>'accepted') OR (scope_kind<>'control' AND outcome<>'control-accepted')),
  CHECK (octet_length(request_canonical)<=1048576),
  CHECK (octet_length(response_canonical)<=4194304),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT
);

ALTER TABLE photara_private.scoped_change_batches ADD CONSTRAINT d19_batch_receipt FOREIGN KEY (workspace_id,operation_id) REFERENCES photara_private.scoped_command_receipts (workspace_id,operation_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

CREATE INDEX d19_scoped_command_receipts_actor ON photara_private.scoped_command_receipts (actor_account_id,workspace_id,project_id,completed_at,operation_id);

-- F5: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara_private.scoped_sync_clients (
  stream_id uuid NOT NULL CHECK (stream_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  account_id uuid NOT NULL CHECK (account_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  device_id uuid NOT NULL CHECK (device_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  authorization_generation bigint NOT NULL CHECK (authorization_generation>=1),
  epoch uuid NOT NULL CHECK (epoch<>'00000000-0000-0000-0000-000000000000'::uuid),
  acknowledged_sequence bigint NOT NULL CHECK (acknowledged_sequence>=0),
  last_seen_at timestamptz(3) NOT NULL,
  PRIMARY KEY (stream_id,account_id,device_id),
  FOREIGN KEY (stream_id,epoch) REFERENCES photara_private.scoped_streams (stream_id,epoch) ON DELETE RESTRICT,
  FOREIGN KEY (account_id,device_id) REFERENCES photara_identity.devices (account_id,device_id) ON DELETE RESTRICT
);

CREATE INDEX d19_scoped_sync_clients_device ON photara_private.scoped_sync_clients (account_id,device_id,stream_id);

ALTER TABLE photara_private.media_upload_sessions
  ADD COLUMN project_id uuid,
  ADD COLUMN actor_account_id uuid,
  ADD COLUMN project_media_purpose text COLLATE "C",
  ADD COLUMN authorization_generation bigint,
  ADD CONSTRAINT d19_upload_project FOREIGN KEY (workspace_id,project_id) REFERENCES photara.project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  ADD CONSTRAINT d19_upload_actor FOREIGN KEY (actor_account_id) REFERENCES photara_identity.accounts (account_id) ON DELETE RESTRICT,
  ADD CONSTRAINT d19_upload_scope CHECK (
    (project_id IS NULL AND project_media_purpose IS NULL AND
      ((actor_account_id IS NULL AND authorization_generation IS NULL) OR
       (actor_account_id IS NOT NULL AND authorization_generation IS NOT NULL AND authorization_generation>0))) OR
    (project_id IS NOT NULL AND actor_account_id IS NOT NULL AND
     project_media_purpose IS NOT NULL AND project_media_purpose IN ('cover','assigned-snapshot') AND
     authorization_generation IS NOT NULL AND authorization_generation>0));

CREATE INDEX d19_media_upload_sessions_project_actor ON photara_private.media_upload_sessions (workspace_id,project_id,actor_account_id,state,expires_at);

CREATE INDEX d19_media_upload_sessions_actor ON photara_private.media_upload_sessions (actor_account_id);

CREATE INDEX d19_scoped_streams_fk3 ON photara_private.scoped_streams (workspace_id,project_id);

CREATE INDEX d19_scoped_change_batches_fk1 ON photara_private.scoped_change_batches (stream_id,epoch,workspace_id);

CREATE INDEX d19_scoped_changes_fk1 ON photara_private.scoped_changes (stream_id,epoch,sequence,workspace_id);

CREATE INDEX d19_scoped_changes_fk5 ON photara_private.scoped_changes (workspace_id);

CREATE INDEX d19_scoped_command_receipts_fk1 ON photara_private.scoped_command_receipts (actor_account_id,device_id);

CREATE INDEX d19_scoped_command_receipts_fk2 ON photara_private.scoped_command_receipts (workspace_id,project_id);

CREATE INDEX d19_scoped_command_receipts_fk3 ON photara_private.scoped_command_receipts (accepted_stream_id,accepted_epoch,accepted_sequence,workspace_id);

CREATE INDEX d19_scoped_sync_clients_fk1 ON photara_private.scoped_sync_clients (stream_id,epoch);
