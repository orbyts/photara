# Node evaluation and cache model

## Principle

Caching must reuse Photara's existing evidence model. It is not a second truth
store and it never makes an authored transform, asset registration, provider
receipt, or publication record disposable.

```text
authoritative state -> canonical semantic values -> evaluation key
                                                -> derived artifacts/cache
```

## Current foundations

Photara already has the ingredients:

- SHA-256 and byte size for asset files;
- one-current-authority constraints and removed history;
- post source SHA and canonical authoring-input SHA;
- immutable template/reference SHA;
- master batch and flattening manifest/report identity;
- Adobe inventory snapshot SHA;
- transfer manifest SHA and idempotent batch reuse;
- Cloudinary delivery manifest SHA and per-object source SHA;
- publication evidence bound to source-specification SHA;
- host reports bound to batch/session and source fingerprints.

The new cache should standardize these as typed input digests and evaluation
receipts, not replace the existing ledger.

## Evaluation key

```text
evaluation_key = SHA-256(canonical {
  node_type_id,
  node_type_version,
  implementation_digest,
  config_schema_version,
  canonical_config,
  authored_state_schema_version,
  authored_state_digest,
  ordered_input_value_digests,
  relevant_provider_snapshot_digests,
  execution_environment_fingerprint
})
```

Only relevant environmental facts belong in the key. A Photoshop node needs
materializer protocol/plugin version and host capabilities; a pure Layout
resolver does not need the macOS version. A Disk node depends on current
representation identities/fingerprints, not the absolute mount point.

## Dirty and stale semantics

- **Dirty**: the instance's desired evaluation key differs from its last
  successful evaluation key or it has never succeeded.
- **Stale output**: an output still exists but one or more declared inputs,
  provider snapshots, implementations, or environment requirements changed.
- **Unavailable**: identity/fingerprint is unchanged but the provider or local
  locator cannot currently supply bytes; this can block execution without
  invalidating semantic output.
- **Corrupt**: an artifact's observed fingerprint differs from its receipt.
- **Needs authoring**: authored state is incomplete; this is not cache miss.

The GUI should distinguish “NAS unmounted” from “source changed.” v0.1 often
discovers both during filesystem verification; the node model can report them
separately.

## Authority and disposal

| Data | Class | Disposal rule |
| --- | --- | --- |
| Graph document and Layout authored state | authoritative | Never evict as cache. Version/migrate explicitly. |
| Neon identity, master, provider, publication evidence | authoritative | Existing retention/state rules. |
| Original, layered master, registered flattened pair | authoritative representation | Never cache-evict. |
| `AssetSet`, resolved geometry, validation reports | derived value | Recompute from authoritative inputs. |
| Thumbnail and authoring preview | disposable proxy | Evict by size/age; regenerate from rendition provider. |
| Photoshop render manifest | transport/cacheable | Recreate from resolved plan. Retain when referenced by receipt if useful. |
| Generated layout PSB | derived artifact, durable | May be reproducible but do not auto-delete; verify/reuse by receipt. |
| WSP JPEG / delivery object | delivery artifact/evidence | Governed by delivery lifecycle, not general cache eviction. |
| Host execution receipt | durable evidence | Retain with output/artifact history. |

## Source propagation

If a flattened HDR or SDR registration changes, the Disk node emits a new
`AssetSet` digest. Every Layout instance consuming it becomes dirty, but its
authored crop remains intact. Resolution validates whether the normalized crop
still applies to the new dimensions. Only affected placements and their
downstream Photoshop artifacts need new evaluation.

Changing one Layout instance crop changes that instance's authored-state
digest and its downstream Photoshop evaluation. A sibling 9:16 instance stays
clean because it has a different node instance and authored-state digest.

Changing item order changes Layout output digest and any downstream artifact
ordering/naming, but should not invalidate unchanged per-item composition
subresults if the Photoshop materializer supports item-level keys.

## Granularity

Use two levels:

1. **Node evaluation** for graph status and whole output value identity.
2. **Sub-artifact keys** for expensive per-asset preview and per-layout-item
   materialization.

Example item key:

```text
SHA-256(template digest + canvas digest + ordered placement bindings +
        source rendition digests + transforms + materializer version)
```

This allows reordering a package without rebuilding every PSB, while the final
ordered `ArtifactSet` still gets a new digest.

## Visual proxy cache

`VisualProxyService` accepts a semantic rendition ref and request:

```rust
struct ProxyRequest {
    kind: Thumbnail | AuthoringPreview,
    max_pixel_edge: u32,
    color_intent: PreviewColorIntent,
    format_preferences: Vec<ProxyFormat>,
}
```

Proxy key includes source file ID/SHA, request, decoder implementation version,
and color-transform version. Store metadata beside the proxy in an application
cache root; never in authoritative project masters. A local flattened-TIFF
provider may decode or extract a useful preview. Later Lightroom/remote
providers may return their own proxy and provenance under the same interface.

For the first milestone, generate a small grid proxy and a larger authoring
proxy on demand, deduplicate concurrent requests, and cap disk usage. Preserve
wide-gamut/HDR metadata when supported, but always provide a tested SDR display
fallback. Exact preview codec and HDR display path remain an implementation
decision after a spike.

## Host and plugin versions

Photoshop output validity should include:

- Photoshop node definition version;
- materialization protocol version;
- PSJS/UXP plugin implementation digest and declared capability version;
- relevant Photoshop major/minor version when behavior differs;
- template/reference digest;
- execution target/device identity only when artifacts depend on it.

A plugin update does not automatically declare all old files corrupt. It makes
them stale relative to a requested re-evaluation. Existing verified artifacts
and receipts remain historical evidence.

## Provider state

Cloud/API nodes consume explicit provider snapshots with observed time,
freshness policy, and digest. A cache hit is valid only if the node's policy
accepts that snapshot age. Human confirmation is a durable evidence input, not
an indefinitely fresh provider inventory.

The initial three-node graph requires only local host status. Do not add Adobe
or Cloudinary state to Layout keys.

## Persistence sketch

Do not add schema during this study. A future minimal store needs:

- node evaluation ID, node instance ID, evaluation key, state, timestamps;
- input and output value type/digest references;
- diagnostics and implementation/environment fingerprints;
- artifact/receipt references;
- cache-access metadata separate from authoritative graph revisions.

Blob/proxy bytes belong in a content-addressed local cache; metadata may begin
in SQLite for the desktop client or in Core's configured state store. Whether
graph authority belongs in Neon or a local-first document is unresolved and
must be decided before implementation.
