-- CXT3c executable service migration; promoted from S4.
CREATE SCHEMA photara AUTHORIZATION photara_owner;
CREATE SCHEMA photara_identity AUTHORIZATION photara_owner;
CREATE SCHEMA photara_private AUTHORIZATION photara_owner;
REVOKE ALL ON SCHEMA photara,photara_identity,photara_private FROM PUBLIC;

CREATE TABLE photara.schema_metadata (
  singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
  schema_family text NOT NULL CHECK(schema_family='photara.service.g2'),
  schema_epoch integer NOT NULL CHECK(schema_epoch=1),
  minimum_api integer NOT NULL CHECK(minimum_api>=1),
  canonical_codec text NOT NULL CHECK(canonical_codec='photara.canonical-json.v1'),
  created_at timestamptz NOT NULL
);
CREATE TABLE photara.normalization_policies (
  policy_version integer PRIMARY KEY CHECK(policy_version>=1),
  unicode_version text NOT NULL,
  rules_sha256 bytea NOT NULL CHECK(octet_length(rules_sha256)=32),
  description text NOT NULL
);
CREATE TABLE photara_identity.accounts (
  account_id uuid PRIMARY KEY,
  display_name text NOT NULL,
  state text NOT NULL CHECK(state IN ('active','disabled','deleted')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  retired_at timestamptz,
  CHECK((state='active' AND retired_at IS NULL) OR
        (state IN ('disabled','deleted') AND retired_at IS NOT NULL))
);
CREATE TABLE photara_identity.account_identities (
  identity_id uuid PRIMARY KEY,
  account_id uuid NOT NULL REFERENCES photara_identity.accounts ON DELETE RESTRICT,
  issuer text COLLATE "C" NOT NULL CHECK(length(issuer)>0),
  subject text COLLATE "C" NOT NULL CHECK(length(subject)>0),
  state text NOT NULL CHECK(state IN ('active','revoked')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  revoked_at timestamptz,
  UNIQUE(issuer,subject),
  CHECK((state='revoked')=(revoked_at IS NOT NULL))
);
CREATE INDEX account_identities_account ON photara_identity.account_identities(account_id);

CREATE TABLE photara.libraries (
  library_id uuid PRIMARY KEY,
  display_name text NOT NULL CHECK(length(btrim(display_name))>0),
  state text NOT NULL CHECK(state IN ('active','tombstoned')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  retired_at timestamptz,
  term_policy_version integer NOT NULL REFERENCES photara.normalization_policies ON DELETE RESTRICT,
  extensions jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(extensions)='object'),
  CHECK((state='tombstoned')=(retired_at IS NOT NULL))
);
CREATE TABLE photara_identity.memberships (
  membership_id uuid PRIMARY KEY,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  account_id uuid NOT NULL REFERENCES photara_identity.accounts ON DELETE RESTRICT,
  role text NOT NULL CHECK(role IN ('owner','admin','editor','viewer')),
  state text NOT NULL CHECK(state IN ('active','revoked')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  revoked_at timestamptz,
  UNIQUE(library_id,account_id),
  CHECK((state='revoked')=(revoked_at IS NOT NULL))
);
CREATE INDEX memberships_account_active ON photara_identity.memberships(account_id,library_id)
  WHERE state='active';
CREATE INDEX memberships_library_role ON photara_identity.memberships(library_id,role)
  WHERE state='active';

CREATE TABLE photara_identity.devices (
  account_id uuid NOT NULL REFERENCES photara_identity.accounts ON DELETE RESTRICT,
  device_id uuid NOT NULL,
  display_name text NOT NULL,
  state text NOT NULL CHECK(state IN ('active','revoked')),
  registered_at timestamptz NOT NULL,
  last_seen_at timestamptz,
  PRIMARY KEY(account_id,device_id)
);
CREATE TABLE photara_private.library_claim_receipts (
  account_id uuid NOT NULL REFERENCES photara_identity.accounts ON DELETE RESTRICT,
  claim_id uuid NOT NULL,
  requested_library_id uuid NOT NULL,
  claimed_library_id uuid REFERENCES photara.libraries ON DELETE RESTRICT,
  request_canonical bytea NOT NULL CHECK(octet_length(request_canonical) BETWEEN 2 AND 1048576),
  request_sha256 bytea NOT NULL CHECK(octet_length(request_sha256)=32),
  outcome text NOT NULL CHECK(outcome IN ('claimed','conflict')),
  response_canonical bytea NOT NULL CHECK(octet_length(response_canonical) BETWEEN 2 AND 4194304),
  response_sha256 bytea NOT NULL CHECK(octet_length(response_sha256)=32),
  completed_at timestamptz NOT NULL,
  PRIMARY KEY(account_id,claim_id),
  CHECK((outcome='claimed')=(claimed_library_id IS NOT NULL)),
  CHECK(claimed_library_id IS NULL OR claimed_library_id=requested_library_id)
);
