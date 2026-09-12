PRAGMA application_id = 0x50485432;
PRAGMA user_version = 2;

CREATE TABLE schema_metadata (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    schema_family TEXT NOT NULL CHECK (schema_family = 'photara.local.g2'),
    database_id BLOB NOT NULL CHECK (length(database_id) = 16),
    schema_epoch INTEGER NOT NULL CHECK (schema_epoch = 1),
    minimum_reader INTEGER NOT NULL CHECK (minimum_reader >= 1),
    minimum_writer INTEGER NOT NULL CHECK (minimum_writer >= 1),
    canonical_codec TEXT NOT NULL CHECK (canonical_codec = 'photara.canonical-json.v1'),
    created_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE normalization_policies (
    policy_version INTEGER PRIMARY KEY CHECK (policy_version >= 1),
    unicode_version TEXT NOT NULL,
    rules_sha256 BLOB NOT NULL CHECK (length(rules_sha256) = 32),
    description TEXT NOT NULL
) STRICT;

CREATE TABLE local_device (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    device_id BLOB NOT NULL UNIQUE CHECK (length(device_id) = 16),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    created_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE workspaces (
    workspace_id BLOB PRIMARY KEY CHECK (length(workspace_id) = 16),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    state TEXT NOT NULL CHECK (state IN ('active', 'tombstoned')),
    local_revision INTEGER NOT NULL CHECK (local_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    retired_at_ms INTEGER,
    term_policy_version INTEGER NOT NULL,
    extensions_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(extensions_json) AND json_type(extensions_json) = 'object'),
    CHECK ((state = 'active' AND retired_at_ms IS NULL) OR (state = 'tombstoned' AND retired_at_ms IS NOT NULL)),
    FOREIGN KEY (term_policy_version) REFERENCES normalization_policies(policy_version) ON DELETE RESTRICT
) STRICT;

CREATE TABLE account_cache (
    account_id BLOB PRIMARY KEY CHECK (length(account_id) = 16),
    display_name TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'disabled', 'deleted')),
    server_revision TEXT NOT NULL CHECK (length(server_revision) > 0),
    observed_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE membership_cache (
    workspace_id BLOB NOT NULL,
    account_id BLOB NOT NULL,
    membership_id BLOB NOT NULL CHECK (length(membership_id) = 16),
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'editor', 'viewer')),
    state TEXT NOT NULL CHECK (state IN ('active', 'revoked')),
    server_revision TEXT NOT NULL CHECK (length(server_revision) > 0),
    observed_at_ms INTEGER NOT NULL,
    PRIMARY KEY (workspace_id, account_id),
    UNIQUE (membership_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE RESTRICT,
    FOREIGN KEY (account_id) REFERENCES account_cache(account_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE library_media (
    workspace_id BLOB NOT NULL,
    sha256 BLOB NOT NULL CHECK (length(sha256) = 32),
    media_type TEXT NOT NULL CHECK (length(media_type) > 0),
    byte_length INTEGER NOT NULL CHECK (byte_length >= 0),
    width INTEGER CHECK (width > 0),
    height INTEGER CHECK (height > 0),
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (workspace_id, sha256),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE media_local_state (
    workspace_id BLOB NOT NULL,
    sha256 BLOB NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('pending', 'available', 'missing', 'corrupt')),
    verified_at_ms INTEGER,
    PRIMARY KEY (workspace_id, sha256),
    FOREIGN KEY (workspace_id, sha256) REFERENCES library_media(workspace_id, sha256) ON DELETE RESTRICT
) STRICT;
