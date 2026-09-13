CREATE TABLE people (
    person_id BLOB PRIMARY KEY CHECK (length(person_id) = 16),
    library_id BLOB NOT NULL,
    record_schema INTEGER NOT NULL CHECK (record_schema = 1),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'tombstoned', 'merged')),
    retired_at_ms INTEGER,
    merged_into_id BLOB CHECK (length(merged_into_id) = 16),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    sort_key TEXT NOT NULL CHECK (length(sort_key) > 0),
    description TEXT NOT NULL DEFAULT '',
    aliases_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(aliases_json) AND json_type(aliases_json) = 'array'),
    thumbnail_sha256 BLOB,
    extensions_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(extensions_json) AND json_type(extensions_json) = 'object'),
    UNIQUE (library_id, person_id),
    CHECK ((state = 'active' AND retired_at_ms IS NULL AND merged_into_id IS NULL)
        OR (state = 'tombstoned' AND retired_at_ms IS NOT NULL AND merged_into_id IS NULL)
        OR (state = 'merged' AND retired_at_ms IS NOT NULL AND merged_into_id IS NOT NULL)),
    CHECK (merged_into_id IS NULL OR merged_into_id <> person_id),
    FOREIGN KEY (library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, merged_into_id) REFERENCES people(library_id, person_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, thumbnail_sha256) REFERENCES library_media(library_id, sha256) ON DELETE RESTRICT
) STRICT;

CREATE INDEX people_browse ON people(library_id, sort_key, person_id) WHERE state = 'active';
CREATE INDEX people_merge_target ON people(library_id, merged_into_id) WHERE merged_into_id IS NOT NULL;

CREATE TABLE organizations (
    organization_id BLOB PRIMARY KEY CHECK (length(organization_id) = 16),
    library_id BLOB NOT NULL,
    record_schema INTEGER NOT NULL CHECK (record_schema = 1),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'tombstoned', 'merged')),
    retired_at_ms INTEGER,
    merged_into_id BLOB CHECK (length(merged_into_id) = 16),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    sort_key TEXT NOT NULL CHECK (length(sort_key) > 0),
    description TEXT NOT NULL DEFAULT '',
    aliases_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(aliases_json) AND json_type(aliases_json) = 'array'),
    thumbnail_sha256 BLOB,
    extensions_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(extensions_json) AND json_type(extensions_json) = 'object'),
    UNIQUE (library_id, organization_id),
    CHECK ((state = 'active' AND retired_at_ms IS NULL AND merged_into_id IS NULL)
        OR (state = 'tombstoned' AND retired_at_ms IS NOT NULL AND merged_into_id IS NULL)
        OR (state = 'merged' AND retired_at_ms IS NOT NULL AND merged_into_id IS NOT NULL)),
    CHECK (merged_into_id IS NULL OR merged_into_id <> organization_id),
    FOREIGN KEY (library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, merged_into_id) REFERENCES organizations(library_id, organization_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, thumbnail_sha256) REFERENCES library_media(library_id, sha256) ON DELETE RESTRICT
) STRICT;

CREATE INDEX organizations_browse ON organizations(library_id, sort_key, organization_id) WHERE state = 'active';
CREATE INDEX organizations_merge_target ON organizations(library_id, merged_into_id) WHERE merged_into_id IS NOT NULL;


CREATE TABLE social_profiles (
    social_profile_id BLOB PRIMARY KEY CHECK (length(social_profile_id) = 16),
    library_id BLOB NOT NULL,
    person_id BLOB,
    organization_id BLOB,
    provider_id TEXT NOT NULL CHECK (length(provider_id) > 0),
    subject_namespace TEXT,
    provider_subject_id TEXT,
    handle TEXT CHECK (handle IS NULL OR length(handle) > 0),
    display_name TEXT NOT NULL DEFAULT '',
    profile_url TEXT CHECK (profile_url IS NULL OR length(profile_url) > 0),
    account_kind TEXT NOT NULL CHECK (account_kind IN ('unknown','personal','creator','business','organization','service','other')),
    provider_account_kind TEXT,
    verification_kind TEXT NOT NULL CHECK (verification_kind IN ('unverified','user-asserted','provider-authorized')),
    verified_at_ms INTEGER,
    provenance_json TEXT NOT NULL CHECK (json_valid(provenance_json) AND json_type(provenance_json) = 'object'),
    fetched_at_ms INTEGER,
    refreshed_at_ms INTEGER,
    next_refresh_after_ms INTEGER,
    fetch_state TEXT NOT NULL CHECK (fetch_state IN ('never','available','unavailable','revoked','expired','error')),
    avatar_policy TEXT NOT NULL CHECK (avatar_policy IN ('none','cache-only','durable-consented')),
    avatar_consent_at_ms INTEGER,
    avatar_expires_at_ms INTEGER,
    avatar_media_sha256 BLOB,
    record_schema INTEGER NOT NULL CHECK (record_schema = 1),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active','tombstoned')),
    retired_at_ms INTEGER,
    extensions_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(extensions_json) AND json_type(extensions_json) = 'object'),
    UNIQUE (library_id, social_profile_id),
    CHECK ((person_id IS NOT NULL) <> (organization_id IS NOT NULL)),
    CHECK ((subject_namespace IS NULL AND provider_subject_id IS NULL)
        OR (length(subject_namespace) > 0 AND length(provider_subject_id) > 0
            AND subject_namespace IS NOT NULL AND provider_subject_id IS NOT NULL)),
    CHECK (provider_subject_id IS NOT NULL OR handle IS NOT NULL OR profile_url IS NOT NULL),
    CHECK ((verification_kind = 'unverified' AND verified_at_ms IS NULL)
        OR (verification_kind <> 'unverified' AND verified_at_ms IS NOT NULL)),
    CHECK (avatar_policy = 'none' OR avatar_consent_at_ms IS NOT NULL),
    CHECK (avatar_policy <> 'cache-only' OR avatar_expires_at_ms IS NOT NULL),
    CHECK (avatar_media_sha256 IS NULL OR avatar_policy = 'durable-consented'),
    CHECK ((state = 'active' AND retired_at_ms IS NULL) OR (state = 'tombstoned' AND retired_at_ms IS NOT NULL)),
    FOREIGN KEY (library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, person_id) REFERENCES people(library_id, person_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, organization_id) REFERENCES organizations(library_id, organization_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, avatar_media_sha256) REFERENCES library_media(library_id, sha256) ON DELETE RESTRICT
) STRICT;
CREATE UNIQUE INDEX social_library_subject ON social_profiles(library_id,provider_id,subject_namespace,provider_subject_id)
    WHERE provider_subject_id IS NOT NULL;
CREATE INDEX social_profiles_person ON social_profiles(library_id,person_id,social_profile_id) WHERE state = 'active';
CREATE INDEX social_profiles_organization ON social_profiles(library_id,organization_id,social_profile_id) WHERE state = 'active';

CREATE TABLE person_capabilities (
    library_id BLOB NOT NULL,
    person_id BLOB NOT NULL,
    capability_id TEXT NOT NULL CHECK (length(capability_id) > 0),
    PRIMARY KEY (library_id, person_id, capability_id),
    FOREIGN KEY (library_id, person_id) REFERENCES people(library_id, person_id) ON DELETE RESTRICT
) STRICT;
CREATE INDEX people_by_capability ON person_capabilities(library_id, capability_id, person_id);

CREATE TABLE person_labels (
    library_id BLOB NOT NULL,
    person_id BLOB NOT NULL,
    label_key TEXT NOT NULL CHECK (length(label_key) > 0),
    display_label TEXT NOT NULL CHECK (length(trim(display_label)) > 0),
    PRIMARY KEY (library_id, person_id, label_key),
    FOREIGN KEY (library_id, person_id) REFERENCES people(library_id, person_id) ON DELETE RESTRICT
) STRICT;
CREATE INDEX people_by_label ON person_labels(library_id, label_key, person_id);

CREATE TABLE organization_labels (
    library_id BLOB NOT NULL,
    organization_id BLOB NOT NULL,
    label_key TEXT NOT NULL CHECK (length(label_key) > 0),
    display_label TEXT NOT NULL CHECK (length(trim(display_label)) > 0),
    PRIMARY KEY (library_id, organization_id, label_key),
    FOREIGN KEY (library_id, organization_id) REFERENCES organizations(library_id, organization_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE person_organization_relationships (
    relationship_id BLOB PRIMARY KEY CHECK (length(relationship_id) = 16),
    library_id BLOB NOT NULL,
    person_id BLOB NOT NULL,
    organization_id BLOB NOT NULL,
    relationship_type TEXT NOT NULL CHECK (length(relationship_type) > 0),
    valid_from_ms INTEGER,
    valid_until_ms INTEGER,
    notes TEXT NOT NULL DEFAULT '',
    labels_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(labels_json) AND json_type(labels_json) = 'array'),
    record_schema INTEGER NOT NULL CHECK (record_schema = 1),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'tombstoned')),
    retired_at_ms INTEGER,
    extensions_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(extensions_json) AND json_type(extensions_json) = 'object'),
    CHECK (valid_from_ms IS NULL OR valid_until_ms IS NULL OR valid_until_ms > valid_from_ms),
    CHECK ((state = 'active' AND retired_at_ms IS NULL) OR (state = 'tombstoned' AND retired_at_ms IS NOT NULL)),
    FOREIGN KEY (library_id, person_id) REFERENCES people(library_id, person_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, organization_id) REFERENCES organizations(library_id, organization_id) ON DELETE RESTRICT
) STRICT;
CREATE INDEX relationships_pair_role ON person_organization_relationships(
    library_id, person_id, organization_id, relationship_type
) WHERE state = 'active';
CREATE INDEX relationships_by_organization ON person_organization_relationships(
    library_id, organization_id, person_id
) WHERE state = 'active';

CREATE TABLE location_kinds (
    location_kind_id BLOB PRIMARY KEY CHECK (length(location_kind_id) = 16),
    library_id BLOB NOT NULL,
    canonical_key TEXT NOT NULL CHECK (length(canonical_key) > 0),
    canonical_display TEXT NOT NULL CHECK (length(trim(canonical_display)) > 0),
    description TEXT NOT NULL DEFAULT '',
    thumbnail_sha256 BLOB,
    record_schema INTEGER NOT NULL CHECK (record_schema = 1),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'tombstoned', 'merged')),
    retired_at_ms INTEGER,
    merged_into_id BLOB CHECK (length(merged_into_id) = 16),
    extensions_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(extensions_json) AND json_type(extensions_json) = 'object'),
    claim_owner_id BLOB GENERATED ALWAYS AS (CASE WHEN state <> 'merged' THEN location_kind_id END) STORED,
    required_canonical_key TEXT GENERATED ALWAYS AS (CASE WHEN state <> 'merged' THEN canonical_key END) STORED,
    retirement_terms_json TEXT CHECK (retirement_terms_json IS NULL OR (json_valid(retirement_terms_json) AND json_type(retirement_terms_json) = 'array')),
    UNIQUE (library_id, location_kind_id),
    UNIQUE (library_id, claim_owner_id),
    CHECK ((state = 'active') = (retirement_terms_json IS NULL)),
    CHECK ((state = 'active' AND retired_at_ms IS NULL AND merged_into_id IS NULL)
        OR (state = 'tombstoned' AND retired_at_ms IS NOT NULL AND merged_into_id IS NULL)
        OR (state = 'merged' AND retired_at_ms IS NOT NULL AND merged_into_id IS NOT NULL)),
    CHECK (merged_into_id IS NULL OR merged_into_id <> location_kind_id),
    FOREIGN KEY (library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, merged_into_id) REFERENCES location_kinds(library_id, location_kind_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, thumbnail_sha256) REFERENCES library_media(library_id, sha256) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, location_kind_id, required_canonical_key)
        REFERENCES location_kind_terms(library_id, location_kind_id, term_key)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TABLE location_kind_terms (
    library_id BLOB NOT NULL,
    term_key TEXT NOT NULL CHECK (length(term_key) > 0),
    location_kind_id BLOB NOT NULL,
    policy_version INTEGER NOT NULL,
    spellings_json TEXT NOT NULL CHECK (json_valid(spellings_json) AND json_type(spellings_json) = 'array' AND json_array_length(spellings_json) > 0),
    PRIMARY KEY (library_id, term_key),
    UNIQUE (library_id, location_kind_id, term_key),
    FOREIGN KEY (library_id, location_kind_id)
        REFERENCES location_kinds(library_id, claim_owner_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (policy_version) REFERENCES normalization_policies(policy_version) ON DELETE RESTRICT
) STRICT;
CREATE INDEX kinds_browse ON location_kinds(library_id, canonical_key, location_kind_id) WHERE state = 'active';
CREATE INDEX kinds_merge_target ON location_kinds(library_id, merged_into_id) WHERE merged_into_id IS NOT NULL;

CREATE TABLE locations (
    location_id BLOB PRIMARY KEY CHECK (length(location_id) = 16),
    library_id BLOB NOT NULL,
    location_kind_id BLOB NOT NULL,
    parent_location_id BLOB CHECK (length(parent_location_id) = 16),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    sort_key TEXT NOT NULL CHECK (length(sort_key) > 0),
    description TEXT NOT NULL DEFAULT '',
    aliases_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(aliases_json) AND json_type(aliases_json) = 'array'),
    address_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(address_json) AND json_type(address_json) = 'object'),
    latitude REAL CHECK (latitude BETWEEN -90.0 AND 90.0),
    longitude REAL CHECK (longitude BETWEEN -180.0 AND 180.0),
    thumbnail_sha256 BLOB,
    record_schema INTEGER NOT NULL CHECK (record_schema = 1),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'tombstoned', 'merged')),
    retired_at_ms INTEGER,
    merged_into_id BLOB CHECK (length(merged_into_id) = 16),
    extensions_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(extensions_json) AND json_type(extensions_json) = 'object'),
    UNIQUE (library_id, location_id),
    CHECK ((latitude IS NULL) = (longitude IS NULL)),
    CHECK (parent_location_id IS NULL OR parent_location_id <> location_id),
    CHECK (merged_into_id IS NULL OR merged_into_id <> location_id),
    CHECK ((state = 'active' AND retired_at_ms IS NULL AND merged_into_id IS NULL)
        OR (state = 'tombstoned' AND retired_at_ms IS NOT NULL AND merged_into_id IS NULL)
        OR (state = 'merged' AND retired_at_ms IS NOT NULL AND merged_into_id IS NOT NULL)),
    FOREIGN KEY (library_id) REFERENCES libraries(library_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, location_kind_id) REFERENCES location_kinds(library_id, location_kind_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, parent_location_id) REFERENCES locations(library_id, location_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, merged_into_id) REFERENCES locations(library_id, location_id) ON DELETE RESTRICT,
    FOREIGN KEY (library_id, thumbnail_sha256) REFERENCES library_media(library_id, sha256) ON DELETE RESTRICT
) STRICT;
CREATE INDEX locations_browse ON locations(library_id, sort_key, location_id) WHERE state = 'active';
CREATE INDEX locations_by_kind ON locations(library_id, location_kind_id, location_id) WHERE state = 'active';
CREATE INDEX locations_children ON locations(library_id, parent_location_id) WHERE state = 'active';
CREATE INDEX locations_merge_target ON locations(library_id, merged_into_id) WHERE merged_into_id IS NOT NULL;
