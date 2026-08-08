CREATE TABLE cloud_accounts (
    id uuid PRIMARY KEY,
    provider text NOT NULL,
    label text NOT NULL,
    remote_catalog_id text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (provider, label)
);

CREATE UNIQUE INDEX cloud_accounts_remote_catalog
    ON cloud_accounts (provider, remote_catalog_id)
    WHERE remote_catalog_id IS NOT NULL;

CREATE TABLE cloud_evidence_imports (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES cloud_accounts(id) ON DELETE RESTRICT,
    source_system text NOT NULL,
    evidence_kind text NOT NULL CHECK (evidence_kind IN (
        'user-confirmed',
        'provider-api'
    )),
    source_name text NOT NULL,
    source_sha256 text NOT NULL CHECK (source_sha256 ~ '^[0-9a-f]{64}$'),
    row_count integer NOT NULL CHECK (row_count >= 0),
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    imported_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, source_system, source_sha256)
);

CREATE TABLE cloud_evidence_entries (
    import_id uuid NOT NULL REFERENCES cloud_evidence_imports(id) ON DELETE CASCADE,
    source_key text NOT NULL,
    original_path text NOT NULL,
    original_relative_path text NOT NULL,
    original_filename text NOT NULL,
    dng_path text,
    dng_filename text,
    source_payload jsonb NOT NULL,
    matched_asset_id uuid REFERENCES assets(id) ON DELETE SET NULL,
    remote_asset_id text,
    PRIMARY KEY (import_id, source_key)
);

CREATE TABLE asset_cloud_presence (
    account_id uuid NOT NULL REFERENCES cloud_accounts(id) ON DELETE RESTRICT,
    asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    status text NOT NULL CHECK (status IN ('present', 'removed')),
    evidence_kind text NOT NULL CHECK (evidence_kind IN (
        'user-confirmed',
        'provider-api'
    )),
    evidence_import_id uuid REFERENCES cloud_evidence_imports(id) ON DELETE SET NULL,
    remote_asset_id text,
    first_confirmed_at timestamptz NOT NULL,
    last_verified_at timestamptz NOT NULL,
    PRIMARY KEY (account_id, asset_id)
);

CREATE UNIQUE INDEX asset_cloud_presence_remote_asset
    ON asset_cloud_presence (account_id, remote_asset_id)
    WHERE remote_asset_id IS NOT NULL;

CREATE TABLE project_asset_decisions (
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    asset_id uuid NOT NULL,
    decision text NOT NULL CHECK (decision IN ('photographer-final')),
    selected boolean NOT NULL,
    decided_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, asset_id, decision),
    FOREIGN KEY (project_id, asset_id)
        REFERENCES project_assets(project_id, asset_id) ON DELETE CASCADE
);

CREATE TABLE cloud_transfer_batches (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES cloud_accounts(id) ON DELETE RESTRICT,
    project_id uuid REFERENCES projects(id) ON DELETE SET NULL,
    mode text NOT NULL CHECK (mode IN ('manual', 'api')),
    state text NOT NULL CHECK (state IN (
        'planned',
        'exporting',
        'awaiting-user-confirmation',
        'uploading',
        'verifying',
        'complete',
        'failed',
        'cancelled'
    )),
    manifest_sha256 text CHECK (
        manifest_sha256 IS NULL OR manifest_sha256 ~ '^[0-9a-f]{64}$'
    ),
    expected_count integer NOT NULL DEFAULT 0 CHECK (expected_count >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    confirmed_at timestamptz
);

CREATE TABLE cloud_transfer_items (
    batch_id uuid NOT NULL REFERENCES cloud_transfer_batches(id) ON DELETE CASCADE,
    asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
    working_file_id uuid REFERENCES asset_files(id) ON DELETE SET NULL,
    state text NOT NULL CHECK (state IN (
        'planned',
        'exported',
        'uploaded',
        'verified',
        'skipped-already-present',
        'failed'
    )),
    error_message text,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (batch_id, asset_id)
);
