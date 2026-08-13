ALTER TABLE asset_files
    DROP CONSTRAINT asset_files_representation_check;

UPDATE asset_files
SET representation = 'flattened-hdr-tiff'
WHERE representation = 'flattened-tiff';

ALTER TABLE asset_files
    ADD CONSTRAINT asset_files_representation_check CHECK (representation IN (
        'camera-raw',
        'xmp-sidecar',
        'working-dng',
        'layered-psb',
        'flattened-hdr-tiff',
        'flattened-sdr-tiff',
        'delivery-rendition',
        'pixieset-proof'
    ));

ALTER TABLE flattened_master_documents
    ADD COLUMN rendition_role text NOT NULL DEFAULT 'hdr'
        CHECK (rendition_role IN ('hdr', 'sdr'));

ALTER TABLE flattened_master_documents
    DROP CONSTRAINT flattened_master_documents_project_id_source_file_id_key;

ALTER TABLE flattened_master_documents
    ADD CONSTRAINT flattened_master_documents_source_rendition_key
        UNIQUE (project_id, source_file_id, rendition_role);
