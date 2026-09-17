# PS2 operation-index and root-inventory scaling review

Status: **architecture recommendation, not a permanent wire or production
implementation**. This review supersedes the *flat index/inventory scaling*
assumption in the [candidate wire appendix](PS2_PRODUCTION_WIRE_APPENDIX.md).
The appendix and its goldens remain unfrozen and unchanged. No reader, writer,
live package, migration, retirement, or storage profile was changed.

## Why both structures must change

The candidate `photara.package.operation-index` v1 serializes all accepted
entries 1…N in every replacement index. One accepted operation therefore
serializes/hashes O(N) index bytes; N individual publications do O(N²)
cumulative index work. Reusing the same index reference on a root-only
turnover does not solve a flat per-root inventory union or exhaustive closure
walk. Immutable old receipts remain necessary O(N) *total retained evidence*;
the requirement is O(log N) or bounded *foreground change/lookup work*
with respect to lifetime operation count N, for bounded-size mutations.
Current authored-state work still scales with the state that must be loaded or
changed.

The current [publication proposal](PS2_PRODUCTION_CODEC_AND_PUBLICATION_CONTRACT.md)
also says to validate the entire candidate before HEAD, reopen the full
closure afterward, and validate independent roots by traversing their flat
inventories. These timings cannot literally coexist with logarithmic root
turnover and historical opening. This is a validation-assurance decision, not
something a different index alone can fix.

## Alternatives assessed

| Structure | Append/checkpoint | Old-ID retry | Open/turnover and main cost |
| --- | --- | --- | --- |
| Flat lifetime index + inventory | O(N) rewrite/hash; simple | O(N) scan unless separate map | O(N) metadata; reject |
| Linked immutable segments | Bounded append | O(number of segments) without another index | Cheap tail, but moves lookup/validation problem |
| Segments with flat directory | Bounded segment append | Directory may help lookup | Directory and flat inventory still grow per turnover |
| LSM-style sorted runs | Good amortized writes | Bloom/index-assisted | Compaction spikes, reserve, recovery and read amplification complicate worst-case guarantees |
| Persistent paged B-tree pair | O(log N) path copy | O(log N) | Good sharing; canonical split/merge and page rewrite costs need testing |
| Compressed radix ID map + dense ordinal Merkle sequence | Bounded by ID width and O(log N) ordinal path | Bounded path plus receipt | No balancing; simpler dense-prefix proof, but sparse-node/physical-page amplification needs testing |

Recommend the **last pair as a logical architecture candidate**, not yet a
page codec choice: one immutable original receipt object per accepted operation;
a dense append-only ordinal Merkle sequence of receipt references; and a
compressed persistent map from the exact OperationId to ordinal, original
request digest and the same receipt reference. The map routes by a domain-
separated hash of OperationId, but leaves retain/compare the exact original ID
and reject an actual key collision. Both structures use bounded immutable
pages and share untouched subtrees. A shared page/validation engine is possible.
The exact fanout, compression, page encoding and object layout remain open.

Acceptance of N+1 verifies the original ID is absent by a fully authenticated
search path, the sequence append is exactly N+1, the two inserted references
name the same original receipt, and the resulting selected roots commit both
new paths. A same-ID/same-digest retry returns its original outcome; a different
digest refuses. A corrupt/missing page is never interpreted as absence. The
ordinal tree supports compact active/recovery prefix-consistency proof; the
current linear `Pi` cannot prove that relation without replaying a suffix and
must not be treated as a logarithmic proof. The bounded uncheckpointed journal
may continue to use its separate linear inclusion chain.

## Compositional inventory, not a second flat set

Each StateRoot should commit its typed semantic dependencies and the two index
roots. Tree pages recursively commit their children and the exact receipt
objects. RootSet retains active, recovery and explicit pinned roots by
reference, sharing unchanged pages. A typed dependency/manifest DAG gives
the **same exact union meaning** as today's flat inventories without
materializing that union on every publication. It excludes itself and parent
dispatch to avoid cycles; opaque optional extensions remain nonedges. An
explicit full audit can enumerate/deduplicate the unique closure and compare
it with typed edges. Root turnover touches new/changed paths and a bounded
number of selected top-level references, not all historical receipt leaves.
If explicit pins remain in a flat list, selecting them still costs O(P) for P
pins; a paged selector or a pin-count bound is needed before claiming otherwise.

This requires a separately reviewed change to the candidate RootSet/StateRoot
inventory fields and outer `inventory` target semantics. It does **not**
authorize changing the unchanged HEAD/bootstrap/outer-commit envelopes or
silently reusing old `photara.package.inventory` v1 flat semantics. Nor may a
content digest be mistaken for proof that an unread descendant currently
exists on disk.

## Three truthful integrity levels

1. **Structural selected-state open:** authenticate HEAD, commit, selected
   roots, feature/identity bindings, bounded top-level tree/manifest structure
   and the current authored state. This establishes the selected commitment
   and usable current state; untouched historical branches are reported *not
   exhaustively verified*. The current authored-state cost remains proportional
   to what must actually be loaded, not necessarily constant.
2. **Incremental publication and retry:** from an already trusted/audited or
   qualified published root, verify each changed/new page, old and new search
   paths, dense ordinal extension, receipt equality, range/count/height
   constraints and new dependency edges. The old root remains independently
   selected as recovery. Unknown outcomes retain original IDs and intent.
3. **Explicit full integrity audit:** traverse all selected index and manifest
   pages and receipts, check missing/corrupt nodes, dense ordinals, global
   OperationId uniqueness, map↔sequence bijection, prefix consistency and
   exact retained closure. This costs O(unique reachable metadata plus bytes
   verified), including authored/history dependencies and conversion-source
   objects; operation count N is only one component.

An arbitrary imported root cannot be declared globally history-verified by
reading only O(log N) pages. Local durable prior-audit evidence may accelerate
reopening only when bound to the exact selected root and qualified storage
identity/continuity; it is not a portable magic proof and cannot establish
that unread bytes have not decayed since the audit. The conservative
authoring policy is fast structural display, then a one-time background full
audit before permitting writes to an untrusted/imported root. Whether a
future portable attestation can safely replace that cost is a separate trust
decision. No design can detect corruption in an unread historical page during
logarithmic open; it is discovered on access or audit and must never become a
false successful lookup.

## Capacity, interruptions and long-lived projects

Admission reserves a safe upper bound for the receipt, both changed paths and
possible branch expansion, manifest/root/commit objects, journal, staging and
filesystem overhead **before** accepting work. Actual registered physical
allocations plus unresolved reservations are authoritative; summing child
byte totals double-counts shared DAG pages. No future-GC credit is spent.
On partial publication or unknown HEAD outcome, preserve the original intent,
receipt and reserve until reconciliation; do not reuse IDs or discard pages.
Active and recovery references share old pages without a copy.

Path copying also creates stale pages. Total accepted receipts and their index
information necessarily grow O(N). Obsolete page versions and crash orphans
cannot be allowed to accumulate without bound; an eventual registered,
incremental, pin-aware collection protocol is required for sustained high-rate
authoring. This review neither implements deletion nor selects retention or
production thresholds. The full physical write/space budget must include
allocation granularity, directory metadata, fsync barriers and batching—not
just encoded JSON length.

## Disposable scaling evidence

The reproducible [Node byte-work model](proposals/ps2/index_inventory_scaling_model.mjs)
uses synthetic ~290-byte flat entries, SHA-256 and canonical-sized JSON page
shapes. Five independent process runs were taken on the current development
Mac; table timings are medians. The paged proxy assumes **two** packed
16-entry-leaf, 16-way path-copy trees plus one small compositional manifest.
It is an illustrative paged-tree shape, **not** the recommended compressed-radix
implementation. Its leaves omit some proposed map fields, so encoded bytes
are not a complete candidate-format estimate. It measures encoding/hash CPU
for changed page shapes, **not** a working tree algorithm, disk write,
directory barrier, fsync, split worst case or qualified durability. The flat
scan uses a retained in-memory ID array; construction is excluded from scan
timing and cold parse is reported separately.

| Accepted operations | Flat replacement index bytes | Flat encode+hash | Flat middle-ID scan | Flat cold parse+hash | Paged proxy pages / encoded bytes per mutation |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 0.29 MB | 1.8 ms | 0.033 ms | 1.0 ms | 7 / 13.1 KB |
| 10,000 | 2.91 MB | 8.8 ms | 0.28 ms | 7.4 ms | 9 / 17.3 KB |
| 100,000 | 29.19 MB | 79.5 ms | 2.78 ms | 94.0 ms | 11 / 21.5 KB |
| 1,000,000 | 292.89 MB | 774.9 ms | 26.49 ms | not allocated | 11 / 21.5 KB |

At one million, the proxy's ID lookup is modeled as five pages, versus a
middle-ID flat scan of 500,000 comparisons. The model does **not** benchmark
structural cold-open page reads for the paged candidate. That requires an
explicit HEAD→commit→selected-root→manifest/index procedure in a real-file
prototype; current authored state may add separate work.

The following *logical-path* model covers the remaining requested actions;
these are not filesystem timings or verified bytes. A retry reads one map path
and the named receipt. A checkpoint that accepts one operation path-copies the
two indexes; a metadata-only root turnover shares both indexes without
reading their historical leaves. Structural open reads the dispatch and
selected root records plus the two index root headers, then only the authored
state needed for display. The fixed dispatch/header count depends on final
typed edges and has not been measured.

| Lifetime N | Retry map-path pages + receipt | Accepted-operation index pages changed | Metadata-only turnover historical pages changed | Structural open historical index headers |
| ---: | ---: | ---: | ---: | ---: |
| 1,000 | 3 + 1 | 6 | 0 | 2 |
| 10,000 | 4 + 1 | 8 | 0 | 2 |
| 100,000 | 5 + 1 | 10 | 0 | 2 |
| 1,000,000 | 5 + 1 | 10 | 0 | 2 |

The two-header open is a commitment/shape check, **not** proof that every
historical descendant still exists or is uncorrupted. A real cold-open test
must include selected current-state bytes and explicit pin-selection cost.
The proxy's 21.5 KB encoded path work is already about 74× its synthetic
290-byte flat entry; eleven separately allocated 4 KiB objects would have
at least a 44 KiB (45,056-byte) allocation floor before receipt, journal, root and
filesystem metadata. Thus lower CPU asymptotics alone do **not** prove an
acceptable production storage amplification. A compact real-file prototype
and batching/packing measurements are required before page sizes or permanent
wire bytes are chosen. Ten million operations would raise the flat index to
roughly 2.9 GB while the 16-way proxy grows by one path level; that is a
formula projection, not a physical benchmark.

The current large-media boundary is unchanged: ordinary open, autosave,
turnover and metadata validation read/hash **zero** large external-media bytes.
Exact capture and explicit strong verification remain separate operations.

## Before proposing new permanent bytes

- Disposable, real-file Rust prototype of at least the recommended radix/
  ordinal pair and one paged B-tree alternative, exercising multiple height
  transitions, adversarial IDs and batched high-rate operations.
- Measure p50/p95/max append, old/new lookup, checkpoint, turnover and cold
  structural open at 1k, 10k, 100k and 1M logical operations; report CPU,
  bytes encoded, actual bytes allocated/written, object count and stale-page
  growth separately. Use compact fixtures where possible.
- Fault cuts at each page/manifest/root/HEAD barrier, finite reserve and
  recovery under original IDs; negative pages for duplicate/conflicting IDs,
  ordinal gaps, malformed ranges, missing/corrupt nodes, prefix mismatch and
  cross-index disagreement.
- Explicit approval of the structural-open versus full-audit assurance policy,
  especially writable admission for imported or unaudited packages.

Only after those results should the flat operation-index/inventory appendix
and golden vectors be replaced with permanent page/manifest wire bytes. No
production reader, writer, live conversion, GC or project switching is
authorized by this review.
