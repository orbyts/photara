-- CXT3c executable service migration; promoted from S4.
CREATE TABLE photara.storage_roots (
  storage_root_id uuid PRIMARY KEY,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  display_name text NOT NULL CHECK(length(btrim(display_name))>0),
  label_key text COLLATE "C" NOT NULL CHECK(length(label_key)>0),
  purpose text NOT NULL CHECK(length(purpose)>0),
  revision bigint NOT NULL CHECK(revision>=1),
  state text NOT NULL CHECK(state IN ('active','tombstoned')),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  retired_at timestamptz,
  UNIQUE(library_id,storage_root_id),
  UNIQUE(library_id,label_key),
  CHECK((state='tombstoned')=(retired_at IS NOT NULL))
);
CREATE TABLE photara.project_catalog (
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  project_id uuid NOT NULL,
  visibility text NOT NULL CHECK(visibility IN ('visible','hidden')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  PRIMARY KEY(library_id,project_id)
);
CREATE TABLE photara.project_locators (
  locator_id uuid PRIMARY KEY,
  library_id uuid NOT NULL,
  project_id uuid NOT NULL,
  storage_root_id uuid,
  relative_path text COLLATE "C",
  state text NOT NULL CHECK(state IN ('active','retired')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  retired_at timestamptz,
  UNIQUE(library_id,project_id,locator_id),
  CHECK((storage_root_id IS NULL)=(relative_path IS NULL)),
  CHECK(relative_path IS NULL OR
    (length(relative_path)>0 AND left(relative_path,1)<>'/'
     AND strpos(relative_path,chr(92))=0 AND strpos(relative_path,':')=0
     AND strpos(relative_path,'//')=0
     AND strpos('/'||relative_path||'/','/../')=0
     AND strpos('/'||relative_path||'/','/./')=0)),
  CHECK((state='retired')=(retired_at IS NOT NULL)),
  FOREIGN KEY(library_id,project_id) REFERENCES photara.project_catalog(library_id,project_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,storage_root_id) REFERENCES photara.storage_roots(library_id,storage_root_id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX locator_rooted_path ON photara.project_locators(library_id,storage_root_id,relative_path)
  WHERE state='active' AND storage_root_id IS NOT NULL;
CREATE INDEX locator_project ON photara.project_locators(library_id,project_id);

CREATE TABLE photara.package_observations (
  observation_id uuid PRIMARY KEY,
  library_id uuid NOT NULL,
  project_id uuid NOT NULL,
  locator_id uuid NOT NULL,
  commit_id uuid NOT NULL,
  commit_sha256 bytea NOT NULL CHECK(octet_length(commit_sha256)=32),
  package_revision numeric(20,0) NOT NULL CHECK(package_revision BETWEEN 1 AND 18446744073709551615),
  index_schema integer NOT NULL CHECK(index_schema=1),
  title text NOT NULL,
  project_lifecycle text NOT NULL CHECK(project_lifecycle IN ('active','archived')),
  asset_count bigint NOT NULL CHECK(asset_count>=0),
  graph_count bigint NOT NULL CHECK(graph_count>=0),
  graph_summaries jsonb NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(graph_summaries)='array'),
  party_snapshots jsonb NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(party_snapshots)='array'),
  location_snapshots jsonb NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(location_snapshots)='array'),
  reported_by_account_id uuid NOT NULL,
  reported_by_device_id uuid NOT NULL,
  observed_at timestamptz NOT NULL,
  received_at timestamptz NOT NULL,
  projection_canonical bytea NOT NULL CHECK(octet_length(projection_canonical) BETWEEN 2 AND 1048576),
  projection_sha256 bytea NOT NULL CHECK(octet_length(projection_sha256)=32),
  UNIQUE(library_id,observation_id),
  UNIQUE(library_id,locator_id,commit_id,commit_sha256,index_schema),
  FOREIGN KEY(library_id,project_id,locator_id)
    REFERENCES photara.project_locators(library_id,project_id,locator_id) ON DELETE RESTRICT,
  FOREIGN KEY(reported_by_account_id,reported_by_device_id)
    REFERENCES photara_identity.devices(account_id,device_id) ON DELETE RESTRICT
);
CREATE INDEX observation_project ON photara.package_observations(library_id,project_id,received_at,observation_id);
CREATE INDEX observation_commit ON photara.package_observations(library_id,project_id,commit_id,commit_sha256);
