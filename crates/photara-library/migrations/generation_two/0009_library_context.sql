-- CXT3b executable local migration, promoted from accepted D19 CXT2.
-- V1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE library_variables (
  variable_id BLOB NOT NULL CHECK (length(variable_id)=16 AND variable_id<>zeroblob(16)),
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  namespace TEXT NOT NULL,
  current_name TEXT NOT NULL,
  display_name TEXT NOT NULL,
  description TEXT NOT NULL,
  value_type_id TEXT NOT NULL,
  value_type_version INTEGER NOT NULL,
  schema_id TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  default_binding_json TEXT CHECK (default_binding_json IS NULL OR (CASE WHEN json_valid(default_binding_json) THEN json_type(default_binding_json)='object' ELSE 0 END)),
  allowed_overrides_json TEXT NOT NULL CHECK (CASE WHEN json_valid(allowed_overrides_json) THEN json_type(allowed_overrides_json)='array' ELSE 0 END),
  sensitivity TEXT NOT NULL,
  portability TEXT NOT NULL,
  provenance_json TEXT NOT NULL CHECK (CASE WHEN json_valid(provenance_json) THEN json_type(provenance_json)='object' ELSE 0 END),
  state TEXT NOT NULL,
  retired_at INTEGER,
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (variable_id),
  UNIQUE (library_id,variable_id),
  CHECK (state IN ('active','tombstoned')),
  CHECK ((state='tombstoned')=(retired_at IS NOT NULL)),
  CHECK (length(current_name) BETWEEN 1 AND 64 AND substr(current_name,1,1) GLOB '[a-z]' AND current_name NOT GLOB '*[^a-z0-9_]*'),
  CHECK (value_type_version>0 AND schema_version>0),
  CHECK (sensitivity IN ('ordinary','personal','restricted')),
  CHECK (portability IN ('portable','capture-consent-required','host-only')),
  FOREIGN KEY (library_id,variable_id,current_name) REFERENCES library_variable_names (library_id,variable_id,name) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

CREATE INDEX d19_library_variables_browse ON library_variables (library_id,state,current_name,variable_id);

-- V2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE library_variable_values (
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  variable_id BLOB NOT NULL CHECK (length(variable_id)=16 AND variable_id<>zeroblob(16)),
  value_id BLOB NOT NULL CHECK (length(value_id)=16 AND value_id<>zeroblob(16)),
  is_present INTEGER NOT NULL CHECK (is_present IN (0,1)),
  binding_json TEXT CHECK (binding_json IS NULL OR (CASE WHEN json_valid(binding_json) THEN json_type(binding_json)='object' ELSE 0 END)),
  origin TEXT NOT NULL,
  source_project_id BLOB CHECK (source_project_id IS NULL OR (length(source_project_id)=16 AND source_project_id<>zeroblob(16))),
  source_run_id BLOB CHECK (source_run_id IS NULL OR (length(source_run_id)=16 AND source_run_id<>zeroblob(16))),
  source_attempt_id BLOB CHECK (source_attempt_id IS NULL OR (length(source_attempt_id)=16 AND source_attempt_id<>zeroblob(16))),
  source_operation_id BLOB CHECK (source_operation_id IS NULL OR (length(source_operation_id)=16 AND source_operation_id<>zeroblob(16))),
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (library_id,variable_id),
  UNIQUE (value_id),
  FOREIGN KEY (library_id,variable_id) REFERENCES library_variables (library_id,variable_id) ON DELETE RESTRICT,
  CHECK ((is_present=1)=(binding_json IS NOT NULL)),
  CHECK (origin IN ('manual','command','node-proposal','imported')),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT
) STRICT;

-- V3: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE library_variable_names (
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  name TEXT NOT NULL,
  variable_id BLOB NOT NULL CHECK (length(variable_id)=16 AND variable_id<>zeroblob(16)),
  claimed_at INTEGER NOT NULL,
  PRIMARY KEY (library_id,name),
  UNIQUE (library_id,variable_id,name),
  FOREIGN KEY (library_id,variable_id) REFERENCES library_variables (library_id,variable_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
  CHECK (length(name) BETWEEN 1 AND 64 AND substr(name,1,1) GLOB '[a-z]' AND name NOT GLOB '*[^a-z0-9_]*'),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT
) STRICT;

-- V4: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE library_expressions (
  expression_id BLOB NOT NULL CHECK (length(expression_id)=16 AND expression_id<>zeroblob(16)),
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  owner_variable_id BLOB NOT NULL CHECK (length(owner_variable_id)=16 AND owner_variable_id<>zeroblob(16)),
  language TEXT NOT NULL,
  container_mode TEXT NOT NULL,
  compiler_version TEXT NOT NULL,
  source_utf8 BLOB NOT NULL,
  source_sha256 BLOB NOT NULL CHECK (length(source_sha256)=32),
  ast_canonical BLOB NOT NULL,
  ast_sha256 BLOB NOT NULL CHECK (length(ast_sha256)=32),
  result_type_id TEXT NOT NULL,
  result_type_version INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (expression_id),
  UNIQUE (library_id,expression_id),
  FOREIGN KEY (library_id,owner_variable_id) REFERENCES library_variables (library_id,variable_id) ON DELETE RESTRICT,
  CHECK (language='photara.expression.v1'),
  CHECK (container_mode IN ('expression','template')),
  CHECK (result_type_version>0),
  CHECK (length(source_utf8)<=16384),
  CHECK (length(ast_canonical)<=65536),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT
) STRICT;

CREATE INDEX d19_library_expressions_owner ON library_expressions (library_id,owner_variable_id,expression_id);

-- V5: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE library_expression_dependencies (
  library_id BLOB NOT NULL CHECK (length(library_id)=16 AND library_id<>zeroblob(16)),
  expression_id BLOB NOT NULL CHECK (length(expression_id)=16 AND expression_id<>zeroblob(16)),
  ordinal INTEGER NOT NULL,
  dependency_kind TEXT NOT NULL,
  variable_id BLOB CHECK (variable_id IS NULL OR (length(variable_id)=16 AND variable_id<>zeroblob(16))),
  slot_id BLOB CHECK (slot_id IS NULL OR (length(slot_id)=16 AND slot_id<>zeroblob(16))),
  expected_type_id TEXT NOT NULL,
  expected_type_version INTEGER NOT NULL,
  PRIMARY KEY (library_id,expression_id,ordinal),
  FOREIGN KEY (library_id,expression_id) REFERENCES library_expressions (library_id,expression_id) ON DELETE RESTRICT,
  FOREIGN KEY (library_id,variable_id) REFERENCES library_variables (library_id,variable_id) ON DELETE RESTRICT,
  FOREIGN KEY (library_id,slot_id) REFERENCES storage_slots (library_id,slot_id) ON DELETE RESTRICT,
  CHECK (ordinal BETWEEN 0 AND 255 AND expected_type_version>0),
  CHECK ((dependency_kind='variable' AND variable_id IS NOT NULL AND slot_id IS NULL) OR (dependency_kind='storage-slot' AND slot_id IS NOT NULL AND variable_id IS NULL)),
  FOREIGN KEY (library_id) REFERENCES libraries (library_id) ON DELETE RESTRICT
) STRICT;

CREATE UNIQUE INDEX d19_library_expression_dependencies_variable_id ON library_expression_dependencies (library_id,expression_id,variable_id) WHERE variable_id IS NOT NULL;

CREATE UNIQUE INDEX d19_library_expression_dependencies_slot_id ON library_expression_dependencies (library_id,expression_id,slot_id) WHERE slot_id IS NOT NULL;

CREATE INDEX d19_library_variables_fk8 ON library_variables (library_id,variable_id,current_name);

CREATE INDEX d19_library_expression_dependencies_fk2 ON library_expression_dependencies (library_id,variable_id);

CREATE INDEX d19_library_expression_dependencies_fk3 ON library_expression_dependencies (library_id,slot_id);
