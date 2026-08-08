CREATE TABLE storage_roots (
    key text PRIMARY KEY CHECK (key <> '' AND key = btrim(key)),
    environment_variable text NOT NULL UNIQUE CHECK (
        environment_variable <> '' AND environment_variable = btrim(environment_variable)
    ),
    purpose text NOT NULL CHECK (purpose <> '' AND purpose = btrim(purpose))
);

INSERT INTO storage_roots (key, environment_variable, purpose) VALUES
    ('images', 'PHOTARA_IMAGES_ROOT', 'Camera RAW and XMP archive'),
    ('projects', 'PHOTARA_PROJECTS_ROOT', 'Project deliverables');

ALTER TABLE cloud_evidence_entries
    ADD COLUMN storage_root_key text;

UPDATE cloud_evidence_entries
SET storage_root_key = 'images';

UPDATE cloud_evidence_entries AS evidence
SET dng_filename = (
    SELECT inventory.file_name
    FROM cloud_provider_inventory_assets AS inventory
    JOIN cloud_provider_inventory_runs AS run ON run.id = inventory.run_id
    JOIN cloud_evidence_imports AS evidence_import ON evidence_import.id = evidence.import_id
    WHERE run.account_id = evidence_import.account_id
      AND inventory.remote_asset_id = evidence.remote_asset_id
      AND NULLIF(btrim(inventory.file_name), '') IS NOT NULL
    ORDER BY run.completed_at DESC
    LIMIT 1
)
WHERE NULLIF(btrim(evidence.dng_filename), '') IS NULL
  AND evidence.remote_asset_id IS NOT NULL;

UPDATE cloud_evidence_entries
SET dng_filename = NULLIF(btrim(dng_filename), ''),
    source_key = 'images:' || original_relative_path,
    source_payload = source_payload - 'source_path' - 'dng_path';

ALTER TABLE cloud_evidence_entries
    ALTER COLUMN storage_root_key SET NOT NULL,
    ADD CONSTRAINT cloud_evidence_entries_storage_root
        FOREIGN KEY (storage_root_key) REFERENCES storage_roots(key) ON DELETE RESTRICT,
    ADD CONSTRAINT cloud_evidence_entries_source_key_nonempty
        CHECK (source_key <> '' AND source_key = btrim(source_key)),
    ADD CONSTRAINT cloud_evidence_entries_relative_path_nonempty
        CHECK (
            original_relative_path <> ''
            AND original_relative_path = btrim(original_relative_path)
        ),
    ADD CONSTRAINT cloud_evidence_entries_filename_nonempty
        CHECK (original_filename <> '' AND original_filename = btrim(original_filename)),
    ADD CONSTRAINT cloud_evidence_entries_dng_filename_nonempty
        CHECK (dng_filename IS NULL OR (dng_filename <> '' AND dng_filename = btrim(dng_filename))),
    ADD CONSTRAINT cloud_evidence_entries_canonical_source_key
        CHECK (source_key = storage_root_key || ':' || original_relative_path),
    DROP COLUMN original_path,
    DROP COLUMN dng_path;
