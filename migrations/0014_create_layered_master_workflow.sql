CREATE TABLE layered_master_documents (
    asset_file_id uuid PRIMARY KEY REFERENCES asset_files(id) ON DELETE CASCADE,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    source_file_id uuid NOT NULL REFERENCES asset_files(id) ON DELETE RESTRICT,
    build_batch_id uuid NOT NULL,
    bits_per_channel smallint NOT NULL CHECK (bits_per_channel IN (16, 32)),
    color_profile text NOT NULL CHECK (btrim(color_profile) <> ''),
    smart_object_source text NOT NULL CHECK (btrim(smart_object_source) <> ''),
    smart_object_embedded boolean NOT NULL CHECK (smart_object_embedded),
    workflow_state text NOT NULL CHECK (workflow_state IN (
        'editing',
        'ready-for-flattening',
        'flattened'
    )),
    verified_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE layered_master_events (
    id uuid PRIMARY KEY,
    asset_file_id uuid NOT NULL REFERENCES layered_master_documents(asset_file_id)
        ON DELETE CASCADE,
    event_type text NOT NULL CHECK (event_type IN (
        'promoted',
        'checkpointed',
        'marked-ready',
        'flattened'
    )),
    sha256 text NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    byte_size bigint NOT NULL CHECK (byte_size > 0),
    note text CHECK (note IS NULL OR btrim(note) <> ''),
    occurred_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX layered_master_documents_project_state
    ON layered_master_documents (project_id, workflow_state);

CREATE INDEX layered_master_events_file_time
    ON layered_master_events (asset_file_id, occurred_at);
