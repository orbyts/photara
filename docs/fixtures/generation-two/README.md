# Inert generation-two fixtures

**Library nomenclature rebaseline — 2026-09-12:** the user explicitly authorized
rewriting the unshipped Generation Two fixtures and schema. Original S6/D18 examples
now use Library naming; package references, commits, sealed-request bytes and
checksums were regenerated with the Rust canonical encoder. See the
[rebaseline record](../../architecture/LIBRARY_NOMENCLATURE_REBASELINE.md).
These examples do not make historical `$project.assets` scenarios valid D19 runtime
inputs; immutable AssetSet and CXT1 context contracts govern current evaluation.

Prepared for [S6 conformance](../../architecture/GENERATION_TWO_FIXTURES.md) and
[S7 review](../../architecture/SCHEMA_REVIEW.md). Synthetic documentation inputs
only; do not run against real accounts, databases, storage roots or providers.

- [context-amendment.json](context-amendment.json): separately reviewable D18
  expression/backtick/fence, scoped variable, metadata, security and CAS scenarios
  plus ASCII-only canonical byte/hash vectors. Not an implemented interpreter or
  additions to the 51-case S6 baseline. See [D18](../../architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md).

- [records.json](records.json): typed Library/identity and separately local device data.
- [package-specimen.json](package-specimen.json): exact file-byte archive of a new
  two-commit `.photara` package; not an active directory package.
- [canonical-vectors.json](canonical-vectors.json): actual retained Rust codec
  bytes/hex/hashes and pending invalid-input validator expectations.
- [sync-trace.json](sync-trace.json): exact sealed requests and deterministic
  offline/accept/feed/conflict/rebase expectations, not live server responses.
- [normalization-and-limits.json](normalization-and-limits.json): proposed pinned
  policy and bounded validation inputs, awaiting S7 approval.
- [social-export.json](social-export.json): D16/D17 manual/multiple social profiles,
  subject collision, chosen display snapshot and inert checksummed logical export.
- [scenarios.json](scenarios.json): 51 independently reset scenario specifications
  and 12 crash boundaries; runtime results are explicitly not run.

Outer JSON files use the Rust canonical encoding. Embedded `utf8`, `body_utf8` and
`canonical_utf8` fields define exact bytes after JSON string decoding. Do not
append a newline to those bytes. Hash fields cover decoded bytes, not escaped
container text. All locators/domains/identities are synthetic; no credentials or
bookmark bytes exist. Legacy/v0.1.3 import is excluded from acceptance.
