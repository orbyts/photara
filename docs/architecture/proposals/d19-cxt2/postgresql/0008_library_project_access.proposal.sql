-- INERT PROPOSAL ONLY — CXT2, accepted design 2026-09-12.
-- NOT AN INSTALLED, APPLIED, OR RUNNABLE MIGRATION. DO NOT EXECUTE.
-- Review text outside every runtime migration directory; no runner references it.
-- Implementation/adaptation and disposable execution require separate CXT3 scope.
-- Read ../README.md and ../RESPONSIBILITIES.md before reviewing this SQL.
-- Reservations map to D19_STATIC_SCHEMA_DELTA.md; baseline bytes remain intact.

-- A1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.library_contract_state (
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  contract_version integer NOT NULL,
  authority_mode text COLLATE "C" NOT NULL,
  local_principal_id uuid CHECK (local_principal_id IS NULL OR (local_principal_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  authorization_generation bigint NOT NULL CHECK (authorization_generation>=1),
  record_schema integer NOT NULL,
  revision bigint NOT NULL CHECK (revision>=1),
  created_at timestamptz(3) NOT NULL,
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (workspace_id),
  CHECK (contract_version=1),
  CHECK (authority_mode='cloud-member'),
  CHECK ((authority_mode='local-only')=(local_principal_id IS NOT NULL)),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
);

-- A2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.project_ownership (
  project_id uuid NOT NULL CHECK (project_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  registration_state text COLLATE "C" NOT NULL,
  association_commit_id uuid CHECK (association_commit_id IS NULL OR (association_commit_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  association_commit_sha256 bytea CHECK (association_commit_sha256 IS NULL OR (octet_length(association_commit_sha256)=32)),
  source_origin_library_id uuid CHECK (source_origin_library_id IS NULL OR (source_origin_library_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  source_format text COLLATE "C" NOT NULL,
  record_schema integer NOT NULL,
  revision bigint NOT NULL CHECK (revision>=1),
  created_at timestamptz(3) NOT NULL,
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (project_id),
  UNIQUE (workspace_id,project_id),
  CHECK (registration_state IN ('pending','active','closed')),
  CHECK ((association_commit_id IS NULL)=(association_commit_sha256 IS NULL)),
  CHECK (registration_state<>'active' OR association_commit_id IS NOT NULL),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
);

CREATE INDEX d19_project_ownership_library_state ON photara.project_ownership (workspace_id,registration_state,project_id);

-- A3: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara.project_access_policies (
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  project_id uuid NOT NULL CHECK (project_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  visibility_policy text COLLATE "C" NOT NULL,
  owner_mask integer NOT NULL,
  admin_mask integer NOT NULL,
  editor_mask integer NOT NULL,
  viewer_mask integer NOT NULL,
  authorization_generation bigint NOT NULL CHECK (authorization_generation>=1),
  record_schema integer NOT NULL,
  revision bigint NOT NULL CHECK (revision>=1),
  created_at timestamptz(3) NOT NULL,
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (workspace_id,project_id),
  FOREIGN KEY (workspace_id,project_id) REFERENCES photara.project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  CHECK (visibility_policy IN ('restricted','library-visible')),
  CHECK (owner_mask IN (0,1,3,71,11,79)),
  CHECK (admin_mask IN (0,1,3,71,11,79)),
  CHECK (editor_mask IN (0,1,3,71,11,79)),
  CHECK (viewer_mask IN (0,1,3,71,11,79)),
  CHECK (visibility_policy<>'restricted' OR (owner_mask=0 AND admin_mask=0 AND editor_mask=0 AND viewer_mask=0)),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
);

-- A4: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara_identity.project_access_grants (
  grant_id uuid NOT NULL CHECK (grant_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  project_id uuid NOT NULL CHECK (project_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  account_id uuid CHECK (account_id IS NULL OR (account_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  local_principal_id uuid CHECK (local_principal_id IS NULL OR (local_principal_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  action_mask integer NOT NULL,
  state text COLLATE "C" NOT NULL,
  revoked_at timestamptz(3),
  record_schema integer NOT NULL,
  revision bigint NOT NULL CHECK (revision>=1),
  created_at timestamptz(3) NOT NULL,
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (grant_id),
  UNIQUE (workspace_id,project_id,grant_id),
  FOREIGN KEY (workspace_id,project_id) REFERENCES photara.project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  FOREIGN KEY (account_id) REFERENCES photara_identity.accounts (account_id) ON DELETE RESTRICT,
  CHECK ((account_id IS NULL)<>(local_principal_id IS NULL)),
  CHECK (state IN ('active','revoked')),
  CHECK ((state='revoked')=(revoked_at IS NOT NULL)),
  CHECK (action_mask BETWEEN 0 AND 255 AND ((action_mask & 2)=0 OR (action_mask & 1)=1) AND ((action_mask & 252)=0 OR (action_mask & 3)=3) AND ((action_mask & 128)=0 OR (action_mask & 16)=16)),
  CHECK (account_id IS NOT NULL AND local_principal_id IS NULL),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
);

CREATE UNIQUE INDEX d19_project_access_grants_account_id ON photara_identity.project_access_grants (workspace_id,project_id,account_id) WHERE account_id IS NOT NULL;

CREATE UNIQUE INDEX d19_project_access_grants_local_principal_id ON photara_identity.project_access_grants (workspace_id,project_id,local_principal_id) WHERE local_principal_id IS NOT NULL;

CREATE INDEX d19_project_access_grants_account ON photara_identity.project_access_grants (account_id,state,workspace_id,project_id);

-- A5: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara_identity.project_invitations (
  invitation_id uuid NOT NULL CHECK (invitation_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  workspace_id uuid NOT NULL CHECK (workspace_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  project_id uuid NOT NULL CHECK (project_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  inviter_account_id uuid NOT NULL CHECK (inviter_account_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  target_account_id uuid NOT NULL CHECK (target_account_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  action_mask integer NOT NULL,
  expected_policy_revision bigint NOT NULL CHECK (expected_policy_revision>=1),
  expires_at timestamptz(3) NOT NULL,
  state text COLLATE "C" NOT NULL,
  accepted_grant_id uuid CHECK (accepted_grant_id IS NULL OR (accepted_grant_id<>'00000000-0000-0000-0000-000000000000'::uuid)),
  completed_at timestamptz(3),
  record_schema integer NOT NULL,
  revision bigint NOT NULL CHECK (revision>=1),
  created_at timestamptz(3) NOT NULL,
  updated_at timestamptz(3) NOT NULL,
  PRIMARY KEY (invitation_id),
  FOREIGN KEY (workspace_id,project_id) REFERENCES photara.project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  FOREIGN KEY (inviter_account_id) REFERENCES photara_identity.accounts (account_id) ON DELETE RESTRICT,
  FOREIGN KEY (target_account_id) REFERENCES photara_identity.accounts (account_id) ON DELETE RESTRICT,
  FOREIGN KEY (workspace_id,project_id,accepted_grant_id) REFERENCES photara_identity.project_access_grants (workspace_id,project_id,grant_id) ON DELETE RESTRICT,
  CHECK (state IN ('pending','accepted','declined','revoked','expired')),
  CHECK ((state='pending')=(completed_at IS NULL)),
  CHECK ((state='accepted')=(accepted_grant_id IS NOT NULL)),
  CHECK (action_mask BETWEEN 0 AND 255 AND ((action_mask & 2)=0 OR (action_mask & 1)=1) AND ((action_mask & 252)=0 OR (action_mask & 3)=3) AND ((action_mask & 128)=0 OR (action_mask & 16)=16)),
  CHECK (expires_at>created_at AND expires_at<=created_at+interval '7 days'),
  FOREIGN KEY (workspace_id) REFERENCES photara.workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
);

CREATE UNIQUE INDEX d19_project_invitations_pending ON photara_identity.project_invitations (workspace_id,project_id,target_account_id) WHERE state='pending';

CREATE INDEX d19_project_invitations_target ON photara_identity.project_invitations (target_account_id,state,expires_at,invitation_id);

CREATE INDEX d19_project_invitations_inviter ON photara_identity.project_invitations (inviter_account_id);

-- C1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE photara_private.project_invitation_secrets (
  invitation_id uuid NOT NULL CHECK (invitation_id<>'00000000-0000-0000-0000-000000000000'::uuid),
  token_verifier bytea NOT NULL CHECK (octet_length(token_verifier)=32),
  verifier_version integer NOT NULL,
  consumed_at timestamptz(3),
  PRIMARY KEY (invitation_id),
  FOREIGN KEY (invitation_id) REFERENCES photara_identity.project_invitations (invitation_id) ON DELETE RESTRICT,
  CHECK (verifier_version=1)
);

CREATE INDEX d19_project_invitations_fk1 ON photara_identity.project_invitations (workspace_id,project_id);

CREATE INDEX d19_project_invitations_fk4 ON photara_identity.project_invitations (workspace_id,project_id,accepted_grant_id);
