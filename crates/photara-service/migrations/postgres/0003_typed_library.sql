-- CXT3c executable service migration; promoted from S4.
CREATE TABLE photara.people (
  person_id uuid PRIMARY KEY,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  record_schema integer NOT NULL CHECK(record_schema=1),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  state text NOT NULL CHECK(state IN ('active','tombstoned','merged')),
  retired_at timestamptz,
  merged_into_id uuid,
  display_name text NOT NULL CHECK(length(btrim(display_name))>0),
  sort_key text COLLATE "C" NOT NULL CHECK(length(sort_key)>0),
  description text NOT NULL DEFAULT '',
  aliases jsonb NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(aliases)='array'),
  thumbnail_sha256 bytea,
  extensions jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(extensions)='object'),
  UNIQUE(library_id,person_id),
  CHECK((state='active' AND retired_at IS NULL AND merged_into_id IS NULL)
     OR (state='tombstoned' AND retired_at IS NOT NULL AND merged_into_id IS NULL)
     OR (state='merged' AND retired_at IS NOT NULL AND merged_into_id IS NOT NULL)),
  CHECK(merged_into_id IS NULL OR merged_into_id<>person_id),
  FOREIGN KEY(library_id,merged_into_id)
    REFERENCES photara.people(library_id,person_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,thumbnail_sha256)
    REFERENCES photara.library_media(library_id,sha256) ON DELETE RESTRICT
);
CREATE INDEX people_browse ON photara.people(library_id,sort_key,person_id) WHERE state='active';
CREATE INDEX people_redirects ON photara.people(library_id,merged_into_id) WHERE merged_into_id IS NOT NULL;
CREATE TABLE photara.organizations (
  organization_id uuid PRIMARY KEY,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  record_schema integer NOT NULL CHECK(record_schema=1),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  state text NOT NULL CHECK(state IN ('active','tombstoned','merged')),
  retired_at timestamptz,
  merged_into_id uuid,
  display_name text NOT NULL CHECK(length(btrim(display_name))>0),
  sort_key text COLLATE "C" NOT NULL CHECK(length(sort_key)>0),
  description text NOT NULL DEFAULT '',
  aliases jsonb NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(aliases)='array'),
  thumbnail_sha256 bytea,
  extensions jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(extensions)='object'),
  UNIQUE(library_id,organization_id),
  CHECK((state='active' AND retired_at IS NULL AND merged_into_id IS NULL)
     OR (state='tombstoned' AND retired_at IS NOT NULL AND merged_into_id IS NULL)
     OR (state='merged' AND retired_at IS NOT NULL AND merged_into_id IS NOT NULL)),
  CHECK(merged_into_id IS NULL OR merged_into_id<>organization_id),
  FOREIGN KEY(library_id,merged_into_id)
    REFERENCES photara.organizations(library_id,organization_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,thumbnail_sha256)
    REFERENCES photara.library_media(library_id,sha256) ON DELETE RESTRICT
);
CREATE INDEX organizations_browse ON photara.organizations(library_id,sort_key,organization_id) WHERE state='active';
CREATE INDEX organizations_redirects ON photara.organizations(library_id,merged_into_id) WHERE merged_into_id IS NOT NULL;
CREATE TABLE photara.social_profiles (
  social_profile_id uuid PRIMARY KEY,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  person_id uuid,
  organization_id uuid,
  provider_id text COLLATE "C" NOT NULL CHECK(length(provider_id)>0),
  subject_namespace text COLLATE "C",
  provider_subject_id text COLLATE "C",
  handle text CHECK(handle IS NULL OR length(handle)>0),
  display_name text NOT NULL DEFAULT '',
  profile_url text CHECK(profile_url IS NULL OR length(profile_url)>0),
  account_kind text NOT NULL CHECK(account_kind IN ('unknown','personal','creator','business','organization','service','other')),
  provider_account_kind text,
  verification_kind text NOT NULL CHECK(verification_kind IN ('unverified','user-asserted','provider-authorized')),
  verified_at timestamptz,
  provenance jsonb NOT NULL CHECK(jsonb_typeof(provenance)='object'),
  fetched_at timestamptz,
  refreshed_at timestamptz,
  next_refresh_after timestamptz,
  fetch_state text NOT NULL CHECK(fetch_state IN ('never','available','unavailable','revoked','expired','error')),
  avatar_policy text NOT NULL CHECK(avatar_policy IN ('none','cache-only','durable-consented')),
  avatar_consent_at timestamptz,
  avatar_expires_at timestamptz,
  avatar_media_sha256 bytea,
  record_schema integer NOT NULL CHECK(record_schema=1),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  state text NOT NULL CHECK(state IN ('active','tombstoned')),
  retired_at timestamptz,
  extensions jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(extensions)='object'),
  UNIQUE(library_id,social_profile_id),
  CHECK((person_id IS NOT NULL)<>(organization_id IS NOT NULL)),
  CHECK((subject_namespace IS NULL AND provider_subject_id IS NULL)
     OR (subject_namespace IS NOT NULL AND provider_subject_id IS NOT NULL
         AND length(subject_namespace)>0 AND length(provider_subject_id)>0)),
  CHECK(provider_subject_id IS NOT NULL OR handle IS NOT NULL OR profile_url IS NOT NULL),
  CHECK((verification_kind='unverified' AND verified_at IS NULL)
     OR (verification_kind<>'unverified' AND verified_at IS NOT NULL)),
  CHECK(avatar_policy='none' OR avatar_consent_at IS NOT NULL),
  CHECK(avatar_policy<>'cache-only' OR avatar_expires_at IS NOT NULL),
  CHECK(avatar_media_sha256 IS NULL OR avatar_policy='durable-consented'),
  CHECK((state='active' AND retired_at IS NULL) OR (state='tombstoned' AND retired_at IS NOT NULL)),
  FOREIGN KEY(library_id,person_id) REFERENCES photara.people(library_id,person_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,organization_id) REFERENCES photara.organizations(library_id,organization_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,avatar_media_sha256) REFERENCES photara.library_media(library_id,sha256) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX social_library_subject ON photara.social_profiles(library_id,provider_id,subject_namespace,provider_subject_id)
  WHERE provider_subject_id IS NOT NULL;
CREATE INDEX social_profiles_person ON photara.social_profiles(library_id,person_id,social_profile_id) WHERE state='active';
CREATE INDEX social_profiles_organization ON photara.social_profiles(library_id,organization_id,social_profile_id) WHERE state='active';

CREATE TABLE photara.person_capabilities (
  library_id uuid NOT NULL,
  person_id uuid NOT NULL,
  capability_id text COLLATE "C" NOT NULL CHECK(length(capability_id)>0),
  PRIMARY KEY(library_id,person_id,capability_id),
  FOREIGN KEY(library_id,person_id) REFERENCES photara.people(library_id,person_id) ON DELETE RESTRICT
);
CREATE INDEX person_capability_filter ON photara.person_capabilities(library_id,capability_id,person_id);
CREATE TABLE photara.person_labels (
  library_id uuid NOT NULL,
  person_id uuid NOT NULL,
  label_key text COLLATE "C" NOT NULL CHECK(length(label_key)>0),
  display_label text NOT NULL CHECK(length(display_label)>0),
  PRIMARY KEY(library_id,person_id,label_key),
  FOREIGN KEY(library_id,person_id) REFERENCES photara.people(library_id,person_id) ON DELETE RESTRICT
);
CREATE INDEX person_label_filter ON photara.person_labels(library_id,label_key,person_id);
CREATE TABLE photara.organization_labels (
  library_id uuid NOT NULL,
  organization_id uuid NOT NULL,
  label_key text COLLATE "C" NOT NULL CHECK(length(label_key)>0),
  display_label text NOT NULL CHECK(length(display_label)>0),
  PRIMARY KEY(library_id,organization_id,label_key),
  FOREIGN KEY(library_id,organization_id) REFERENCES photara.organizations(library_id,organization_id) ON DELETE RESTRICT
);
CREATE INDEX organization_label_filter ON photara.organization_labels(library_id,label_key,organization_id);

CREATE TABLE photara.person_organization_relationships (
  relationship_id uuid PRIMARY KEY,
  library_id uuid NOT NULL,
  person_id uuid NOT NULL,
  organization_id uuid NOT NULL,
  relationship_type text COLLATE "C" NOT NULL CHECK(length(relationship_type)>0),
  valid_from timestamptz,
  valid_until timestamptz,
  notes text NOT NULL DEFAULT '',
  labels jsonb NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(labels)='array'),
  record_schema integer NOT NULL CHECK(record_schema=1),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  state text NOT NULL CHECK(state IN ('active','tombstoned')),
  retired_at timestamptz,
  extensions jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(extensions)='object'),
  CHECK(valid_from IS NULL OR valid_until IS NULL OR valid_until>valid_from),
  CHECK((state='tombstoned')=(retired_at IS NOT NULL)),
  FOREIGN KEY(library_id,person_id) REFERENCES photara.people(library_id,person_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,organization_id) REFERENCES photara.organizations(library_id,organization_id) ON DELETE RESTRICT
);
CREATE INDEX relationships_pair_role ON photara.person_organization_relationships(
  library_id,person_id,organization_id,relationship_type) WHERE state='active';
CREATE INDEX relationships_organization ON photara.person_organization_relationships(
  library_id,organization_id,person_id) WHERE state='active';

CREATE TABLE photara.location_kinds (
  location_kind_id uuid PRIMARY KEY,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  canonical_key text COLLATE "C" NOT NULL CHECK(length(canonical_key)>0),
  canonical_display text NOT NULL CHECK(length(btrim(canonical_display))>0),
  description text NOT NULL DEFAULT '',
  thumbnail_sha256 bytea,
  record_schema integer NOT NULL CHECK(record_schema=1),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  state text NOT NULL CHECK(state IN ('active','tombstoned','merged')),
  retired_at timestamptz,
  merged_into_id uuid,
  extensions jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(extensions)='object'),
  UNIQUE(library_id,location_kind_id),
  claim_owner_id uuid GENERATED ALWAYS AS (CASE WHEN state<>'merged' THEN location_kind_id END) STORED,
  required_canonical_key text COLLATE "C" GENERATED ALWAYS AS (CASE WHEN state<>'merged' THEN canonical_key END) STORED,
  retirement_terms jsonb CHECK(retirement_terms IS NULL OR jsonb_typeof(retirement_terms)='array'),
  UNIQUE(library_id,claim_owner_id),
  CHECK((state='active')=(retirement_terms IS NULL)),
  CHECK((state='active' AND retired_at IS NULL AND merged_into_id IS NULL)
     OR (state='tombstoned' AND retired_at IS NOT NULL AND merged_into_id IS NULL)
     OR (state='merged' AND retired_at IS NOT NULL AND merged_into_id IS NOT NULL)),
  CHECK(merged_into_id IS NULL OR merged_into_id<>location_kind_id),
  FOREIGN KEY(library_id,merged_into_id)
    REFERENCES photara.location_kinds(library_id,location_kind_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,thumbnail_sha256)
    REFERENCES photara.library_media(library_id,sha256) ON DELETE RESTRICT
);
CREATE TABLE photara.location_kind_terms (
  library_id uuid NOT NULL,
  term_key text COLLATE "C" NOT NULL CHECK(length(term_key)>0),
  location_kind_id uuid NOT NULL,
  policy_version integer NOT NULL REFERENCES photara.normalization_policies ON DELETE RESTRICT,
  spellings jsonb NOT NULL CHECK(jsonb_typeof(spellings)='array' AND jsonb_array_length(spellings)>0),
  PRIMARY KEY(library_id,term_key),
  UNIQUE(library_id,location_kind_id,term_key),
  FOREIGN KEY(library_id,location_kind_id)
    REFERENCES photara.location_kinds(library_id,claim_owner_id)
    DEFERRABLE INITIALLY DEFERRED
);
ALTER TABLE photara.location_kinds ADD CONSTRAINT kind_canonical_claim
  FOREIGN KEY(library_id,location_kind_id,required_canonical_key)
  REFERENCES photara.location_kind_terms(library_id,location_kind_id,term_key)
  DEFERRABLE INITIALLY DEFERRED;
CREATE INDEX kind_browse ON photara.location_kinds(library_id,canonical_key,location_kind_id) WHERE state='active';
CREATE INDEX kind_redirects ON photara.location_kinds(library_id,merged_into_id) WHERE merged_into_id IS NOT NULL;

CREATE TABLE photara.locations (
  location_id uuid PRIMARY KEY,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  location_kind_id uuid NOT NULL,
  parent_location_id uuid,
  display_name text NOT NULL CHECK(length(btrim(display_name))>0),
  sort_key text COLLATE "C" NOT NULL CHECK(length(sort_key)>0),
  description text NOT NULL DEFAULT '',
  aliases jsonb NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(aliases)='array'),
  address jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(address)='object'),
  latitude double precision CHECK(latitude BETWEEN -90 AND 90),
  longitude double precision CHECK(longitude BETWEEN -180 AND 180),
  thumbnail_sha256 bytea,
  record_schema integer NOT NULL CHECK(record_schema=1),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  state text NOT NULL CHECK(state IN ('active','tombstoned','merged')),
  retired_at timestamptz,
  merged_into_id uuid,
  extensions jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(extensions)='object'),
  UNIQUE(library_id,location_id),
  CHECK((latitude IS NULL)=(longitude IS NULL)),
  CHECK(parent_location_id IS NULL OR parent_location_id<>location_id),
  CHECK(merged_into_id IS NULL OR merged_into_id<>location_id),
  CHECK((state='active' AND retired_at IS NULL AND merged_into_id IS NULL)
     OR (state='tombstoned' AND retired_at IS NOT NULL AND merged_into_id IS NULL)
     OR (state='merged' AND retired_at IS NOT NULL AND merged_into_id IS NOT NULL)),
  FOREIGN KEY(library_id,location_kind_id)
    REFERENCES photara.location_kinds(library_id,location_kind_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,parent_location_id)
    REFERENCES photara.locations(library_id,location_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,merged_into_id)
    REFERENCES photara.locations(library_id,location_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,thumbnail_sha256)
    REFERENCES photara.library_media(library_id,sha256) ON DELETE RESTRICT
);
CREATE INDEX location_browse ON photara.locations(library_id,sort_key,location_id) WHERE state='active';
CREATE INDEX location_kind_filter ON photara.locations(library_id,location_kind_id,location_id) WHERE state='active';
CREATE INDEX location_children ON photara.locations(library_id,parent_location_id,location_id) WHERE state='active';
CREATE INDEX location_redirects ON photara.locations(library_id,merged_into_id) WHERE merged_into_id IS NOT NULL;
