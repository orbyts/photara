CREATE TABLE cloudinary_delivery_batches (
    id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    post_name text NOT NULL CHECK (post_name ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'),
    platform text NOT NULL CHECK (platform IN ('instagram', 'threads')),
    account_label text NOT NULL CHECK (account_label <> '' AND account_label = btrim(account_label)),
    cloud_name text NOT NULL CHECK (cloud_name <> '' AND cloud_name = btrim(cloud_name)),
    folder_mode text NOT NULL CHECK (folder_mode IN ('dynamic', 'fixed')),
    source_specification_sha256 text NOT NULL CHECK (source_specification_sha256 ~ '^[0-9a-f]{64}$'),
    manifest_sha256 text NOT NULL CHECK (manifest_sha256 ~ '^[0-9a-f]{64}$'),
    asset_folder text NOT NULL CHECK (asset_folder <> '' AND asset_folder = btrim(asset_folder)),
    item_count integer NOT NULL CHECK (item_count > 0),
    asset_count integer NOT NULL CHECK (asset_count > 0),
    state text NOT NULL DEFAULT 'prepared' CHECK (
        state IN ('prepared', 'canary-uploaded', 'uploaded', 'verified')
    ),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    verified_at timestamptz,
    UNIQUE (
        project_id, post_name, platform, account_label,
        source_specification_sha256, manifest_sha256
    )
);

CREATE TABLE cloudinary_delivery_assets (
    batch_id uuid NOT NULL REFERENCES cloudinary_delivery_batches(id) ON DELETE RESTRICT,
    asset_index integer NOT NULL CHECK (asset_index > 0),
    item_id text NOT NULL CHECK (item_id ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'),
    frame_index integer NOT NULL CHECK (frame_index > 0),
    local_relative_path text NOT NULL CHECK (local_relative_path <> ''),
    source_sha256 text NOT NULL CHECK (source_sha256 ~ '^[0-9a-f]{64}$'),
    source_byte_size bigint NOT NULL CHECK (source_byte_size > 0),
    width integer NOT NULL CHECK (width > 0),
    height integer NOT NULL CHECK (height > 0),
    color_profile text NOT NULL CHECK (color_profile <> ''),
    asset_folder text NOT NULL CHECK (asset_folder <> ''),
    public_id text NOT NULL CHECK (public_id <> ''),
    state text NOT NULL DEFAULT 'prepared' CHECK (state IN ('prepared', 'uploaded', 'verified')),
    cloudinary_asset_id text,
    cloudinary_version bigint,
    secure_url text,
    provider_byte_size bigint CHECK (provider_byte_size IS NULL OR provider_byte_size > 0),
    provider_format text,
    provider_etag text,
    uploaded_at timestamptz,
    verified_at timestamptz,
    PRIMARY KEY (batch_id, asset_index),
    UNIQUE (batch_id, public_id),
    CHECK (
        (state = 'prepared' AND cloudinary_asset_id IS NULL AND uploaded_at IS NULL)
        OR
        (state IN ('uploaded', 'verified') AND cloudinary_asset_id IS NOT NULL AND uploaded_at IS NOT NULL)
    )
);

CREATE INDEX cloudinary_delivery_batches_project_platform
    ON cloudinary_delivery_batches (project_id, platform, created_at);

CREATE INDEX cloudinary_delivery_assets_provider_id
    ON cloudinary_delivery_assets (cloudinary_asset_id)
    WHERE cloudinary_asset_id IS NOT NULL;
