-- CXT4d: durable local onboarding; no content transport is activated.
CREATE TABLE onboarding_session (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1),
  generation INTEGER NOT NULL CHECK(generation>=0),
  issuer TEXT,
  subject TEXT,
  CHECK((issuer IS NULL)=(subject IS NULL))
) STRICT;
INSERT INTO onboarding_session VALUES(1,0,NULL,NULL);
CREATE TRIGGER onboarding_session_monotonic BEFORE UPDATE ON onboarding_session
WHEN NEW.singleton<>OLD.singleton OR NEW.generation<>OLD.generation+1
BEGIN SELECT RAISE(ABORT,'onboarding_generation'); END;
CREATE TRIGGER onboarding_session_no_delete BEFORE DELETE ON onboarding_session
BEGIN SELECT RAISE(ABORT,'onboarding_retained'); END;

CREATE TABLE onboarding_intents (
  operation_id BLOB PRIMARY KEY CHECK(length(operation_id)=16 AND operation_id<>zeroblob(16)),
  database_id BLOB NOT NULL CHECK(length(database_id)=16 AND database_id<>zeroblob(16)),
  device_id BLOB NOT NULL REFERENCES local_device(device_id) ON DELETE RESTRICT,
  library_id BLOB NOT NULL REFERENCES libraries(library_id) ON DELETE RESTRICT,
  environment_id TEXT NOT NULL CHECK(length(environment_id) BETWEEN 1 AND 255),
  expected_issuer TEXT NOT NULL,
  issuer TEXT,
  subject TEXT,
  local_principal_id BLOB NOT NULL CHECK(length(local_principal_id)=16),
  library_revision INTEGER NOT NULL CHECK(library_revision>=1),
  contract_revision INTEGER NOT NULL CHECK(contract_revision>=1),
  command_canonical BLOB NOT NULL CHECK(length(command_canonical) BETWEEN 1 AND 65536),
  command_sha256 BLOB NOT NULL CHECK(length(command_sha256)=32),
  credential_reference TEXT NOT NULL CHECK(length(credential_reference) BETWEEN 1 AND 512),
  device_commitment TEXT NOT NULL CHECK(length(device_commitment)=64),
  state TEXT NOT NULL CHECK(state IN ('prepared','cancelled','authenticated','unknown','receipt-ready','reconciliation-required','library-choice-required','applied')),
  created_at INTEGER NOT NULL,
  CHECK((issuer IS NULL)=(subject IS NULL)),
  CHECK(issuer IS NULL OR issuer=expected_issuer),
  CHECK((state IN ('prepared','cancelled'))=(issuer IS NULL))
) STRICT;
CREATE UNIQUE INDEX onboarding_unresolved_library ON onboarding_intents(library_id,environment_id) WHERE state NOT IN ('applied','cancelled');
CREATE TABLE onboarding_credential_references (
  environment_id TEXT NOT NULL,
  issuer TEXT NOT NULL,
  subject TEXT NOT NULL,
  device_id BLOB NOT NULL REFERENCES local_device(device_id) ON DELETE RESTRICT,
  credential_reference TEXT NOT NULL,
  device_commitment TEXT NOT NULL CHECK(length(device_commitment)=64),
  PRIMARY KEY(environment_id,issuer,subject,device_id),
  UNIQUE(credential_reference)
) STRICT;
CREATE TRIGGER onboarding_credential_no_update BEFORE UPDATE ON onboarding_credential_references
BEGIN SELECT RAISE(ABORT,'onboarding_immutable'); END;
CREATE TRIGGER onboarding_credential_no_delete BEFORE DELETE ON onboarding_credential_references
BEGIN SELECT RAISE(ABORT,'onboarding_retained'); END;

CREATE TABLE onboarding_receipts (
  operation_id BLOB PRIMARY KEY REFERENCES onboarding_intents(operation_id) ON DELETE RESTRICT,
  receipt_canonical BLOB NOT NULL CHECK(length(receipt_canonical) BETWEEN 1 AND 65536),
  receipt_sha256 BLOB NOT NULL CHECK(length(receipt_sha256)=32),
  received_at INTEGER NOT NULL
) STRICT;

-- Current session DTO is the exact account/membership observation. The service
-- does not supply account revision/display fields required by account_cache;
-- never invent those fields or activate its content sync target from this DTO.
CREATE TABLE library_cloud_bindings (
  library_id BLOB PRIMARY KEY REFERENCES libraries(library_id) ON DELETE RESTRICT,
  operation_id BLOB NOT NULL REFERENCES onboarding_receipts(operation_id) ON DELETE RESTRICT,
  environment_id TEXT NOT NULL,
  issuer TEXT NOT NULL,
  subject TEXT NOT NULL,
  account_id BLOB NOT NULL CHECK(length(account_id)=16 AND account_id<>zeroblob(16)),
  identity_id BLOB NOT NULL CHECK(length(identity_id)=16 AND identity_id<>zeroblob(16)),
  access_canonical BLOB NOT NULL CHECK(length(access_canonical) BETWEEN 1 AND 65536),
  access_sha256 BLOB NOT NULL CHECK(length(access_sha256)=32),
  observed_at INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK(local_revision>=1),
  state TEXT NOT NULL CHECK(state IN ('cached','signed-out','access-disabled')),
  created_at INTEGER NOT NULL
) STRICT;
CREATE INDEX onboarding_binding_principal ON library_cloud_bindings(environment_id,issuer,subject);

CREATE TABLE onboarding_library_selection (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1),
  library_id BLOB NOT NULL REFERENCES libraries(library_id) ON DELETE RESTRICT,
  operation_id BLOB NOT NULL REFERENCES onboarding_receipts(operation_id) ON DELETE RESTRICT,
  chosen_at INTEGER NOT NULL
) STRICT;

CREATE TRIGGER onboarding_intent_no_delete BEFORE DELETE ON onboarding_intents
BEGIN SELECT RAISE(ABORT,'onboarding_retained'); END;
CREATE TRIGGER onboarding_intent_immutable BEFORE UPDATE ON onboarding_intents
WHEN NEW.operation_id IS NOT OLD.operation_id OR NEW.database_id IS NOT OLD.database_id
 OR NEW.device_id IS NOT OLD.device_id OR NEW.library_id IS NOT OLD.library_id
 OR NEW.environment_id IS NOT OLD.environment_id OR NEW.expected_issuer IS NOT OLD.expected_issuer
 OR NEW.local_principal_id IS NOT OLD.local_principal_id OR NEW.library_revision IS NOT OLD.library_revision
 OR NEW.contract_revision IS NOT OLD.contract_revision OR NEW.command_canonical IS NOT OLD.command_canonical
 OR NEW.command_sha256 IS NOT OLD.command_sha256 OR NEW.credential_reference IS NOT OLD.credential_reference
 OR NEW.device_commitment IS NOT OLD.device_commitment OR NEW.created_at IS NOT OLD.created_at
 OR (OLD.issuer IS NOT NULL AND (NEW.issuer IS NOT OLD.issuer OR NEW.subject IS NOT OLD.subject))
 OR (OLD.state IN ('applied','cancelled') AND NEW.state<>OLD.state)
BEGIN SELECT RAISE(ABORT,'onboarding_immutable'); END;
CREATE TRIGGER onboarding_receipt_no_update BEFORE UPDATE ON onboarding_receipts
BEGIN SELECT RAISE(ABORT,'onboarding_immutable'); END;
CREATE TRIGGER onboarding_receipt_no_delete BEFORE DELETE ON onboarding_receipts
BEGIN SELECT RAISE(ABORT,'onboarding_retained'); END;
CREATE TRIGGER onboarding_binding_no_delete BEFORE DELETE ON library_cloud_bindings
BEGIN SELECT RAISE(ABORT,'onboarding_retained'); END;
CREATE TRIGGER onboarding_binding_immutable BEFORE UPDATE ON library_cloud_bindings
WHEN NEW.library_id IS NOT OLD.library_id OR NEW.operation_id IS NOT OLD.operation_id
 OR NEW.environment_id IS NOT OLD.environment_id OR NEW.issuer IS NOT OLD.issuer OR NEW.subject IS NOT OLD.subject
 OR NEW.account_id IS NOT OLD.account_id OR NEW.identity_id IS NOT OLD.identity_id
 OR NEW.created_at IS NOT OLD.created_at OR NEW.local_revision<>OLD.local_revision+1
BEGIN SELECT RAISE(ABORT,'onboarding_immutable'); END;

UPDATE schema_metadata SET minimum_reader=3,minimum_writer=3 WHERE singleton=1;
