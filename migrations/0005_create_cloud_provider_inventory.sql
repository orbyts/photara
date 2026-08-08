CREATE TABLE cloud_provider_inventory_runs (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES cloud_accounts(id) ON DELETE RESTRICT,
    remote_catalog_id text NOT NULL,
    snapshot_sha256 text NOT NULL CHECK (snapshot_sha256 ~ '^[0-9a-f]{64}$'),
    asset_count integer NOT NULL CHECK (asset_count >= 0),
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    completed_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, snapshot_sha256)
);

CREATE TABLE cloud_provider_inventory_assets (
    run_id uuid NOT NULL REFERENCES cloud_provider_inventory_runs(id) ON DELETE CASCADE,
    remote_asset_id text NOT NULL,
    subtype text NOT NULL,
    file_name text,
    sha256 text CHECK (sha256 IS NULL OR sha256 ~ '^[0-9A-Fa-f]{64}$'),
    capture_date text,
    source_payload jsonb NOT NULL,
    PRIMARY KEY (run_id, remote_asset_id)
);

CREATE INDEX cloud_provider_inventory_assets_filename
    ON cloud_provider_inventory_assets (run_id, file_name)
    WHERE file_name IS NOT NULL;
