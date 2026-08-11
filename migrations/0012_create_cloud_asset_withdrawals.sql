CREATE TABLE cloud_asset_withdrawals (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES cloud_accounts(id) ON DELETE RESTRICT,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
    remote_asset_id text NOT NULL,
    remote_filename text NOT NULL CHECK (
        remote_filename <> '' AND remote_filename = btrim(remote_filename)
    ),
    state text NOT NULL CHECK (state IN (
        'awaiting-user-deletion',
        'verified-removed',
        'cancelled'
    )),
    reason text,
    planned_inventory_run_id uuid NOT NULL
        REFERENCES cloud_provider_inventory_runs(id) ON DELETE RESTRICT,
    verified_inventory_run_id uuid
        REFERENCES cloud_provider_inventory_runs(id) ON DELETE RESTRICT,
    requested_at timestamptz NOT NULL DEFAULT now(),
    verified_at timestamptz,
    FOREIGN KEY (project_id, asset_id)
        REFERENCES project_assets(project_id, asset_id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX cloud_asset_withdrawals_active_asset
    ON cloud_asset_withdrawals (account_id, asset_id)
    WHERE state = 'awaiting-user-deletion';

CREATE INDEX cloud_asset_withdrawals_remote_asset
    ON cloud_asset_withdrawals (account_id, remote_asset_id);
