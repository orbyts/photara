CREATE TABLE post_publications (
    id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    post_name text NOT NULL CHECK (
        post_name ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'
    ),
    platform text NOT NULL CHECK (platform IN ('instagram', 'threads')),
    provider text NOT NULL CHECK (provider IN ('instagram', 'threads')),
    account_label text NOT NULL CHECK (
        account_label <> '' AND account_label = btrim(account_label)
    ),
    publication_method text NOT NULL CHECK (
        publication_method IN ('manual-confirmation', 'provider-api')
    ),
    source_specification_sha256 text NOT NULL CHECK (
        source_specification_sha256 ~ '^[0-9a-f]{64}$'
    ),
    external_id text CHECK (external_id IS NULL OR btrim(external_id) <> ''),
    external_url text CHECK (external_url IS NULL OR btrim(external_url) <> ''),
    published_at timestamptz,
    confirmed_at timestamptz NOT NULL DEFAULT now(),
    evidence_note text NOT NULL CHECK (btrim(evidence_note) <> ''),
    UNIQUE (
        project_id,
        post_name,
        platform,
        provider,
        account_label,
        source_specification_sha256,
        publication_method
    )
);

CREATE INDEX post_publications_project_platform
    ON post_publications (project_id, platform, confirmed_at);
