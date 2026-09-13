# CXT3d Neon Generation Two activation

Completed 2026-09-13 from synchronized commit `be1ed80` in the existing Neon
project `photara` (`steep-waterfall-28781561`). This is a schema-only activation.
No Account, Identity, Device, Library, Project, package, media, or legacy user row
was seeded or imported.

## Branch topology

- The empty primary/default branch `production`
  (`br-shiny-glitter-afu6adtk`) was renamed to `main`.
- The populated child branch `development`
  (`br-crimson-glitter-affkn9pw`) was renamed to `legacy-v0.1.x`.
- `legacy-v0.1.x` remains unchanged with 34 public tables and a 20-entry SQLx
  migration ledger. It is reference/import source material only.
- No second Neon project or Generation Two database was created.

Because the legacy branch was forked before this activation, installing the new
schema on `main` did not install it on or rewrite `legacy-v0.1.x`.

## Role and migration boundary

The existing Neon owner is used only as the migration credential. Four Photara
NOLOGIN roles were created with no superuser, database creation, role creation, or
BYPASSRLS capability:

- `photara_owner`: migration/schema owner; the Neon owner may `SET ROLE` to it.
- `photara_api`: ordinary service capability role.
- `photara_control`: trusted controller capability role.
- `photara_auth_read`: identity/access-fact reader capability role.

The three runtime roles do not inherit or belong to `photara_owner` or one another.
No runtime LOGIN role, password, desktop connection string, or service-host secret
was created. Runtime credentials belong to the later service deployment boundary,
never to the native client.

The checked `photara_service::migrate` entry point applied the repository's exact
13 migrations over an encrypted Neon connection. A second run succeeded without
adding or rewriting migrations, proving the deployed family/floor/checksum ledger
is accepted by the same migration boundary.

## Live verification

The deployed `main/neondb` inventory matches the accepted CXT3c inventory:

| Check | Live result |
| --- | ---: |
| Schema family | `photara.service.g2` |
| Epoch / minimum API | `1 / 2` |
| Canonical codec | `photara.canonical-json.v1` |
| Successful migration ledger | `13 / 13` |
| Domain tables | 55 |
| Columns | 569 |
| Indexes | 153 |
| Non-internal triggers | 115 |
| Functions | 48 |
| Policies | 173 |
| RLS tables forced / enabled | `45 / 45` |
| Tables not owned by `photara_owner` | 0 |
| Accounts / Libraries / Projects | `0 / 0 / 0` |

Every live SQLx SHA-384 checksum for versions 1–13 equals
`CXT3C_POSTGRES_INVENTORY.json`. The existing CXT3c disposable PostgreSQL suites
remain the executable authorization proof; this activation did not manufacture
runtime logins merely to repeat those tests against production.

## Next separately bounded sequence: CXT4 onboarding and opening shell

The detailed approved sequence is now
[CXT4 onboarding and opening Library shell](CXT4_ONBOARDING_AND_OPENING.md).

Do not replace onboarding with a developer seed. Implement and prove the actual
native user path:

1. Auth0 Authorization Code + PKCE with Google sign-in and a production claim
   verifier at the service boundary.
2. Exact verified `(issuer, subject)` identity resolution; never infer identity
   from email.
3. Idempotent Account creation/recovery and device registration.
4. Default `My Library` creation locally and in Neon, with explicit ownership,
   membership, contract state, empty scoped stream, and durable reconciliation if
   either side succeeds first.
5. Returning-user recovery without creating duplicate Accounts or Libraries.
6. No privileged Neon credential in the macOS application. Provision separate
   runtime LOGIN roles only in approved service-host secret storage.

The onboarding slice must define cancellation, offline/local-only choice, Auth0 or
network failure recovery, retries after unknown outcomes, and the transition from
an existing local `My Library` created by CXT3b. CloudKit, media providers, legacy
import, and package publication remain separate.
