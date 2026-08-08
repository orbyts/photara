ALTER TABLE cloud_transfer_batches
    ADD COLUMN manifest jsonb NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE cloud_transfer_items
    ADD COLUMN planned_filename text,
    ADD CONSTRAINT cloud_transfer_items_planned_filename_nonempty
        CHECK (
            planned_filename IS NULL
            OR (planned_filename <> '' AND planned_filename = btrim(planned_filename))
        );

CREATE UNIQUE INDEX cloud_transfer_batches_manifest
    ON cloud_transfer_batches (account_id, project_id, manifest_sha256)
    WHERE project_id IS NOT NULL AND manifest_sha256 IS NOT NULL;
