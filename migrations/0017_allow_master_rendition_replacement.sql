ALTER TABLE flattened_master_documents
    DROP CONSTRAINT flattened_master_documents_source_rendition_key;

CREATE INDEX flattened_master_documents_source_rendition
    ON flattened_master_documents (project_id, source_file_id, rendition_role);
