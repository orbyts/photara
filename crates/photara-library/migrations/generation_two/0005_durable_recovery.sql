CREATE TABLE operation_intents (
  operation_id BLOB PRIMARY KEY CHECK(length(operation_id)=16),
  library_id BLOB NOT NULL,
  project_id BLOB CHECK(project_id IS NULL OR length(project_id)=16),
  operation_kind TEXT NOT NULL CHECK(operation_kind IN
    ('create-package','save-package','move-package','rebind-package',
     'collect-resources','publish-evidence','fork-package')),
  idempotency_key TEXT NOT NULL CHECK(length(idempotency_key)>0),
  request_json TEXT NOT NULL CHECK(json_valid(request_json) AND json_type(request_json)='object'),
  request_sha256 BLOB NOT NULL CHECK(length(request_sha256)=32),
  base_commit_id BLOB CHECK(base_commit_id IS NULL OR length(base_commit_id)=16),
  base_commit_sha256 BLOB CHECK(base_commit_sha256 IS NULL OR length(base_commit_sha256)=32),
  proposed_commit_id BLOB CHECK(proposed_commit_id IS NULL OR length(proposed_commit_id)=16),
  proposed_commit_sha256 BLOB CHECK(proposed_commit_sha256 IS NULL OR length(proposed_commit_sha256)=32),
  state TEXT NOT NULL CHECK(state IN
    ('prepared','executing','awaiting-package','committed','needs-recovery','failed','cancelled')),
  local_revision INTEGER NOT NULL CHECK(local_revision>=1),
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  last_error_code TEXT,
  UNIQUE(library_id,idempotency_key),
  CHECK((base_commit_id IS NULL)=(base_commit_sha256 IS NULL)),
  CHECK((proposed_commit_id IS NULL)=(proposed_commit_sha256 IS NULL)),
  FOREIGN KEY(library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT
) STRICT;
CREATE INDEX operation_intents_pending ON operation_intents(library_id,updated_at_ms)
  WHERE state IN ('prepared','executing','awaiting-package','needs-recovery');

CREATE TABLE operation_events (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  operation_id BLOB NOT NULL,
  phase TEXT NOT NULL CHECK(length(phase)>0),
  occurred_at_ms INTEGER NOT NULL,
  details_json TEXT NOT NULL CHECK(json_valid(details_json) AND json_type(details_json)='object'),
  FOREIGN KEY(operation_id) REFERENCES operation_intents(operation_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE recovery_items (
  recovery_item_id BLOB PRIMARY KEY CHECK(length(recovery_item_id)=16),
  operation_id BLOB NOT NULL,
  record_schema TEXT NOT NULL CHECK(length(record_schema)>0),
  payload_json TEXT NOT NULL CHECK(json_valid(payload_json) AND json_type(payload_json)='object'),
  payload_sha256 BLOB NOT NULL CHECK(length(payload_sha256)=32),
  durable_relative_path TEXT,
  blob_sha256 BLOB CHECK(blob_sha256 IS NULL OR length(blob_sha256)=32),
  byte_length INTEGER CHECK(byte_length IS NULL OR byte_length>=0),
  created_at_ms INTEGER NOT NULL,
  CHECK((durable_relative_path IS NULL)=(blob_sha256 IS NULL)),
  CHECK((durable_relative_path IS NULL)=(byte_length IS NULL)),
  CHECK(durable_relative_path IS NULL OR
    (length(durable_relative_path)>0 AND substr(durable_relative_path,1,1)<>'/'
     AND instr(durable_relative_path,char(92))=0
     AND instr(durable_relative_path,':')=0 AND instr(durable_relative_path,char(0))=0
     AND instr(durable_relative_path,'//')=0
     AND instr('/'||durable_relative_path||'/','/../')=0
     AND instr('/'||durable_relative_path||'/','/./')=0)),
  FOREIGN KEY(operation_id) REFERENCES operation_intents(operation_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE recovery_publications (
  recovery_item_id BLOB PRIMARY KEY,
  project_id BLOB NOT NULL CHECK(length(project_id)=16),
  commit_id BLOB NOT NULL CHECK(length(commit_id)=16),
  commit_sha256 BLOB NOT NULL CHECK(length(commit_sha256)=32),
  verified_at_ms INTEGER NOT NULL,
  FOREIGN KEY(recovery_item_id) REFERENCES recovery_items(recovery_item_id) ON DELETE RESTRICT
) STRICT;
