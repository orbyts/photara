ALTER TABLE cloud_transfer_batches
    ADD COLUMN cleanup_started_at timestamptz,
    ADD COLUMN cleaned_at timestamptz,
    ADD CONSTRAINT cloud_transfer_batches_cleanup_order
        CHECK (
            cleaned_at IS NULL
            OR (cleanup_started_at IS NOT NULL AND cleaned_at >= cleanup_started_at)
        );
