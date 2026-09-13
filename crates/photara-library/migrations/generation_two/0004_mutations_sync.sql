CREATE TABLE sync_targets (
  target_id BLOB PRIMARY KEY CHECK(length(target_id)=16),
  library_id BLOB NOT NULL UNIQUE,
  account_id BLOB NOT NULL,
  backend TEXT NOT NULL CHECK(backend='photara-cloud'),
  environment_id TEXT NOT NULL CHECK(length(environment_id)>0),
  state TEXT NOT NULL CHECK(state IN ('enabled','paused','auth-required')),
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  UNIQUE(library_id,target_id),
  FOREIGN KEY(library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT,
  FOREIGN KEY(account_id) REFERENCES account_cache(account_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE mutations (
  mutation_id BLOB PRIMARY KEY CHECK(length(mutation_id)=16),
  library_id BLOB NOT NULL,
  source_device_id BLOB NOT NULL CHECK(length(source_device_id)=16),
  origin TEXT NOT NULL CHECK(origin IN ('local','remote')),
  command_schema INTEGER NOT NULL CHECK(command_schema>=1),
  primary_entity_kind TEXT NOT NULL CHECK(primary_entity_kind IN
    ('library','person','organization','social-profile','person-organization-relationship',
     'location-kind','location','storage-root','project-catalog','project-locator')),
  primary_entity_id BLOB NOT NULL CHECK(length(primary_entity_id)=16),
  created_at_ms INTEGER NOT NULL,
  envelope_json TEXT NOT NULL CHECK(json_valid(envelope_json) AND json_type(envelope_json)='object'),
  envelope_sha256 BLOB NOT NULL CHECK(length(envelope_sha256)=32),
  UNIQUE(library_id,mutation_id),
  FOREIGN KEY(library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE local_changes (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id BLOB NOT NULL,
  mutation_id BLOB NOT NULL,
  entity_kind TEXT NOT NULL CHECK(entity_kind IN
    ('library','person','organization','social-profile','person-organization-relationship',
     'location-kind','location','storage-root','project-catalog','project-locator')),
  entity_id BLOB NOT NULL CHECK(length(entity_id)=16),
  local_revision INTEGER NOT NULL CHECK(local_revision>=1),
  change_kind TEXT NOT NULL CHECK(change_kind IN ('create','update','tombstone','merge')),
  changed_at_ms INTEGER NOT NULL,
  post_state_json TEXT NOT NULL CHECK(json_valid(post_state_json) AND json_type(post_state_json)='object'),
  post_state_sha256 BLOB NOT NULL CHECK(length(post_state_sha256)=32),
  UNIQUE(library_id,entity_kind,entity_id,local_revision),
  FOREIGN KEY(library_id,mutation_id) REFERENCES mutations(library_id,mutation_id) ON DELETE RESTRICT
) STRICT;
CREATE INDEX local_changes_library_sequence ON local_changes(library_id,sequence);
CREATE INDEX local_changes_mutation ON local_changes(mutation_id,sequence);

CREATE TABLE mutation_baselines (
  library_id BLOB NOT NULL,
  mutation_id BLOB NOT NULL,
  entity_kind TEXT NOT NULL,
  entity_id BLOB NOT NULL CHECK(length(entity_id)=16),
  expected_server_revision TEXT,
  predecessor_mutation_id BLOB,
  PRIMARY KEY(mutation_id,entity_kind,entity_id),
  CHECK(expected_server_revision IS NULL OR length(expected_server_revision)>0),
  CHECK(expected_server_revision IS NULL OR predecessor_mutation_id IS NULL),
  CHECK(predecessor_mutation_id IS NULL OR predecessor_mutation_id<>mutation_id),
  FOREIGN KEY(library_id,mutation_id) REFERENCES mutations(library_id,mutation_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,predecessor_mutation_id) REFERENCES mutations(library_id,mutation_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE sync_object_state (
  target_id BLOB NOT NULL,
  library_id BLOB NOT NULL,
  entity_kind TEXT NOT NULL,
  entity_id BLOB NOT NULL CHECK(length(entity_id)=16),
  server_revision TEXT NOT NULL CHECK(length(server_revision)>0),
  synced_local_revision INTEGER NOT NULL CHECK(synced_local_revision>=1),
  base_snapshot_json TEXT NOT NULL CHECK(json_valid(base_snapshot_json) AND json_type(base_snapshot_json)='object'),
  base_snapshot_sha256 BLOB NOT NULL CHECK(length(base_snapshot_sha256)=32),
  PRIMARY KEY(target_id,entity_kind,entity_id),
  FOREIGN KEY(library_id,target_id) REFERENCES sync_targets(library_id,target_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE sync_outbox (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  target_id BLOB NOT NULL,
  library_id BLOB NOT NULL,
  mutation_id BLOB NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('queued','sending','acknowledged','rejected','conflict','superseded','discarded')),
  attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count>=0),
  not_before_ms INTEGER NOT NULL,
  claimed_session_id BLOB CHECK(claimed_session_id IS NULL OR length(claimed_session_id)=16),
  request_json TEXT CHECK(request_json IS NULL OR (json_valid(request_json) AND json_type(request_json)='object')),
  request_sha256 BLOB CHECK(request_sha256 IS NULL OR length(request_sha256)=32),
  acknowledged_at_ms INTEGER,
  last_error_code TEXT,
  UNIQUE(target_id,mutation_id),
  UNIQUE(library_id,target_id,sequence),
  CHECK((request_json IS NULL)=(request_sha256 IS NULL)),
  CHECK(state NOT IN ('sending','acknowledged') OR request_json IS NOT NULL),
  CHECK((state='acknowledged')=(acknowledged_at_ms IS NOT NULL)),
  FOREIGN KEY(library_id,target_id) REFERENCES sync_targets(library_id,target_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,mutation_id) REFERENCES mutations(library_id,mutation_id) ON DELETE RESTRICT
) STRICT;
CREATE INDEX sync_outbox_ready ON sync_outbox(target_id,not_before_ms,sequence)
  WHERE state='queued';

CREATE TABLE sync_inbox (
  batch_id BLOB PRIMARY KEY CHECK(length(batch_id)=16),
  target_id BLOB NOT NULL,
  library_id BLOB NOT NULL,
  stream_epoch BLOB NOT NULL CHECK(length(stream_epoch)=16),
  batch_sequence TEXT NOT NULL CHECK(length(batch_sequence) BETWEEN 1 AND 19 AND batch_sequence NOT GLOB '*[^0-9]*' AND substr(batch_sequence,1,1)<>'0' AND (length(batch_sequence)<19 OR batch_sequence<='9223372036854775807')),
  cursor_before TEXT,
  cursor_after TEXT NOT NULL CHECK(length(cursor_after)>0),
  payload_json TEXT NOT NULL CHECK(json_valid(payload_json) AND json_type(payload_json)='object'),
  payload_sha256 BLOB NOT NULL CHECK(length(payload_sha256)=32),
  received_at_ms INTEGER NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('received','applied','conflict','unsupported')),
  applied_at_ms INTEGER,
  last_error_code TEXT,
  UNIQUE(target_id,cursor_after),
  UNIQUE(target_id,stream_epoch,batch_sequence),
  UNIQUE(library_id,target_id,batch_id),
  CHECK((state='applied')=(applied_at_ms IS NOT NULL)),
  FOREIGN KEY(library_id,target_id) REFERENCES sync_targets(library_id,target_id) ON DELETE RESTRICT
) STRICT;
CREATE UNIQUE INDEX sync_inbox_one_pending_page ON sync_inbox(target_id)
  WHERE state<>'applied';

CREATE TABLE sync_cursors (
  target_id BLOB PRIMARY KEY,
  library_id BLOB NOT NULL,
  applied_cursor TEXT,
  local_revision INTEGER NOT NULL CHECK(local_revision>=1),
  updated_at_ms INTEGER NOT NULL,
  FOREIGN KEY(library_id,target_id) REFERENCES sync_targets(library_id,target_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE sync_conflicts (
  conflict_id BLOB PRIMARY KEY CHECK(length(conflict_id)=16),
  target_id BLOB NOT NULL,
  library_id BLOB NOT NULL,
  batch_id BLOB,
  outbox_sequence INTEGER,
  entity_kind TEXT NOT NULL,
  entity_id BLOB NOT NULL CHECK(length(entity_id)=16),
  base_json TEXT CHECK(base_json IS NULL OR (json_valid(base_json) AND json_type(base_json)='object')),
  local_json TEXT NOT NULL CHECK(json_valid(local_json) AND json_type(local_json)='object'),
  remote_json TEXT NOT NULL CHECK(json_valid(remote_json) AND json_type(remote_json)='object'),
  state TEXT NOT NULL CHECK(state IN ('open','resolved')),
  resolution_mutation_id BLOB,
  created_at_ms INTEGER NOT NULL,
  resolved_at_ms INTEGER,
  CHECK((batch_id IS NULL)<>(outbox_sequence IS NULL)),
  CHECK((state='resolved')=(resolved_at_ms IS NOT NULL)),
  FOREIGN KEY(library_id,target_id) REFERENCES sync_targets(library_id,target_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,target_id,batch_id) REFERENCES sync_inbox(library_id,target_id,batch_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,target_id,outbox_sequence) REFERENCES sync_outbox(library_id,target_id,sequence) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,resolution_mutation_id) REFERENCES mutations(library_id,mutation_id) ON DELETE RESTRICT
) STRICT;
CREATE INDEX sync_conflicts_open ON sync_conflicts(library_id,created_at_ms)
  WHERE state='open';

CREATE TABLE media_transfers (
  target_id BLOB NOT NULL,
  library_id BLOB NOT NULL,
  sha256 BLOB NOT NULL,
  direction TEXT NOT NULL CHECK(direction IN ('upload','download')),
  state TEXT NOT NULL CHECK(state IN ('queued','transferring','complete','failed')),
  attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count>=0),
  updated_at_ms INTEGER NOT NULL,
  last_error_code TEXT,
  PRIMARY KEY(target_id,sha256,direction),
  FOREIGN KEY(library_id,target_id) REFERENCES sync_targets(library_id,target_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,sha256) REFERENCES library_media(library_id,sha256) ON DELETE RESTRICT
) STRICT;
CREATE TABLE remote_mutation_receipts (
  target_id BLOB NOT NULL,
  library_id BLOB NOT NULL,
  mutation_id BLOB NOT NULL,
  request_sha256 BLOB NOT NULL CHECK(length(request_sha256)=32),
  outcome TEXT NOT NULL CHECK(outcome IN ('accepted','rejected','conflict')),
  response_json TEXT NOT NULL CHECK(json_valid(response_json) AND json_type(response_json)='object' AND length(CAST(response_json AS BLOB))<=4194304),
  response_sha256 BLOB NOT NULL CHECK(length(response_sha256)=32),
  accepted_epoch BLOB CHECK(accepted_epoch IS NULL OR length(accepted_epoch)=16),
  accepted_sequence TEXT CHECK(accepted_sequence IS NULL OR (length(accepted_sequence) BETWEEN 1 AND 19 AND accepted_sequence NOT GLOB '*[^0-9]*' AND substr(accepted_sequence,1,1)<>'0' AND (length(accepted_sequence)<19 OR accepted_sequence<='9223372036854775807'))),
  received_at_ms INTEGER NOT NULL,
  PRIMARY KEY(target_id,mutation_id),
  CHECK((outcome='accepted')=(accepted_epoch IS NOT NULL)),
  CHECK((outcome='accepted')=(accepted_sequence IS NOT NULL)),
  FOREIGN KEY(library_id,target_id) REFERENCES sync_targets(library_id,target_id) ON DELETE RESTRICT,
  FOREIGN KEY(target_id,mutation_id) REFERENCES sync_outbox(target_id,mutation_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE local_mutation_dispositions (
  library_id BLOB NOT NULL,
  mutation_id BLOB NOT NULL,
  disposition TEXT NOT NULL CHECK(disposition IN ('superseded','discarded')),
  replacement_mutation_id BLOB,
  reason_code TEXT NOT NULL CHECK(length(reason_code)>0),
  created_at_ms INTEGER NOT NULL,
  PRIMARY KEY(library_id,mutation_id),
  CHECK(replacement_mutation_id IS NULL OR replacement_mutation_id<>mutation_id),
  CHECK((disposition='superseded')=(replacement_mutation_id IS NOT NULL)),
  FOREIGN KEY(library_id,mutation_id) REFERENCES mutations(library_id,mutation_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,replacement_mutation_id) REFERENCES mutations(library_id,mutation_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE sync_snapshot_installs (
  installation_id BLOB PRIMARY KEY CHECK(length(installation_id)=16),
  target_id BLOB NOT NULL,
  library_id BLOB NOT NULL,
  server_snapshot_id BLOB NOT NULL CHECK(length(server_snapshot_id)=16),
  stream_epoch BLOB NOT NULL CHECK(length(stream_epoch)=16),
  high_water_cursor TEXT NOT NULL CHECK(length(high_water_cursor)>0),
  expected_local_cursor TEXT,
  payload_json TEXT NOT NULL CHECK(json_valid(payload_json) AND json_type(payload_json)='object' AND length(CAST(payload_json AS BLOB))<=16777216),
  payload_sha256 BLOB NOT NULL CHECK(length(payload_sha256)=32),
  state TEXT NOT NULL CHECK(state IN ('staged','installed','conflict','unsupported','superseded')),
  created_at_ms INTEGER NOT NULL,
  installed_at_ms INTEGER,
  UNIQUE(target_id,server_snapshot_id),
  CHECK((state='installed')=(installed_at_ms IS NOT NULL)),
  FOREIGN KEY(library_id,target_id) REFERENCES sync_targets(library_id,target_id) ON DELETE RESTRICT
) STRICT;
CREATE UNIQUE INDEX snapshot_one_pending ON sync_snapshot_installs(target_id)
  WHERE state IN ('staged','conflict','unsupported');
