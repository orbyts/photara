ALTER TABLE cloud_transfer_items
    ADD COLUMN remote_asset_id text,
    ADD COLUMN uploaded_at timestamptz,
    ADD COLUMN verified_at timestamptz,
    ADD CONSTRAINT cloud_transfer_items_remote_asset_id_format
        CHECK (remote_asset_id IS NULL OR remote_asset_id ~ '^[0-9a-f]{32}$');

CREATE UNIQUE INDEX cloud_transfer_items_remote_asset
    ON cloud_transfer_items (remote_asset_id)
    WHERE remote_asset_id IS NOT NULL;
