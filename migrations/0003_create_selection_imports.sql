CREATE TABLE selection_imports (
    id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    provider text NOT NULL CHECK (provider IN ('pixieset')),
    selection_kind text NOT NULL CHECK (selection_kind IN (
        'client-favorite',
        'client-shortlist',
        'hero'
    )),
    source_name text NOT NULL,
    source_sha256 text NOT NULL CHECK (source_sha256 ~ '^[0-9a-f]{64}$'),
    collection_name text NOT NULL,
    favorite_name text NOT NULL,
    client_email text,
    source_contents text NOT NULL,
    imported_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, provider, selection_kind, source_sha256)
);

CREATE TABLE selection_import_entries (
    import_id uuid NOT NULL REFERENCES selection_imports(id) ON DELETE CASCADE,
    proof_filename text NOT NULL,
    original_filename text NOT NULL,
    note text,
    photo_set text,
    provider_created_at text,
    PRIMARY KEY (import_id, proof_filename)
);

CREATE TABLE project_selection_memberships (
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    original_filename text NOT NULL,
    selection_kind text NOT NULL CHECK (selection_kind IN (
        'client-favorite',
        'client-shortlist',
        'hero'
    )),
    import_id uuid NOT NULL REFERENCES selection_imports(id) ON DELETE RESTRICT,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, original_filename, selection_kind)
);
