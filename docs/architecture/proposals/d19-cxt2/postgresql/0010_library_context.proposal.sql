-- INERT PROPOSAL ONLY — CXT2, accepted design 2026-09-12.
-- NOT AN INSTALLED, APPLIED, OR RUNNABLE MIGRATION. DO NOT EXECUTE.
-- Review text outside every runtime migration directory; no runner references it.
-- Implementation/adaptation and disposable execution require separate CXT3 scope.
-- Read ../README.md and ../RESPONSIBILITIES.md before reviewing this SQL.
-- Reservations map to D19_STATIC_SCHEMA_DELTA.md; baseline bytes remain intact.

-- V1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.library_variables (
  variable_id uuid NOT NULL CHECK (variable_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  library_id uuid NOT NULL CHECK (library_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  namespace text COLLATE "C" NOT NULL,
  current_name text COLLATE "C" NOT NULL,
  display_name text COLLATE "C" NOT NULL,
  description text COLLATE "C" NOT NULL,
  value_type_id text COLLATE "C" NOT NULL,
  value_type_version integer NOT NULL,
  schema_id text COLLATE "C" NOT NULL,
  schema_version integer NOT NULL,
  default_binding_json jsonb CHECK (default_binding_json IS NULL OR (jsonb_typeof(default_binding_json)='object')),
  allowed_overrides_json jsonb NOT NULL CHECK (jsonb_typeof(allowed_overrides_json)='array'),
  sensitivity text COLLATE "C" NOT NULL,
  portability text COLLATE "C" NOT NULL,
  provenance_json jsonb NOT NULL CHECK (jsonb_typeof(provenance_json)='object'),
  state text COLLATE "C" NOT NULL,
  retired_at timestamptz(3),
  record_schema integer NOT NULL,
  revision bigint NOT NULL CHECK (revision>=1),
  created_at timestamptz(3) NOT NULL,
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (variable_id),
  UNIQUE (library_id,variable_id),
  CHECK (state IN ('active','tombstoned')),
  CHECK ((state='tombstoned')=(retired_at IS NOT NULL)),
  CHECK (current_name ~ '^[a-z][a-z0-9_]{0,63}$'),
  CHECK (value_type_version>0 AND schema_version>0),
  CHECK (sensitivity IN ('ordinary','personal','restricted')),
  CHECK (portability IN ('portable','capture-consent-required','host-only')),
  CHECK ((sensitivity='ordinary' AND portability<>'host-only') OR (sensitivity='personal' AND portability='portable')),
  FOREIGN KEY (library_id) REFERENCES photara.libraries (library_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
);

CREATE INDEX d19_library_variables_browse ON photara.library_variables (library_id,state,current_name,variable_id);

-- V2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.library_variable_values (
  library_id uuid NOT NULL CHECK (library_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  variable_id uuid NOT NULL CHECK (variable_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  value_id uuid NOT NULL CHECK (value_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  is_present boolean NOT NULL,
  binding_json jsonb CHECK (binding_json IS NULL OR (jsonb_typeof(binding_json)='object')),
  origin text COLLATE "C" NOT NULL,
  source_project_id uuid CHECK (source_project_id IS NULL OR (source_project_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  source_run_id uuid CHECK (source_run_id IS NULL OR (source_run_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  source_attempt_id uuid CHECK (source_attempt_id IS NULL OR (source_attempt_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  source_operation_id uuid CHECK (source_operation_id IS NULL OR (source_operation_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (library_id,variable_id),
  UNIQUE (value_id),
  FOREIGN KEY (library_id,variable_id) REFERENCES photara.library_variables (library_id,variable_id) ON DELETE RESTRICT,
  CHECK (is_present=(binding_json IS NOT NULL)),
  CHECK (origin IN ('manual','command','node-proposal','imported')),
  FOREIGN KEY (library_id) REFERENCES photara.libraries (library_id) ON DELETE RESTRICT
);

-- V3: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.library_variable_names (
  library_id uuid NOT NULL CHECK (library_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  name text COLLATE "C" NOT NULL,
  variable_id uuid NOT NULL CHECK (variable_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  claimed_at timestamptz(3) NOT NULL,
  PRIMARY KEY (library_id,name),
  UNIQUE (library_id,variable_id,name),
  FOREIGN KEY (library_id,variable_id) REFERENCES photara.library_variables (library_id,variable_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
  CHECK (name ~ '^[a-z][a-z0-9_]{0,63}$'),
  FOREIGN KEY (library_id) REFERENCES photara.libraries (library_id) ON DELETE RESTRICT
);

ALTER TABLE photara.library_variables ADD CONSTRAINT d19_variable_current_name FOREIGN KEY (library_id,variable_id,current_name) REFERENCES photara.library_variable_names (library_id,variable_id,name) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

CREATE INDEX d19_library_variables_current_name ON photara.library_variables (library_id,variable_id,current_name);

-- V4: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.library_expressions (
  expression_id uuid NOT NULL CHECK (expression_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  library_id uuid NOT NULL CHECK (library_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  owner_variable_id uuid NOT NULL CHECK (owner_variable_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  language text COLLATE "C" NOT NULL,
  container_mode text COLLATE "C" NOT NULL,
  compiler_version text COLLATE "C" NOT NULL,
  source_utf8 bytea NOT NULL,
  source_sha256 bytea NOT NULL CHECK (octet_length(source_sha256)=32),
  ast_canonical bytea NOT NULL,
  ast_sha256 bytea NOT NULL CHECK (octet_length(ast_sha256)=32),
  result_type_id text COLLATE "C" NOT NULL,
  result_type_version integer NOT NULL,
  created_at timestamptz(3) NOT NULL,
  PRIMARY KEY (expression_id),
  UNIQUE (library_id,expression_id),
  FOREIGN KEY (library_id,owner_variable_id) REFERENCES photara.library_variables (library_id,variable_id) ON DELETE RESTRICT,
  CHECK (language='photara.expression.v1'),
  CHECK (container_mode IN ('expression','template')),
  CHECK (result_type_version>0),
  CHECK (octet_length(source_utf8)<=16384),
  CHECK (octet_length(ast_canonical)<=65536),
  FOREIGN KEY (library_id) REFERENCES photara.libraries (library_id) ON DELETE RESTRICT
);

CREATE INDEX d19_library_expressions_owner ON photara.library_expressions (library_id,owner_variable_id,expression_id);

-- V5: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.library_expression_dependencies (
  library_id uuid NOT NULL CHECK (library_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  expression_id uuid NOT NULL CHECK (expression_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  ordinal integer NOT NULL,
  dependency_kind text COLLATE "C" NOT NULL,
  variable_id uuid CHECK (variable_id IS NULL OR (variable_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  slot_id uuid CHECK (slot_id IS NULL OR (slot_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  expected_type_id text COLLATE "C" NOT NULL,
  expected_type_version integer NOT NULL,
  PRIMARY KEY (library_id,expression_id,ordinal),
  FOREIGN KEY (library_id,expression_id) REFERENCES photara.library_expressions (library_id,expression_id) ON DELETE RESTRICT,
  FOREIGN KEY (library_id,variable_id) REFERENCES photara.library_variables (library_id,variable_id) ON DELETE RESTRICT,
  FOREIGN KEY (library_id,slot_id) REFERENCES photara.storage_slots (library_id,slot_id) ON DELETE RESTRICT,
  CHECK (ordinal BETWEEN 0 AND 255 AND expected_type_version>0),
  CHECK ((dependency_kind='variable' AND variable_id IS NOT NULL AND slot_id IS NULL) OR (dependency_kind='storage-slot' AND slot_id IS NOT NULL AND variable_id IS NULL)),
  FOREIGN KEY (library_id) REFERENCES photara.libraries (library_id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX d19_library_expression_dependencies_variable_id ON photara.library_expression_dependencies (library_id,expression_id,variable_id) WHERE variable_id IS NOT NULL;

CREATE UNIQUE INDEX d19_library_expression_dependencies_slot_id ON photara.library_expression_dependencies (library_id,expression_id,slot_id) WHERE slot_id IS NOT NULL;

CREATE INDEX d19_library_expression_dependencies_fk2 ON photara.library_expression_dependencies (library_id,variable_id);

CREATE INDEX d19_library_expression_dependencies_fk3 ON photara.library_expression_dependencies (library_id,slot_id);
