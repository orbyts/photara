-- CXT3c executable service migration; promoted from S4.
CREATE TABLE photara_private.billing_events (
  provider text COLLATE "C" NOT NULL,
  event_id text COLLATE "C" NOT NULL,
  payload_sha256 bytea NOT NULL CHECK(octet_length(payload_sha256)=32),
  received_at timestamptz NOT NULL,
  event_kind text NOT NULL,
  payload jsonb NOT NULL CHECK(jsonb_typeof(payload)='object'),
  PRIMARY KEY(provider,event_id)
);
CREATE TABLE photara_private.library_subscriptions (
  subscription_id uuid PRIMARY KEY,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  provider text COLLATE "C" NOT NULL,
  provider_subscription_id text COLLATE "C" NOT NULL,
  plan_key text NOT NULL,
  state text NOT NULL CHECK(state IN ('trialing','active','past-due','paused','cancelled')),
  revision bigint NOT NULL CHECK(revision>=1),
  period_start timestamptz NOT NULL,
  period_end timestamptz NOT NULL CHECK(period_end>period_start),
  source_event_id text COLLATE "C" NOT NULL,
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  UNIQUE(provider,provider_subscription_id),
  UNIQUE(library_id,subscription_id),
  FOREIGN KEY(provider,source_event_id)
    REFERENCES photara_private.billing_events(provider,event_id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX subscriptions_one_current ON photara_private.library_subscriptions(library_id)
  WHERE state IN ('trialing','active','past-due','paused');

CREATE TABLE photara_private.library_entitlement_grants (
  grant_id uuid PRIMARY KEY,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  capability_key text COLLATE "C" NOT NULL CHECK(length(capability_key)>0),
  source text NOT NULL CHECK(source IN ('subscription','manual')),
  subscription_id uuid,
  enabled boolean NOT NULL,
  quota_limit bigint CHECK(quota_limit IS NULL OR quota_limit>=0),
  valid_from timestamptz NOT NULL,
  valid_until timestamptz,
  state text NOT NULL CHECK(state IN ('active','revoked')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  CHECK(valid_until IS NULL OR valid_until>valid_from),
  CHECK((source='subscription')=(subscription_id IS NOT NULL)),
  FOREIGN KEY(library_id,subscription_id)
    REFERENCES photara_private.library_subscriptions(library_id,subscription_id) ON DELETE RESTRICT
);
CREATE INDEX library_grants_lookup ON photara_private.library_entitlement_grants(library_id,capability_key,valid_from)
  WHERE state='active';

CREATE TABLE photara_private.account_developer_grants (
  grant_id uuid PRIMARY KEY,
  account_id uuid NOT NULL REFERENCES photara_identity.accounts ON DELETE RESTRICT,
  capability_key text COLLATE "C" NOT NULL CHECK(length(capability_key)>0),
  reason_code text NOT NULL,
  valid_from timestamptz NOT NULL,
  valid_until timestamptz,
  state text NOT NULL CHECK(state IN ('active','revoked')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  CHECK(valid_until IS NULL OR valid_until>valid_from)
);
CREATE INDEX developer_grants_lookup ON photara_private.account_developer_grants(account_id,capability_key,valid_from)
  WHERE state='active';

CREATE TABLE photara.library_media (
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  sha256 bytea NOT NULL CHECK(octet_length(sha256)=32),
  media_type text NOT NULL,
  byte_length bigint NOT NULL CHECK(byte_length>=0),
  width integer CHECK(width IS NULL OR width>0),
  height integer CHECK(height IS NULL OR height>0),
  created_at timestamptz NOT NULL,
  PRIMARY KEY(library_id,sha256)
);
CREATE TABLE photara_private.media_objects (
  library_id uuid NOT NULL,
  sha256 bytea NOT NULL,
  object_key text COLLATE "C" NOT NULL UNIQUE CHECK(length(object_key)>0),
  state text NOT NULL CHECK(state IN ('pending','available','quarantined')),
  verified_at timestamptz,
  created_at timestamptz NOT NULL,
  CHECK(state<>'available' OR verified_at IS NOT NULL),
  PRIMARY KEY(library_id,sha256),
  FOREIGN KEY(library_id,sha256)
    REFERENCES photara.library_media(library_id,sha256) ON DELETE RESTRICT
);
CREATE TABLE photara_private.media_upload_sessions (
  upload_id uuid PRIMARY KEY,
  library_id uuid NOT NULL,
  sha256 bytea NOT NULL,
  staging_object_key text COLLATE "C" NOT NULL UNIQUE CHECK(length(staging_object_key)>0),
  state text NOT NULL CHECK(state IN ('pending','verifying','complete','failed','expired')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL,
  expires_at timestamptz NOT NULL CHECK(expires_at>created_at),
  completed_at timestamptz,
  CHECK((state='complete')=(completed_at IS NOT NULL)),
  FOREIGN KEY(library_id,sha256)
    REFERENCES photara.library_media(library_id,sha256) ON DELETE RESTRICT
);
CREATE INDEX media_upload_pending ON photara_private.media_upload_sessions(library_id,expires_at)
  WHERE state IN ('pending','verifying');
