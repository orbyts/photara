-- Preserve every original intent byte/state; terminal disposition is additive.
CREATE TABLE onboarding_dispositions (
  operation_id BLOB PRIMARY KEY REFERENCES onboarding_intents(operation_id) ON DELETE RESTRICT,
  disposition TEXT NOT NULL CHECK(disposition='abandoned-expired'),
  absence_canonical BLOB NOT NULL CHECK(length(absence_canonical) BETWEEN 1 AND 65536),
  absence_sha256 BLOB NOT NULL CHECK(length(absence_sha256)=32),
  session_generation INTEGER NOT NULL CHECK(session_generation>0),
  observed_at INTEGER NOT NULL
) STRICT;
CREATE TABLE onboarding_replacements (
  operation_id BLOB PRIMARY KEY REFERENCES onboarding_intents(operation_id) ON DELETE RESTRICT,
  previous_operation_id BLOB NOT NULL REFERENCES onboarding_dispositions(operation_id) ON DELETE RESTRICT,
  CHECK(operation_id<>previous_operation_id)
) STRICT;
CREATE TRIGGER onboarding_disposition_admission BEFORE INSERT ON onboarding_dispositions
WHEN NOT EXISTS(SELECT 1 FROM onboarding_intents i JOIN onboarding_session s ON s.singleton=1
 WHERE i.operation_id=NEW.operation_id AND i.state='unknown' AND i.issuer=s.issuer
 AND i.subject=s.subject AND s.generation=NEW.session_generation)
 OR EXISTS(SELECT 1 FROM onboarding_receipts WHERE operation_id=NEW.operation_id)
BEGIN SELECT RAISE(ABORT,'onboarding_disposition_refused'); END;
CREATE TRIGGER onboarding_disposition_no_update BEFORE UPDATE ON onboarding_dispositions
BEGIN SELECT RAISE(ABORT,'onboarding_immutable'); END;
CREATE TRIGGER onboarding_disposition_no_delete BEFORE DELETE ON onboarding_dispositions
BEGIN SELECT RAISE(ABORT,'onboarding_retained'); END;
CREATE TRIGGER onboarding_replacement_admission BEFORE INSERT ON onboarding_replacements
WHEN NOT EXISTS(SELECT 1 FROM onboarding_intents n JOIN onboarding_intents p
 ON n.library_id=p.library_id AND n.environment_id=p.environment_id AND n.device_id=p.device_id
 AND n.credential_reference=p.credential_reference AND n.device_commitment=p.device_commitment
 AND n.expected_issuer=p.expected_issuer WHERE n.operation_id=NEW.operation_id
 AND p.operation_id=NEW.previous_operation_id AND n.state='prepared' AND p.issuer IS NOT NULL)
BEGIN SELECT RAISE(ABORT,'onboarding_replacement_refused'); END;
CREATE TRIGGER onboarding_replacement_no_update BEFORE UPDATE ON onboarding_replacements
BEGIN SELECT RAISE(ABORT,'onboarding_immutable'); END;
CREATE TRIGGER onboarding_replacement_no_delete BEFORE DELETE ON onboarding_replacements
BEGIN SELECT RAISE(ABORT,'onboarding_retained'); END;

-- SQLite partial indexes cannot consult the additive disposition table. These
-- guards run under the same serialized write transaction and retain uniqueness.
DROP INDEX onboarding_unresolved_library;
CREATE INDEX onboarding_unresolved_library ON onboarding_intents(library_id,environment_id);
CREATE TRIGGER onboarding_one_unresolved_insert BEFORE INSERT ON onboarding_intents
WHEN NEW.state NOT IN ('applied','cancelled') AND EXISTS(
 SELECT 1 FROM onboarding_intents i WHERE i.library_id=NEW.library_id AND i.environment_id=NEW.environment_id
 AND i.state NOT IN ('applied','cancelled') AND NOT EXISTS(SELECT 1 FROM onboarding_dispositions d WHERE d.operation_id=i.operation_id))
BEGIN SELECT RAISE(ABORT,'onboarding_unresolved'); END;
CREATE TRIGGER onboarding_one_unresolved_update BEFORE UPDATE OF state ON onboarding_intents
WHEN NEW.state NOT IN ('applied','cancelled')
 AND NOT EXISTS(SELECT 1 FROM onboarding_dispositions WHERE operation_id=NEW.operation_id)
 AND EXISTS(SELECT 1 FROM onboarding_intents i WHERE i.operation_id<>NEW.operation_id
 AND i.library_id=NEW.library_id AND i.environment_id=NEW.environment_id
 AND i.state NOT IN ('applied','cancelled') AND NOT EXISTS(SELECT 1 FROM onboarding_dispositions d WHERE d.operation_id=i.operation_id))
BEGIN SELECT RAISE(ABORT,'onboarding_unresolved'); END;
UPDATE schema_metadata SET minimum_reader=4,minimum_writer=4 WHERE singleton=1;
CREATE TRIGGER onboarding_disposed_state_immutable BEFORE UPDATE OF state ON onboarding_intents
WHEN EXISTS(SELECT 1 FROM onboarding_dispositions WHERE operation_id=OLD.operation_id) AND NEW.state<>OLD.state
BEGIN SELECT RAISE(ABORT,'onboarding_immutable'); END;
