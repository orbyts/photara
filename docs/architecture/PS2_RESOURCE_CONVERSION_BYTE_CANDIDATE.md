# PS2 linked resource and conversion byte candidate

Status: **unfrozen pure-memory projection, 2026-09-27**. Twelve tests pass,
including 26 independently generated linked scenarios. This advances the
[field-level wire gate](PS2_PRODUCTION_CODEC_AND_PUBLICATION_CONTRACT.md#field-level-wire-completion-required-before-codec-code)
using the [appendix's additive resource and conversion shapes](PS2_PRODUCTION_WIRE_APPENDIX.md).
No production schema, codec, package writer, conversion, Asset Store or qualified
storage profile is selected or implemented.

The [generator](proposals/ps2/resource-conversion/generate.py) independently
assembles Python canonical bytes, lengths and SHA-256 values in the
[fixed corpus](proposals/ps2/resource-conversion/linked.json). The new
[Rust tests](../../crates/photara-store/tests/resource_conversion_candidate.rs)
compare those bytes with Photara canonical JSON and exercise a
[private typed verifier](../../crates/photara-store/tests/resource_conversion_candidate/wire.rs).
The old nineteen syntactic examples and synthetic resource tests remain unchanged.

## Resource closure and semantics

The valid corpus has sixteen exact records. Each independent active/recovery
projection selects nine dependency records totaling 6,574 bytes, excluding its
own projection envelope. Both select the same immutable CapturedVersion, capture
evidence, identity, portable working binding and v3 representation. They select
different backing revisions, logical locations, publication evidence and
root-bound obligations. Removing the other root's exclusive records does not
prevent verification of the retained root.

Only ResourceState's five typed selection arrays, captured-version's capture
reference, backing's publication reference and v3's version reference create
resource edges. CapturedVersion has no backing or retention-policy pointer.
Inventories exactly equal the resulting sorted unique dependency sets. Opaque
namespaced extension values remain byte-exact and cannot create dependencies,
even when they resemble missing ObjectRefs. Feature refusal precedes resource
lookup. Existing `ExternalCoordinate`, `RelativeComponents`, `MediaType`,
`QualifiedName` and checked timestamp types validate the relevant values.

The verifier checks selection IDs/schema/project/library; capture identity,
operation/time and version bytes; distinct capture versus retained-publication
variants; destination backing ID/revision/location; obligation version and
required backing selection; and a typed source bound to the selected projection's
root UUID. The v3 projection checks selected version, resource, media type/length
and fingerprint; the actual existing context ResourceValue decoder rejects the
new captured-managed descriptor. This does not change v2 meanings.

**Structural readability and retention support are separate results.** Valid
metadata requiring two copies with only one selected backing remains readable,
as do valid evidence records whose profile is insufficient for the obligation.
The fixture's stricter support admission refuses them. Missing required backing
selection, invalid typed pin source, zero-copy obligation or capture evidence
masquerading as retained publication are structural errors. The single-copy
profile is explicitly injected synthetic metadata, not an accepted profile or
proof of storage durability, independent replicas, availability or external
bytes. The 8 GB version length never causes media allocation, reading or hashing.

The 25 rehashed negative scenarios reach named semantic errors, including unknown
fields, v2/v3 confusion, duplicate selections/current bindings, wrong IDs,
version/media/fingerprint substitutions, bad pin/qualification/destination,
unsafe coordinates, missing capability and inventory surplus/omission. Additional
tests remove/corrupt every selected dependency, reject noncanonical/duplicate-key
JSON and distinguish readable-but-unsupported obligations.

## Exact original snapshot and registered resumption

The conversion specimen uses the existing real **fifteen-file** package from the
[operation corpus](proposals/ps2/operations/linked-operations.json), byte-for-byte.
Three unknown optional regular-file byte strings are added, including invalid
UTF-8 and content beneath names resembling staging/conversion namespaces. The
manifest binds all eighteen paths, lengths and hashes, original bootstrap and
HEAD bytes, original commit ID, original format coordinate and exact
`conversion-sources/<conversion_id>/package` namespace. Nothing is canonicalized
on copy.

MemoryPackage permits only internal package names. Therefore the unchanged
actual v1.1 reader verifies all fifteen original internal files; the complete
conversion manifest independently verifies every unknown optional byte. This
report does not claim the legacy MemoryPackage constructor accepts unknown
optional paths. The completed snapshot map retains all eighteen files.

At each of nineteen partial-copy positions, resumption requires immutable intent
and explicit fixture registration containing the original conversion ID,
canonical descriptor digest and snapshot namespace. Missing or changed
registration refuses even a byte-identical fully occupied namespace. Occupied
snapshot entries must equal the registered partial map and exact admitted source
bytes before a completed map can be returned. Unknown/conflicting occupancy,
changed source/manifest/ID or an unrelated staging/conversion file refuses with
input maps unchanged. Invalid portable components, including reserved device
names, controls and trailing dot/space, refuse rather than disappearing or being
rewritten.

Only the exact registered lock name, exact attempt staging name and registered
snapshot namespace are excluded from original enumeration. These are **injected
memory registration facts**. They prove no native inode identity, cooperative
lease, process ownership, no-replace write or durable barrier. Existing real-file
conversion/fault evidence remains separate; this test performs no native package
writes or media operations.

## Composition boundary and remaining review

`example.ps2.resource-closure-projection` is a harness envelope, not a second
HEAD, StateRoot or proposed production selector. `verify_selected_state` accepts
an already verified selected StateRoot's root ID and ResourceState reference,
returns the exact resource closure plus fixture support status, and does not
inject an unused representation into inventory. Its caller must verify the
required feature and derive the pin context from that actual selected root.

Joining this resource closure and ConversionSource to one selected candidate
HEAD/outer commit/RootSet/StateRoot alongside actual Graph operation, packed
placement, accounting and typed-tree evidence remains a separate composition
step. The standalone v3 binding is a fully linked projection test, not a claim
that a real Graph operation already selects it. Nonempty v3 lineage, all pin
origins/policy evidence, multi-copy qualification and full resource evolution
remain outside this bounded specimen. No unknown variant is silently accepted.

```sh
python3 docs/architecture/proposals/ps2/resource-conversion/generate.py
cargo test --offline -p photara-store --test resource_conversion_candidate
cargo clippy --offline -p photara-store --test resource_conversion_candidate -- -D warnings
rustfmt --edition 2024 --check crates/photara-store/tests/resource_conversion_candidate.rs crates/photara-store/tests/resource_conversion_candidate/wire.rs
```

The [evidence record](verification/ps2-resource-conversion-byte-candidate-20260927.json)
contains twelve-test output, strict Clippy, fixed input/source hashes and exact
regeneration verification. These results are engineering evidence for wire
review, not a permanent format freeze, production conversion or `Saved` guarantee.
