-- INERT PROPOSAL ONLY — CXT2, accepted design 2026-09-12.
-- NOT AN INSTALLED, APPLIED, OR RUNNABLE MIGRATION. DO NOT EXECUTE.
-- Review text outside every runtime migration directory; no runner references it.
-- Implementation/adaptation and disposable execution require separate CXT3 scope.
-- Read ../README.md and ../RESPONSIBILITIES.md before reviewing this SQL.
-- Reservations map to D19_STATIC_SCHEMA_DELTA.md; baseline bytes remain intact.

-- A1: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE library_contract_state (
  workspace_id BLOB NOT NULL CHECK (length(workspace_id)=16 AND workspace_id<>zeroblob(16)),
  contract_version INTEGER NOT NULL,
  authority_mode TEXT NOT NULL,
  local_principal_id BLOB CHECK (local_principal_id IS NULL OR (length(local_principal_id)=16 AND local_principal_id<>zeroblob(16))),
  authorization_generation INTEGER NOT NULL CHECK (authorization_generation>=1),
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (workspace_id),
  CHECK (contract_version=1),
  CHECK (authority_mode IN ('local-only','cloud-member','project-only')),
  CHECK ((authority_mode='local-only')=(local_principal_id IS NOT NULL)),
  FOREIGN KEY (workspace_id) REFERENCES workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

-- A2: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE project_ownership (
  project_id BLOB NOT NULL CHECK (length(project_id)=16 AND project_id<>zeroblob(16)),
  workspace_id BLOB NOT NULL CHECK (length(workspace_id)=16 AND workspace_id<>zeroblob(16)),
  registration_state TEXT NOT NULL,
  association_commit_id BLOB CHECK (association_commit_id IS NULL OR (length(association_commit_id)=16 AND association_commit_id<>zeroblob(16))),
  association_commit_sha256 BLOB CHECK (association_commit_sha256 IS NULL OR (length(association_commit_sha256)=32)),
  source_origin_library_id BLOB CHECK (source_origin_library_id IS NULL OR (length(source_origin_library_id)=16 AND source_origin_library_id<>zeroblob(16))),
  source_format TEXT NOT NULL,
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (project_id),
  UNIQUE (workspace_id,project_id),
  CHECK (registration_state IN ('pending','active','closed')),
  CHECK ((association_commit_id IS NULL)=(association_commit_sha256 IS NULL)),
  CHECK (registration_state<>'active' OR association_commit_id IS NOT NULL),
  FOREIGN KEY (workspace_id) REFERENCES workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

CREATE INDEX d19_project_ownership_library_state ON project_ownership (workspace_id,registration_state,project_id);

-- A3: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE project_access_policies (
  workspace_id BLOB NOT NULL CHECK (length(workspace_id)=16 AND workspace_id<>zeroblob(16)),
  project_id BLOB NOT NULL CHECK (length(project_id)=16 AND project_id<>zeroblob(16)),
  visibility_policy TEXT NOT NULL,
  owner_mask INTEGER NOT NULL,
  admin_mask INTEGER NOT NULL,
  editor_mask INTEGER NOT NULL,
  viewer_mask INTEGER NOT NULL,
  authorization_generation INTEGER NOT NULL CHECK (authorization_generation>=1),
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (workspace_id,project_id),
  FOREIGN KEY (workspace_id,project_id) REFERENCES project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  CHECK (visibility_policy IN ('restricted','library-visible')),
  CHECK (owner_mask IN (0,1,3,71,11,79)),
  CHECK (admin_mask IN (0,1,3,71,11,79)),
  CHECK (editor_mask IN (0,1,3,71,11,79)),
  CHECK (viewer_mask IN (0,1,3,71,11,79)),
  CHECK (visibility_policy<>'restricted' OR (owner_mask=0 AND admin_mask=0 AND editor_mask=0 AND viewer_mask=0)),
  FOREIGN KEY (workspace_id) REFERENCES workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

-- A4: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE project_access_grants (
  grant_id BLOB NOT NULL CHECK (length(grant_id)=16 AND grant_id<>zeroblob(16)),
  workspace_id BLOB NOT NULL CHECK (length(workspace_id)=16 AND workspace_id<>zeroblob(16)),
  project_id BLOB NOT NULL CHECK (length(project_id)=16 AND project_id<>zeroblob(16)),
  account_id BLOB CHECK (account_id IS NULL OR (length(account_id)=16 AND account_id<>zeroblob(16))),
  local_principal_id BLOB CHECK (local_principal_id IS NULL OR (length(local_principal_id)=16 AND local_principal_id<>zeroblob(16))),
  action_mask INTEGER NOT NULL,
  state TEXT NOT NULL,
  revoked_at INTEGER,
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (grant_id),
  UNIQUE (workspace_id,project_id,grant_id),
  FOREIGN KEY (workspace_id,project_id) REFERENCES project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  FOREIGN KEY (account_id) REFERENCES account_cache (account_id) ON DELETE RESTRICT,
  CHECK ((account_id IS NULL)<>(local_principal_id IS NULL)),
  CHECK (state IN ('active','revoked')),
  CHECK ((state='revoked')=(revoked_at IS NOT NULL)),
  CHECK (action_mask BETWEEN 0 AND 255 AND ((action_mask & 2)=0 OR (action_mask & 1)=1) AND ((action_mask & 252)=0 OR (action_mask & 3)=3) AND ((action_mask & 128)=0 OR (action_mask & 16)=16)),
  FOREIGN KEY (workspace_id) REFERENCES workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

CREATE UNIQUE INDEX d19_project_access_grants_account_id ON project_access_grants (workspace_id,project_id,account_id) WHERE account_id IS NOT NULL;

CREATE UNIQUE INDEX d19_project_access_grants_local_principal_id ON project_access_grants (workspace_id,project_id,local_principal_id) WHERE local_principal_id IS NOT NULL;

CREATE INDEX d19_project_access_grants_account ON project_access_grants (account_id,state,workspace_id,project_id);

-- A5: approved signature in D19_STATIC_SCHEMA_DELTA.md.
CREATE TABLE project_invitations (
  invitation_id BLOB NOT NULL CHECK (length(invitation_id)=16 AND invitation_id<>zeroblob(16)),
  workspace_id BLOB NOT NULL CHECK (length(workspace_id)=16 AND workspace_id<>zeroblob(16)),
  project_id BLOB NOT NULL CHECK (length(project_id)=16 AND project_id<>zeroblob(16)),
  inviter_account_id BLOB NOT NULL CHECK (length(inviter_account_id)=16 AND inviter_account_id<>zeroblob(16)),
  target_account_id BLOB NOT NULL CHECK (length(target_account_id)=16 AND target_account_id<>zeroblob(16)),
  action_mask INTEGER NOT NULL,
  expected_policy_revision INTEGER NOT NULL CHECK (expected_policy_revision>=1),
  expires_at INTEGER NOT NULL,
  state TEXT NOT NULL,
  accepted_grant_id BLOB CHECK (accepted_grant_id IS NULL OR (length(accepted_grant_id)=16 AND accepted_grant_id<>zeroblob(16))),
  completed_at INTEGER,
  record_schema INTEGER NOT NULL,
  local_revision INTEGER NOT NULL CHECK (local_revision>=1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (invitation_id),
  FOREIGN KEY (workspace_id,project_id) REFERENCES project_ownership (workspace_id,project_id) ON DELETE RESTRICT,
  FOREIGN KEY (inviter_account_id) REFERENCES account_cache (account_id) ON DELETE RESTRICT,
  FOREIGN KEY (target_account_id) REFERENCES account_cache (account_id) ON DELETE RESTRICT,
  FOREIGN KEY (workspace_id,project_id,accepted_grant_id) REFERENCES project_access_grants (workspace_id,project_id,grant_id) ON DELETE RESTRICT,
  CHECK (state IN ('pending','accepted','declined','revoked','expired')),
  CHECK ((state='pending')=(completed_at IS NULL)),
  CHECK ((state='accepted')=(accepted_grant_id IS NOT NULL)),
  CHECK (action_mask BETWEEN 0 AND 255 AND ((action_mask & 2)=0 OR (action_mask & 1)=1) AND ((action_mask & 252)=0 OR (action_mask & 3)=3) AND ((action_mask & 128)=0 OR (action_mask & 16)=16)),
  CHECK (expires_at>created_at AND expires_at<=created_at+604800000),
  FOREIGN KEY (workspace_id) REFERENCES workspaces (workspace_id) ON DELETE RESTRICT,
  CHECK (record_schema=1),
  CHECK (updated_at>=created_at)
) STRICT;

CREATE UNIQUE INDEX d19_project_invitations_pending ON project_invitations (workspace_id,project_id,target_account_id) WHERE state='pending';

CREATE INDEX d19_project_invitations_target ON project_invitations (target_account_id,state,expires_at,invitation_id);

CREATE INDEX d19_project_invitations_inviter ON project_invitations (inviter_account_id);

CREATE INDEX d19_project_invitations_fk1 ON project_invitations (workspace_id,project_id);

CREATE INDEX d19_project_invitations_fk4 ON project_invitations (workspace_id,project_id,accepted_grant_id);
