DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM asset_files
        WHERE representation = 'camera-raw' AND location !~* '/images/'
    ) THEN
        RAISE EXCEPTION 'cannot normalize camera RAW location without an Images component';
    END IF;
END $$;

UPDATE asset_files
SET location = 'images:' || regexp_replace(location, '^.*/images/', '', 'i')
WHERE representation = 'camera-raw' AND location ~* '/images/';

ALTER TABLE asset_files
    ADD CONSTRAINT asset_files_location_nonempty
        CHECK (location <> '' AND location = btrim(location)),
    ADD CONSTRAINT asset_files_camera_raw_portable
        CHECK (representation <> 'camera-raw' OR location LIKE 'images:%');
