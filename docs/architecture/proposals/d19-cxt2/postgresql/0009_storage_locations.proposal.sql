-- INERT PROPOSAL ONLY — CXT2, accepted design 2026-09-12.
-- NOT AN INSTALLED, APPLIED, OR RUNNABLE MIGRATION. DO NOT EXECUTE.
-- Review text outside every runtime migration directory; no runner references it.
-- Implementation/adaptation and disposable execution require separate CXT3 scope.
-- Read ../README.md and ../RESPONSIBILITIES.md before reviewing this SQL.
-- Reservations map to D19_STATIC_SCHEMA_DELTA.md; baseline bytes remain intact.

-- S1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.storage_location_specs (
  library_id uuid NOT NULL CHECK (library_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  storage_root_id uuid NOT NULL CHECK (storage_root_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  storage_kind text COLLATE "C" NOT NULL,
  provider_id text COLLATE "C",
  supported_rights_json jsonb NOT NULL CHECK (jsonb_typeof(supported_rights_json)='array'),
  record_schema integer NOT NULL,
  revision bigint NOT NULL CHECK (revision>=1),
  created_at timestamptz(3) NOT NULL,
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (library_id,storage_root_id),
  FOREIGN KEY (library_id,storage_root_id) REFERENCES photara.storage_roots (library_id,storage_root_id) ON DELETE RESTRICT,
  CHECK (storage_kind IN ('filesystem','provider')),
  CHECK ((storage_kind='provider')=(provider_id IS NOT NULL)),
  FOREIGN KEY (library_id) REFERENCES photara.libraries (library_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
);

-- S2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.storage_slots (
  slot_id uuid NOT NULL CHECK (slot_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  library_id uuid NOT NULL CHECK (library_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  current_name text COLLATE "C" NOT NULL,
  display_name text COLLATE "C" NOT NULL,
  storage_root_id uuid NOT NULL CHECK (storage_root_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  state text COLLATE "C" NOT NULL,
  retired_at timestamptz(3),
  record_schema integer NOT NULL,
  revision bigint NOT NULL CHECK (revision>=1),
  created_at timestamptz(3) NOT NULL,
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (slot_id),
  UNIQUE (library_id,slot_id),
  FOREIGN KEY (library_id,storage_root_id) REFERENCES photara.storage_roots (library_id,storage_root_id) ON DELETE RESTRICT,
  CHECK (state IN ('active','tombstoned')),
  CHECK ((state='tombstoned')=(retired_at IS NOT NULL)),
  CHECK (current_name ~ '^[a-z][a-z0-9_]{0,63}$'),
  FOREIGN KEY (library_id) REFERENCES photara.libraries (library_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
);

CREATE INDEX d19_storage_slots_root ON photara.storage_slots (library_id,storage_root_id,state);

-- S3: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.storage_slot_names (
  library_id uuid NOT NULL CHECK (library_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  name text COLLATE "C" NOT NULL,
  slot_id uuid NOT NULL CHECK (slot_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  claimed_at timestamptz(3) NOT NULL,
  PRIMARY KEY (library_id,name),
  UNIQUE (library_id,slot_id,name),
  FOREIGN KEY (library_id,slot_id) REFERENCES photara.storage_slots (library_id,slot_id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
  CHECK (name ~ '^[a-z][a-z0-9_]{0,63}$'),
  FOREIGN KEY (library_id) REFERENCES photara.libraries (library_id) ON DELETE RESTRICT
);

ALTER TABLE photara.storage_slots ADD CONSTRAINT d19_slot_current_name FOREIGN KEY (library_id,slot_id,current_name) REFERENCES photara.storage_slot_names (library_id,slot_id,name) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

CREATE INDEX d19_storage_slots_current_name ON photara.storage_slots (library_id,slot_id,current_name);
