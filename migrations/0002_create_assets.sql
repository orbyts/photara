CREATE TABLE assets (
    id uuid PRIMARY KEY,
    original_filename text NOT NULL,
    original_stem text NOT NULL,
    capture_date date NOT NULL,
    author_code text NOT NULL,
    original_sha256 text NOT NULL UNIQUE
        CHECK (original_sha256 ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE project_assets (
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
    added_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, asset_id)
);

CREATE TABLE asset_files (
    id uuid PRIMARY KEY,
    asset_id uuid NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    representation text NOT NULL CHECK (representation IN (
        'camera-raw',
        'xmp-sidecar',
        'working-dng',
        'layered-psb',
        'flattened-tiff',
        'delivery-rendition',
        'pixieset-proof'
    )),
    location text NOT NULL,
    sha256 text CHECK (sha256 IS NULL OR sha256 ~ '^[0-9a-f]{64}$'),
    byte_size bigint CHECK (byte_size IS NULL OR byte_size >= 0),
    authoritative boolean NOT NULL,
    state text NOT NULL DEFAULT 'current'
        CHECK (state IN ('current', 'removed')),
    created_at timestamptz NOT NULL DEFAULT now(),
    removed_at timestamptz,
    CHECK (
        (state = 'current' AND removed_at IS NULL)
        OR (state = 'removed' AND removed_at IS NOT NULL)
    )
);

CREATE UNIQUE INDEX asset_files_one_current_authority
    ON asset_files (asset_id, representation)
    WHERE authoritative AND state = 'current';

CREATE UNIQUE INDEX asset_files_current_location
    ON asset_files (location)
    WHERE state = 'current';

CREATE TABLE asset_file_origins (
    source_file_id uuid NOT NULL REFERENCES asset_files(id) ON DELETE RESTRICT,
    derived_file_id uuid NOT NULL REFERENCES asset_files(id) ON DELETE CASCADE,
    operation text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (source_file_id, derived_file_id),
    CHECK (source_file_id <> derived_file_id)
);
