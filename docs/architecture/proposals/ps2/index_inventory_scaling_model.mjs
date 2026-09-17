import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';

// Disposable synthetic byte-work model. It is not a package codec or writer.
const sizes = [1_000, 10_000, 100_000, 1_000_000];
const fanout = 16;
const leafCapacity = 16;
const digest = 'a'.repeat(64);
const uuid = i => `00000000-0000-4000-8000-${i.toString(16).padStart(12, '0')}`;
const entry = i => JSON.stringify({
  acceptance_ordinal: String(i), operation_id: uuid(i), request_sha256: digest,
  receipt: { kind: 'json', sha256: digest, byte_length: '1024' }
});
const hash = bytes => createHash('sha256').update(bytes).digest('hex');

function flat(n) {
  const start = performance.now();
  const h = createHash('sha256');
  let bytes = 2;
  h.update('[');
  for (let i = 1; i <= n; i++) {
    const encoded = entry(i);
    if (i > 1) h.update(',');
    h.update(encoded);
    bytes += encoded.length + (i > 1 ? 1 : 0);
  }
  h.update(']');
  h.digest('hex');
  const encodeHashMs = performance.now() - start;

  // A retained in-memory ordinal array is the flat format's lookup shape.
  // Constructing it is excluded from scanMs; cold parse cost is separate.
  const ids = Array.from({ length: n }, (_, i) => uuid(i + 1));
  const target = ids[Math.floor(n / 2) - 1];
  const scanStart = performance.now();
  let comparisons = 0;
  for (const candidate of ids) {
    comparisons++;
    if (candidate === target) break;
  }
  const scanMs = performance.now() - scanStart;
  let coldParseHashMs = null;
  if (n <= 100_000) {
    const encoded = '[' + Array.from({ length: n }, (_, i) => entry(i + 1)).join(',') + ']';
    const coldStart = performance.now();
    const parsed = JSON.parse(encoded);
    hash(encoded);
    if (parsed.length !== n) throw new Error('fixture mismatch');
    coldParseHashMs = performance.now() - coldStart;
  }
  return { bytes, encodeHashMs, comparisons, scanMs, coldParseHashMs };
}

function height(n) {
  let pages = Math.ceil(n / leafCapacity);
  let levels = 1;
  while (pages > 1) { pages = Math.ceil(pages / fanout); levels++; }
  return levels;
}

function paged(n) {
  const levels = height(n);
  const leafObject = { schema: 'synthetic.leaf', entries: Array.from(
    { length: leafCapacity }, (_, i) => ({ key: uuid(i), receipt_sha256: digest })) };
  const internalObject = { schema: 'synthetic.branch', children: Array.from(
    { length: fanout }, (_, i) => ({ first_key: uuid(i), sha256: digest })) };
  const manifestObject = { schema: 'synthetic.manifest', ordinal_root: digest,
    id_root: digest, authored_root: digest, recovery_root: digest };
  const leaf = JSON.stringify(leafObject);
  const internal = JSON.stringify(internalObject);
  const manifest = JSON.stringify(manifestObject);
  const perMutationBytes = 2 * (leaf.length + (levels - 1) * internal.length) + manifest.length;
  const runs = 1_000;
  const start = performance.now();
  for (let i = 0; i < runs; i++) {
    for (let tree = 0; tree < 2; tree++) {
      hash(JSON.stringify(leafObject));
      for (let level = 1; level < levels; level++) hash(JSON.stringify(internalObject));
    }
    hash(JSON.stringify(manifestObject));
  }
  return { levels, pagesWrittenModel: 2 * levels + 1, perMutationBytesModel: perMutationBytes,
    encodeHashMs: (performance.now() - start) / runs,
    oldLookupPagesModel: levels,
    manifestComponentBytesModel: manifest.length,
    fourKiBAllocationFloorModel: (2 * levels + 1) * 4096 };
}

console.log(JSON.stringify({ kind: 'disposable synthetic model', fanout, leafCapacity,
  runtime: process.version, rows: sizes.map(n => ({ n, flat: flat(n), paged: paged(n) })) }, null, 2));
