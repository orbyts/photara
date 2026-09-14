# CXT4 development cloud and product identity

Approved direction recorded 2026-09-13. This document separates the permanent
cloud architecture from its development deployment policy and makes the current
`Photara` name an intentionally replaceable codename.

## Invariants

- The production desktop application never contains a Neon connection string or
  privileged database credential. Auth0 proves identity; the service enforces
  authorization and domain rules; Neon stores cloud state.
- Local-only use remains independent of Auth0, Fly.io and Neon.
- Neon `main` is the shared development cloud source of truth for Suhail's
  development Libraries and Projects. Each development Mac may keep a SQLite
  cache/replica; it is not an independent cloud authority.
- Local and hosted service adapters expose the same versioned HTTP DTOs, token
  verification, bootstrap semantics and database role boundaries. Environment
  selection changes deployment coordinates, not product behavior.
- No public release depends on development tenant identifiers, callback schemes,
  hostnames, credentials or the `Photara` codename.

## Deployment profiles

### Local-only

The native client uses local SQLite and creates the default `My Library` without
authentication. No cloud process or provider is required.

### Development cloud

The native Auth0/Google browser flow is real and uses the dedicated development
tenant/client/API. After callback, the native client calls the same Rust/Axum API
contract at loopback. The API is launched as a separately configured development
process or sidecar and reads scoped development secrets from host-local secure
configuration; secrets are never compiled into the app, committed, logged or
stored in a Project package. It connects to Neon `main`, so signing in with the
same Auth0 identity from another configured development Mac resolves the same
Account, default Library and cloud content. A new Mac starts with an empty local
cache and hydrates authorized cloud state after sign-in.

This is an operator/developer facility, not a distributable customer mode. A
normal user build cannot select it or receive its Neon credentials.

### Remote acceptance

At explicit acceptance gates, deploy the identical image and configuration model
to Fly.io, verify public HTTPS ingress, encrypted secret injection, Auth0 JWKS and
Neon connectivity, then destroy the Fly Machine. Merely stopping a Machine is not
the cleanup contract because stopped root filesystems remain billable. Keep the
checked deployment manifest and reproducible commands; do not keep paid compute
provisioned between gates.

### Production cloud

After the production identity, company/account ownership, domain, Apple signing,
website, subscription policy and operational budget are ready, deploy the same
service permanently to Fly.io. The production client uses only the production
HTTPS origin and production Auth0/Neon resources. Cloud subscription pricing must
include API compute/network, database/storage/backups, authentication, monitoring,
support, payment-channel fees and operating margin. It is not derived from one
Fly Machine divided by one user; the API is shared infrastructure.

## Environment coordinates

One checked, typed environment descriptor supplies at least:

- release channel (`development`, `remoteAcceptance`, or `production`);
- API base origin;
- Auth0 issuer, audience, native client ID and callback/logout scheme;
- cloud schema/API compatibility floor; and
- telemetry/logging policy.

Production builds fail closed if loopback, development issuer/audience or
development callback coordinates are present. Development builds visibly identify
their channel in diagnostics without exposing secrets. Ad hoc user-editable server
URLs are not part of the normal product UI.

## Replaceable product identity

Before further user-facing identifiers spread, add one typed product-identity
boundary consumed by Swift, build tooling and host adapters. It owns:

- display/product name and short name;
- bundle identifier, Keychain service and native callback scheme;
- Project package display type and filename extension;
- API audience namespace, service hostname and user agent;
- application-support/cache directory names;
- default Library display name; and
- website, support, privacy and store URLs when they exist.

The identity boundary contains no secret. Semantic UUIDs, database relationships,
graph/port/node contracts, manifests and portable Rust DTOs must not derive their
identity from the visible brand. Database migration filenames and already-deployed
ledger checksums remain immutable. Development resources may retain codename labels
until the one coordinated pre-release identity cutover.

The cutover creates new production Auth0 and service resources rather than silently
relabeling development trust coordinates. Because no public compatibility baseline
exists yet, the Project extension and user-visible storage names may be replaced at
that gate. After the first public release they become compatibility-managed values.

## Bounded execution sequence

1. **CXT4b-dev:** freeze the typed environment/product-identity seam; start the API
   locally with host-only secrets; prove readiness against Neon without creating a
   user through an operator seed.
2. **CXT4d-dev:** connect the accepted opening shell to native Auth0 PKCE and the
   loopback API; store user tokens in Keychain and reconcile the local `My Library`.
3. **CXT4e-dev:** complete the first real sign-in and idempotency/restart/offline
   acceptance against Neon; repeat from a second configured development Mac and
   prove the same Account/Library is hydrated without duplication.
4. **Remote acceptance:** briefly deploy to Fly.io and repeat the service/security
   subset over HTTPS; record cost and destroy the Machine.
5. Continue Project creation and node vertical slices using the development cloud
   profile. Permanent Fly deployment and the final brand gate remain pre-production
   work, not blockers for daily development use.
