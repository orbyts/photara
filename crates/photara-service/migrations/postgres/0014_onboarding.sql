-- CXT4b additive onboarding boundary. Never installed by runtime startup.
ALTER TABLE photara_identity.account_identities
  ADD CONSTRAINT onboarding_exact_identity UNIQUE(issuer,subject,identity_id,account_id);

CREATE TABLE photara_identity.account_defaults (
  account_id uuid PRIMARY KEY REFERENCES photara_identity.accounts ON DELETE RESTRICT,
  library_id uuid NOT NULL REFERENCES photara.libraries ON DELETE RESTRICT,
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL CHECK(updated_at>=created_at),
  CHECK(account_id<>'00000000-0000-0000-0000-000000000000'),
  CHECK(library_id<>'00000000-0000-0000-0000-000000000000')
);
CREATE INDEX account_defaults_library ON photara_identity.account_defaults(library_id);

CREATE TABLE photara_identity.device_credentials (
  account_id uuid NOT NULL,
  device_id uuid NOT NULL CHECK(device_id<>'00000000-0000-0000-0000-000000000000'),
  commitment bytea NOT NULL UNIQUE CHECK(octet_length(commitment)=32 AND commitment<>decode(repeat('00',32),'hex')),
  state text NOT NULL CHECK(state IN ('active','suspended')),
  revision bigint NOT NULL CHECK(revision>=1),
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL CHECK(updated_at>=created_at),
  PRIMARY KEY(account_id,device_id),
  FOREIGN KEY(account_id,device_id) REFERENCES photara_identity.devices ON DELETE RESTRICT
);

CREATE TABLE photara_private.onboarding_receipts (
  issuer text COLLATE "C" NOT NULL,
  subject text COLLATE "C" NOT NULL,
  operation_id uuid NOT NULL CHECK(operation_id<>'00000000-0000-0000-0000-000000000000'),
  identity_id uuid NOT NULL,
  account_id uuid NOT NULL,
  device_id uuid NOT NULL,
  device_commitment bytea NOT NULL CHECK(octet_length(device_commitment)=32),
  action text NOT NULL CHECK(action IN ('bootstrap','resume','logout')),
  request_canonical bytea NOT NULL CHECK(octet_length(request_canonical) BETWEEN 2 AND 65536),
  request_sha256 bytea NOT NULL CHECK(request_sha256=sha256(request_canonical)),
  response_canonical bytea NOT NULL CHECK(octet_length(response_canonical) BETWEEN 2 AND 65536),
  response_sha256 bytea NOT NULL CHECK(response_sha256=sha256(response_canonical)),
  completed_at timestamptz NOT NULL,
  PRIMARY KEY(issuer,subject,operation_id),
  FOREIGN KEY(issuer,subject,identity_id,account_id) REFERENCES photara_identity.account_identities(issuer,subject,identity_id,account_id) ON DELETE RESTRICT,
  FOREIGN KEY(account_id,device_id) REFERENCES photara_identity.device_credentials ON DELETE RESTRICT
);
CREATE INDEX onboarding_receipts_identity ON photara_private.onboarding_receipts(issuer,subject,identity_id,account_id);
CREATE INDEX onboarding_receipts_device ON photara_private.onboarding_receipts(account_id,device_id);

CREATE TABLE photara_private.onboarding_challenges (
  challenge_id uuid PRIMARY KEY CHECK(challenge_id<>'00000000-0000-0000-0000-000000000000'),
  nonce_sha256 bytea NOT NULL UNIQUE CHECK(octet_length(nonce_sha256)=32),
  action text NOT NULL CHECK(action IN ('bootstrap','resume')),
  operation_id uuid NOT NULL CHECK(operation_id<>'00000000-0000-0000-0000-000000000000'),
  request_sha256 bytea NOT NULL CHECK(octet_length(request_sha256)=32),
  device_id uuid NOT NULL CHECK(device_id<>'00000000-0000-0000-0000-000000000000'),
  device_commitment bytea NOT NULL CHECK(octet_length(device_commitment)=32 AND device_commitment<>decode(repeat('00',32),'hex')),
  issued_at timestamptz NOT NULL,
  expires_at timestamptz NOT NULL CHECK(expires_at=issued_at+interval '5 minutes'),
  consumed_at timestamptz,
  consuming_issuer text COLLATE "C",
  consuming_subject text COLLATE "C",
  CHECK((consumed_at IS NULL AND consuming_issuer IS NULL AND consuming_subject IS NULL) OR
        (consumed_at>=issued_at AND consumed_at<expires_at AND consuming_issuer IS NOT NULL AND consuming_subject IS NOT NULL)),
  FOREIGN KEY(consuming_issuer,consuming_subject,operation_id)
    REFERENCES photara_private.onboarding_receipts(issuer,subject,operation_id)
    ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX onboarding_challenges_expiry ON photara_private.onboarding_challenges(expires_at);
CREATE INDEX onboarding_challenges_receipt ON photara_private.onboarding_challenges(consuming_issuer,consuming_subject,operation_id);

CREATE FUNCTION photara_private.guard_onboarding_record() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,photara_private,photara_identity AS $$
BEGIN
  IF TG_TABLE_NAME='onboarding_receipts' OR TG_TABLE_NAME='account_defaults' THEN
    RAISE EXCEPTION 'immutable onboarding record' USING ERRCODE='23514';
  ELSIF TG_TABLE_NAME='device_credentials' THEN
    IF NEW.account_id<>OLD.account_id OR NEW.device_id<>OLD.device_id OR NEW.commitment<>OLD.commitment
       OR NEW.created_at<>OLD.created_at OR NEW.revision<>OLD.revision+1 OR NEW.state=OLD.state
       OR NEW.updated_at<OLD.updated_at THEN
      RAISE EXCEPTION 'invalid credential transition' USING ERRCODE='23514';
    END IF;
  ELSIF TG_OP='DELETE' THEN
    IF OLD.expires_at>clock_timestamp()-interval '24 hours' THEN
      RAISE EXCEPTION 'challenge retention' USING ERRCODE='23514';
    END IF;
    RETURN OLD;
  ELSIF OLD.consumed_at IS NOT NULL OR NEW.consumed_at IS NULL
    OR (to_jsonb(NEW)-ARRAY['consumed_at','consuming_issuer','consuming_subject'])<>
       (to_jsonb(OLD)-ARRAY['consumed_at','consuming_issuer','consuming_subject']) THEN
    RAISE EXCEPTION 'immutable challenge binding' USING ERRCODE='23514';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER onboarding_receipts_immutable BEFORE UPDATE OR DELETE ON photara_private.onboarding_receipts FOR EACH ROW EXECUTE FUNCTION photara_private.guard_onboarding_record();
CREATE TRIGGER account_defaults_immutable BEFORE UPDATE OR DELETE ON photara_identity.account_defaults FOR EACH ROW EXECUTE FUNCTION photara_private.guard_onboarding_record();
CREATE TRIGGER device_credentials_transition BEFORE UPDATE ON photara_identity.device_credentials FOR EACH ROW EXECUTE FUNCTION photara_private.guard_onboarding_record();
CREATE TRIGGER onboarding_challenges_transition BEFORE UPDATE OR DELETE ON photara_private.onboarding_challenges FOR EACH ROW EXECUTE FUNCTION photara_private.guard_onboarding_record();

CREATE FUNCTION photara_private.close_onboarding_challenge() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,photara_private AS $$
BEGIN
  IF NEW.consumed_at IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM photara_private.onboarding_receipts r WHERE r.issuer=NEW.consuming_issuer
      AND r.subject=NEW.consuming_subject AND r.operation_id=NEW.operation_id
      AND r.device_id=NEW.device_id AND r.device_commitment=NEW.device_commitment
      AND r.request_sha256=NEW.request_sha256 AND r.action=NEW.action
  ) THEN
    RAISE EXCEPTION 'challenge receipt closure' USING ERRCODE='23514';
  END IF;
  RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER onboarding_challenge_closure AFTER INSERT OR UPDATE ON photara_private.onboarding_challenges DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.close_onboarding_challenge();

CREATE FUNCTION photara_private.close_onboarding_receipt() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,photara_private AS $$
BEGIN
  IF NEW.action IN ('bootstrap','resume') AND NOT EXISTS (
    SELECT 1 FROM photara_private.onboarding_challenges c
    WHERE c.consuming_issuer=NEW.issuer AND c.consuming_subject=NEW.subject
      AND c.operation_id=NEW.operation_id AND c.action=NEW.action
      AND c.request_sha256=NEW.request_sha256 AND c.device_id=NEW.device_id
      AND c.device_commitment=NEW.device_commitment AND c.consumed_at IS NOT NULL
  ) THEN RAISE EXCEPTION 'receipt challenge closure' USING ERRCODE='23514'; END IF;
  RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER onboarding_receipt_closure AFTER INSERT ON photara_private.onboarding_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION photara_private.close_onboarding_receipt();
CREATE FUNCTION photara_private.guard_default_owner() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,photara_identity,photara AS $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM photara_identity.memberships m
    JOIN photara_identity.accounts a USING(account_id) JOIN photara.libraries l USING(library_id)
    WHERE m.account_id=NEW.account_id AND m.library_id=NEW.library_id
      AND m.state='active' AND m.role='owner' AND a.state='active' AND l.state='active')
  THEN RAISE EXCEPTION 'default requires active ownership' USING ERRCODE='23514'; END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER account_defaults_owner BEFORE INSERT ON photara_identity.account_defaults FOR EACH ROW EXECUTE FUNCTION photara_private.guard_default_owner();

-- Control-only relations follow the existing identity/private privilege boundary.
-- No API/PUBLIC grant and no public RLS policy is introduced.
REVOKE ALL ON photara_identity.account_defaults,photara_identity.device_credentials,
  photara_private.onboarding_receipts,photara_private.onboarding_challenges FROM PUBLIC,photara_api,photara_auth_read;
GRANT SELECT,INSERT ON photara_identity.account_defaults,photara_private.onboarding_receipts TO photara_control;
-- PostgreSQL requires UPDATE privilege for FOR UPDATE; the immutable trigger
-- still refuses every actual default-pointer update.
GRANT UPDATE ON photara_identity.account_defaults TO photara_control;
GRANT SELECT,INSERT,UPDATE ON photara_identity.device_credentials TO photara_control;
GRANT SELECT,INSERT,UPDATE,DELETE ON photara_private.onboarding_challenges TO photara_control;
REVOKE ALL ON FUNCTION photara_private.guard_onboarding_record(),photara_private.close_onboarding_challenge() FROM PUBLIC,photara_api,photara_auth_read;
REVOKE ALL ON FUNCTION photara_private.close_onboarding_receipt(),photara_private.guard_default_owner() FROM PUBLIC,photara_api,photara_auth_read;
UPDATE photara.schema_metadata SET minimum_api=3 WHERE singleton;
