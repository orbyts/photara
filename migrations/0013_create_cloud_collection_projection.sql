CREATE TABLE cloud_collection_sync_runs (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES cloud_accounts(id) ON DELETE RESTRICT,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    inventory_run_id uuid NOT NULL
        REFERENCES cloud_provider_inventory_runs(id) ON DELETE RESTRICT,
    state text NOT NULL CHECK (state IN ('complete', 'failed')),
    plan jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE cloud_collection_nodes (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES cloud_accounts(id) ON DELETE RESTRICT,
    semantic_path text NOT NULL CHECK (
        semantic_path <> '' AND semantic_path = btrim(semantic_path)
    ),
    display_name text NOT NULL CHECK (
        display_name <> '' AND display_name = btrim(display_name)
    ),
    node_kind text NOT NULL CHECK (node_kind IN ('set', 'album')),
    parent_id uuid REFERENCES cloud_collection_nodes(id) ON DELETE RESTRICT,
    remote_id text NOT NULL CHECK (remote_id ~ '^[0-9a-f]{32}$'),
    last_sync_run_id uuid NOT NULL
        REFERENCES cloud_collection_sync_runs(id) ON DELETE RESTRICT,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, semantic_path),
    UNIQUE (account_id, remote_id)
);

CREATE TABLE cloud_collection_memberships (
    collection_id uuid NOT NULL REFERENCES cloud_collection_nodes(id) ON DELETE RESTRICT,
    asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
    remote_asset_id text NOT NULL CHECK (remote_asset_id ~ '^[0-9a-f]{32}$'),
    last_sync_run_id uuid NOT NULL
        REFERENCES cloud_collection_sync_runs(id) ON DELETE RESTRICT,
    last_verified_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (collection_id, asset_id)
);
