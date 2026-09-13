-- INERT PROPOSAL ONLY — CXT2, accepted design 2026-09-12.
-- NOT AN INSTALLED, APPLIED, OR RUNNABLE MIGRATION. DO NOT EXECUTE.
-- Review text outside every runtime migration directory; no runner references it.
-- Implementation/adaptation and disposable execution require separate CXT3 scope.
-- Read ../README.md and ../RESPONSIBILITIES.md before reviewing this SQL.
-- Reservations map to D19_STATIC_SCHEMA_DELTA.md; baseline bytes remain intact.

-- H4: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE device_context_snapshots (
  snapshot_id BLOB NOT NULL CHECK (length(snapshot_id)=16 AND snapshot_id<>zeroblob(16)),
  device_id BLOB NOT NULL CHECK (length(device_id)=16 AND device_id<>zeroblob(16)),
  project_id BLOB NOT NULL CHECK (length(project_id)=16 AND project_id<>zeroblob(16)),
  run_id BLOB NOT NULL CHECK (length(run_id)=16 AND run_id<>zeroblob(16)),
  context_canonical BLOB NOT NULL,
  context_sha256 BLOB NOT NULL CHECK (length(context_sha256)=32),
  content_digest BLOB NOT NULL CHECK (length(content_digest)=32),
  created_at INTEGER NOT NULL,
  PRIMARY KEY (snapshot_id),
  FOREIGN KEY (device_id) REFERENCES local_device (device_id) ON DELETE RESTRICT,
  UNIQUE (device_id,project_id,run_id,snapshot_id),
  CHECK (length(context_canonical)<=1048576)
) STRICT;

CREATE INDEX d19_device_context_snapshots_run ON device_context_snapshots (project_id,run_id);

-- O1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE context_apply_intents (
  operation_id BLOB NOT NULL CHECK (length(operation_id)=16 AND operation_id<>zeroblob(16)),
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  target_project_id BLOB CHECK (target_project_id IS NULL OR (length(target_project_id)=16 AND target_project_id<>zeroblob(16))),
  proposal_id BLOB NOT NULL CHECK (length(proposal_id)=16 AND proposal_id<>zeroblob(16)),
  request_canonical BLOB NOT NULL,
  request_sha256 BLOB NOT NULL CHECK (length(request_sha256)=32),
  source_run_id BLOB NOT NULL CHECK (length(source_run_id)=16 AND source_run_id<>zeroblob(16)),
  source_attempt_id BLOB NOT NULL CHECK (length(source_attempt_id)=16 AND source_attempt_id<>zeroblob(16)),
  target_authority TEXT NOT NULL,
  expected_commit_id BLOB CHECK (expected_commit_id IS NULL OR (length(expected_commit_id)=16 AND expected_commit_id<>zeroblob(16))),
  expected_commit_sha256 BLOB CHECK (expected_commit_sha256 IS NULL OR (length(expected_commit_sha256)=32)),
  state TEXT NOT NULL,
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (operation_id),
  UNIQUE (library_id,operation_id),
  CHECK (target_authority IN ('library','project')),
  CHECK (state IN ('prepared','awaiting-publication','settled','unknown')),
  CHECK ((expected_commit_id IS NULL)=(expected_commit_sha256 IS NULL)),
  CHECK ((target_authority='project')=(target_project_id IS NOT NULL)),
  CHECK ((target_authority='project')=(expected_commit_id IS NOT NULL)),
  CHECK (length(request_canonical)<=1048576),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

CREATE INDEX d19_context_apply_intents_state ON context_apply_intents (library_id,state,operation_id);

-- O2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE context_apply_receipts (
  receipt_id BLOB NOT NULL CHECK (length(receipt_id)=16 AND receipt_id<>zeroblob(16)),
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  operation_id BLOB NOT NULL CHECK (length(operation_id)=16 AND operation_id<>zeroblob(16)),
  observation_kind TEXT NOT NULL,
  result_canonical BLOB NOT NULL,
  result_sha256 BLOB NOT NULL CHECK (length(result_sha256)=32),
  package_commit_id BLOB CHECK (package_commit_id IS NULL OR (length(package_commit_id)=16 AND package_commit_id<>zeroblob(16))),
  package_commit_sha256 BLOB CHECK (package_commit_sha256 IS NULL OR (length(package_commit_sha256)=32)),
  observed_at INTEGER NOT NULL,
  PRIMARY KEY (receipt_id),
  FOREIGN KEY (library_id,operation_id) REFERENCES context_apply_intents (library_id,operation_id) ON DELETE RESTRICT,
  CHECK (observation_kind IN ('local-applied','package-published','rejected','conflict','unknown')),
  CHECK ((package_commit_id IS NULL)=(package_commit_sha256 IS NULL)),
  CHECK (observation_kind<>'package-published' OR package_commit_id IS NOT NULL),
  CHECK (observation_kind<>'local-applied' OR package_commit_id IS NULL),
  CHECK (length(result_canonical)<=4194304),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT
) STRICT;

CREATE UNIQUE INDEX d19_context_apply_receipts_final ON context_apply_receipts (operation_id) WHERE observation_kind IN ('local-applied','package-published','rejected','conflict');

CREATE INDEX d19_context_apply_receipts_operation ON context_apply_receipts (library_id,operation_id,observed_at,receipt_id);
