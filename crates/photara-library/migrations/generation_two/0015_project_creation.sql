-- UI1 local intent/receipt; no host path enters a portable or service record.
CREATE TABLE project_creation_intents (
  operation_id BLOB PRIMARY KEY CHECK(length(operation_id)=16),
  library_id BLOB NOT NULL REFERENCES libraries(library_id) ON DELETE RESTRICT,
  project_id BLOB NOT NULL UNIQUE CHECK(length(project_id)=16),
  request_canonical TEXT NOT NULL CHECK(length(request_canonical) BETWEEN 1 AND 65536),
  request_sha256 BLOB NOT NULL CHECK(length(request_sha256)=32),
  destination_key TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('prepared','staged','dispatching','remote-committed','published','complete','cancelling','cancelled')),
  stage_pin TEXT,
  cloud_receipt TEXT,
  created_at_ms INTEGER NOT NULL,
  CHECK(state NOT IN ('staged','dispatching','remote-committed','published','complete') OR stage_pin IS NOT NULL)
) STRICT;
CREATE UNIQUE INDEX project_creation_destination ON project_creation_intents(destination_key) WHERE state<>'cancelled';
CREATE TRIGGER project_creation_immutable BEFORE UPDATE ON project_creation_intents
WHEN NEW.operation_id IS NOT OLD.operation_id OR NEW.library_id IS NOT OLD.library_id
 OR NEW.project_id IS NOT OLD.project_id OR NEW.request_canonical IS NOT OLD.request_canonical
 OR NEW.request_sha256 IS NOT OLD.request_sha256 OR NEW.destination_key IS NOT OLD.destination_key
 OR NEW.created_at_ms IS NOT OLD.created_at_ms
 OR (OLD.stage_pin IS NOT NULL AND NEW.stage_pin IS NOT OLD.stage_pin)
 OR (OLD.cloud_receipt IS NOT NULL AND NEW.cloud_receipt IS NOT OLD.cloud_receipt)
 OR (OLD.state IN ('complete','cancelled') AND NEW.state<>OLD.state)
BEGIN SELECT RAISE(ABORT,'creation_immutable'); END;
CREATE TRIGGER project_creation_retained BEFORE DELETE ON project_creation_intents
BEGIN SELECT RAISE(ABORT,'creation_retained'); END;
UPDATE schema_metadata SET minimum_reader=5,minimum_writer=5 WHERE singleton=1;
