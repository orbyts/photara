-- CXT3c executable service migration; promoted from S4.
CREATE TABLE photara_private.library_streams (
  library_id uuid PRIMARY KEY REFERENCES photara.libraries ON DELETE RESTRICT,
  epoch uuid NOT NULL,
  last_sequence bigint NOT NULL DEFAULT 0 CHECK(last_sequence>=0),
  minimum_retained_sequence bigint NOT NULL DEFAULT 0 CHECK(minimum_retained_sequence>=0),
  UNIQUE(library_id,epoch),
  CHECK(minimum_retained_sequence<=last_sequence)
);
CREATE TABLE photara_private.mutation_receipts (
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  mutation_id uuid NOT NULL,
  actor_account_id uuid NOT NULL,
  device_id uuid NOT NULL,
  command_schema integer NOT NULL CHECK(command_schema>=1),
  command_kind text NOT NULL CHECK(length(command_kind)>0),
  request_canonical bytea NOT NULL CHECK(octet_length(request_canonical) BETWEEN 2 AND 1048576),
  request_sha256 bytea NOT NULL CHECK(octet_length(request_sha256)=32),
  outcome text NOT NULL CHECK(outcome IN ('accepted','rejected','conflict')),
  response_canonical bytea NOT NULL CHECK(octet_length(response_canonical) BETWEEN 2 AND 4194304),
  response_sha256 bytea NOT NULL CHECK(octet_length(response_sha256)=32),
  error_code text,
  accepted_epoch uuid,
  accepted_sequence bigint,
  received_at timestamptz NOT NULL,
  completed_at timestamptz NOT NULL,
  PRIMARY KEY(library_id,mutation_id),
  CHECK((outcome='accepted')=(accepted_epoch IS NOT NULL)),
  CHECK((outcome='accepted')=(accepted_sequence IS NOT NULL)),
  CHECK(accepted_sequence IS NULL OR accepted_sequence>0),
  CHECK(outcome<>'accepted' OR error_code IS NULL),
  FOREIGN KEY(actor_account_id,device_id)
    REFERENCES photara_identity.devices(account_id,device_id) ON DELETE RESTRICT
);
CREATE TABLE photara.library_change_batches (
  library_id uuid NOT NULL,
  epoch uuid NOT NULL,
  sequence bigint NOT NULL CHECK(sequence>0),
  mutation_id uuid NOT NULL,
  committed_at timestamptz NOT NULL,
  change_count integer NOT NULL CHECK(change_count BETWEEN 1 AND 1000),
  batch_canonical bytea NOT NULL CHECK(octet_length(batch_canonical) BETWEEN 2 AND 4194304),
  batch_sha256 bytea NOT NULL CHECK(octet_length(batch_sha256)=32),
  PRIMARY KEY(library_id,epoch,sequence),
  UNIQUE(library_id,mutation_id),
  FOREIGN KEY(library_id,epoch)
    REFERENCES photara_private.library_streams(library_id,epoch) ON DELETE RESTRICT,
  FOREIGN KEY(library_id,mutation_id)
    REFERENCES photara_private.mutation_receipts(library_id,mutation_id)
    DEFERRABLE INITIALLY DEFERRED
);
ALTER TABLE photara_private.mutation_receipts ADD CONSTRAINT receipt_accepted_batch
  FOREIGN KEY(library_id,accepted_epoch,accepted_sequence)
  REFERENCES photara.library_change_batches(library_id,epoch,sequence)
  DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE photara.library_changes (
  library_id uuid NOT NULL,
  epoch uuid NOT NULL,
  sequence bigint NOT NULL,
  ordinal integer NOT NULL CHECK(ordinal>=0),
  entity_kind text NOT NULL CHECK(entity_kind IN
    ('library','person','organization','social-profile','person-organization-relationship',
     'location-kind','location','storage-root','project-catalog','project-locator')),
  entity_id uuid NOT NULL,
  entity_revision bigint NOT NULL CHECK(entity_revision>=1),
  change_kind text NOT NULL CHECK(change_kind IN ('create','update','tombstone','merge')),
  record_schema integer NOT NULL CHECK(record_schema>=1),
  post_state jsonb NOT NULL CHECK(jsonb_typeof(post_state)='object'),
  post_state_canonical bytea NOT NULL CHECK(octet_length(post_state_canonical) BETWEEN 2 AND 1048576),
  post_state_sha256 bytea NOT NULL CHECK(octet_length(post_state_sha256)=32),
  PRIMARY KEY(library_id,epoch,sequence,ordinal),
  UNIQUE(library_id,entity_kind,entity_id,entity_revision),
  FOREIGN KEY(library_id,epoch,sequence)
    REFERENCES photara.library_change_batches(library_id,epoch,sequence) ON DELETE RESTRICT
);
CREATE INDEX changes_entity ON photara.library_changes(library_id,entity_kind,entity_id,entity_revision);

CREATE TABLE photara_private.sync_clients (
  library_id uuid NOT NULL,
  account_id uuid NOT NULL,
  device_id uuid NOT NULL,
  stream_epoch uuid NOT NULL,
  acknowledged_sequence bigint NOT NULL DEFAULT 0 CHECK(acknowledged_sequence>=0),
  last_seen_at timestamptz NOT NULL,
  PRIMARY KEY(library_id,account_id,device_id),
  FOREIGN KEY(library_id,stream_epoch)
    REFERENCES photara_private.library_streams(library_id,epoch) ON DELETE RESTRICT,
  FOREIGN KEY(account_id,device_id)
    REFERENCES photara_identity.devices(account_id,device_id) ON DELETE RESTRICT
);
CREATE TABLE photara_private.security_audit (
  audit_id uuid PRIMARY KEY,
  actor_account_id uuid REFERENCES photara_identity.accounts ON DELETE RESTRICT,
  library_id uuid REFERENCES photara.libraries ON DELETE RESTRICT,
  action_code text NOT NULL,
  target_kind text NOT NULL,
  target_id uuid,
  occurred_at timestamptz NOT NULL,
  details jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(details)='object')
);
CREATE INDEX security_audit_library ON photara_private.security_audit(library_id,occurred_at,audit_id);
