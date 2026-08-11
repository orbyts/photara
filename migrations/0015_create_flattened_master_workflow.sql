CREATE TABLE flattened_master_documents (
    asset_file_id uuid PRIMARY KEY REFERENCES asset_files(id) ON DELETE CASCADE,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    source_file_id uuid NOT NULL REFERENCES layered_master_documents(asset_file_id)
        ON DELETE RESTRICT,
    build_batch_id uuid NOT NULL,
    bits_per_channel smallint NOT NULL CHECK (bits_per_channel = 32),
    color_profile text NOT NULL CHECK (btrim(color_profile) <> ''),
    layer_count integer NOT NULL CHECK (layer_count = 1),
    verified_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, source_file_id)
);

CREATE INDEX flattened_master_documents_project
    ON flattened_master_documents (project_id);
