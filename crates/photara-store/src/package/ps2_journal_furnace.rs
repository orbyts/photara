//! PS2 test-only journal framing furnace. No caller can acknowledge an edit or
//! open a device journal through this module. Semantic replay and durable I/O
//! remain separate gates.
use photara_core::canonical_json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};
use uuid::Uuid;

const MAGIC: &[u8; 8] = b"PHPSJ001";
const MAX_PAYLOAD: usize = 16 * 1024 * 1024;
const CHECKSUM_SIZE: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    format_version: u32,
    journal_id: Uuid,
    device_id: Uuid,
    project_id: Uuid,
    library_id: Uuid,
    incarnation_id: Uuid,
    manifest_sha256: String,
    base_head_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    format_version: u32,
    sequence: u64,
    #[serde(rename = "record_id")]
    id: Uuid,
    session_generation: Uuid,
    project_id: Uuid,
    incarnation_id: Uuid,
    kind: String,
    body: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Tail {
    Complete,
    Incomplete,
    Corrupt,
    Unsupported,
}

#[derive(Debug)]
struct Scan {
    records: Vec<Record>,
    verified_len: usize,
    tail: Tail,
    last_checksum: [u8; CHECKSUM_SIZE],
}

#[derive(Debug, Eq, PartialEq)]
enum HeaderError {
    Incomplete,
    Invalid,
    Unsupported,
}

fn checksum(parts: &[&[u8]]) -> [u8; CHECKSUM_SIZE] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>, HeaderError> {
    canonical_json(value).map_err(|_| HeaderError::Invalid)
}

fn exact_json<T: for<'a> Deserialize<'a> + Serialize>(bytes: &[u8]) -> Option<T> {
    let value: T = serde_json::from_slice(bytes).ok()?;
    (canonical_json(&value).ok()?.as_slice() == bytes).then_some(value)
}

fn valid_header(header: &Header) -> bool {
    header.format_version == 1
        && [
            header.journal_id,
            header.device_id,
            header.project_id,
            header.library_id,
            header.incarnation_id,
        ]
        .iter()
        .all(|id| !id.is_nil())
        && [
            header.manifest_sha256.as_str(),
            header.base_head_sha256.as_str(),
        ]
        .iter()
        .all(|s| super::Sha256Hex::parse(s).is_ok())
}

fn header_bytes(header: &Header) -> Result<Vec<u8>, HeaderError> {
    if !valid_header(header) {
        return Err(HeaderError::Invalid);
    }
    let payload = canonical(header)?;
    let length = u32::try_from(payload.len()).map_err(|_| HeaderError::Invalid)?;
    let mut bytes = Vec::with_capacity(MAGIC.len() + 4 + payload.len() + CHECKSUM_SIZE);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&payload);
    let digest = checksum(&[&bytes]);
    bytes.extend_from_slice(&digest);
    Ok(bytes)
}

fn read_header(bytes: &[u8]) -> Result<(Header, usize, [u8; CHECKSUM_SIZE]), HeaderError> {
    if bytes.len() < MAGIC.len() + 4 {
        return Err(HeaderError::Incomplete);
    }
    if &bytes[..MAGIC.len()] != MAGIC {
        return Err(HeaderError::Invalid);
    }
    let length = u32::from_le_bytes(bytes[8..12].try_into().expect("four bytes")) as usize;
    // Header is deliberately much smaller than a mutation frame.
    if length > 4096 {
        return Err(HeaderError::Invalid);
    }
    let end = 12usize.checked_add(length).ok_or(HeaderError::Invalid)?;
    let total = end.checked_add(CHECKSUM_SIZE).ok_or(HeaderError::Invalid)?;
    if bytes.len() < total {
        return Err(HeaderError::Incomplete);
    }
    let expected = checksum(&[&bytes[..end]]);
    if bytes[end..total] != expected {
        return Err(HeaderError::Invalid);
    }
    let header: Header = exact_json(&bytes[12..end]).ok_or(HeaderError::Invalid)?;
    if header.format_version != 1 {
        return Err(HeaderError::Unsupported);
    }
    if !valid_header(&header) {
        return Err(HeaderError::Invalid);
    }
    Ok((header, total, expected))
}

fn append_frame(
    bytes: &mut Vec<u8>,
    previous: [u8; CHECKSUM_SIZE],
    record: &Record,
) -> Result<[u8; CHECKSUM_SIZE], HeaderError> {
    if record.format_version != 1 || record.id.is_nil() {
        return Err(HeaderError::Invalid);
    }
    let payload = canonical(record)?;
    if payload.len() > MAX_PAYLOAD {
        return Err(HeaderError::Invalid);
    }
    let length = u32::try_from(payload.len())
        .map_err(|_| HeaderError::Invalid)?
        .to_le_bytes();
    let digest = checksum(&[&previous, &length, &payload]);
    bytes.extend_from_slice(&length);
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(&digest);
    Ok(digest)
}

fn scan(bytes: &[u8]) -> Result<Scan, HeaderError> {
    let (header, mut offset, mut previous) = read_header(bytes)?;
    let mut records = Vec::new();
    let mut record_ids = BTreeSet::new();
    loop {
        let verified_len = offset;
        if offset == bytes.len() {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Complete,
                last_checksum: previous,
            });
        }
        if bytes.len() - offset < 4 {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Incomplete,
                last_checksum: previous,
            });
        }
        let length_bytes: [u8; 4] = bytes[offset..offset + 4].try_into().expect("four bytes");
        let length = u32::from_le_bytes(length_bytes) as usize;
        if length > MAX_PAYLOAD {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Corrupt,
                last_checksum: previous,
            });
        }
        let Some(end) = offset.checked_add(4).and_then(|n| n.checked_add(length)) else {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Corrupt,
                last_checksum: previous,
            });
        };
        let Some(total) = end.checked_add(CHECKSUM_SIZE) else {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Corrupt,
                last_checksum: previous,
            });
        };
        if total > bytes.len() {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Incomplete,
                last_checksum: previous,
            });
        }
        let expected = checksum(&[&previous, &length_bytes, &bytes[offset + 4..end]]);
        if bytes[end..total] != expected {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Corrupt,
                last_checksum: previous,
            });
        }
        let Some(record): Option<Record> = exact_json(&bytes[offset + 4..end]) else {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Corrupt,
                last_checksum: previous,
            });
        };
        if record.format_version != 1 {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Unsupported,
                last_checksum: previous,
            });
        }
        if record.sequence != records.len() as u64 + 1
            || record.id.is_nil()
            || !record_ids.insert(record.id)
            || record.session_generation.is_nil()
            || record.project_id != header.project_id
            || record.incarnation_id != header.incarnation_id
        {
            return Ok(Scan {
                records,
                verified_len,
                tail: Tail::Corrupt,
                last_checksum: previous,
            });
        }
        records.push(record);
        previous = expected;
        offset = total;
    }
}

// Recovery must pin the journal to the package identity before it considers
// any otherwise checksum-valid frame. A copied journal is not a recovery log
// for a different package or a replacement at the same locator.
fn scan_bound(bytes: &[u8], expected: &Header) -> Result<Scan, HeaderError> {
    let (actual, _, _) = read_header(bytes)?;
    if &actual != expected {
        return Err(HeaderError::Invalid);
    }
    scan(bytes)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppendFault {
    None,
    BeforeWrite,
    ShortWrite,
    AfterWrite,
    AfterSync,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppendOutcome {
    Acknowledged,
    Existing,
    NotPerformed,
    OutcomeUnknown,
    Frozen,
    Conflict,
}

fn mutation_evidence(record: &Record) -> Option<(Uuid, String)> {
    if record.kind != "Mutation" {
        return None;
    }
    let operation_id = Uuid::parse_str(record.body.get("operation_id")?.as_str()?).ok()?;
    if operation_id.is_nil() {
        return None;
    }
    let request_digest = record.body.get("request_digest")?.as_str()?;
    super::Sha256Hex::parse(request_digest).ok()?;
    Some((operation_id, request_digest.to_owned()))
}

fn mutation_index(scan: &Scan) -> Option<BTreeMap<Uuid, String>> {
    if scan.tail != Tail::Complete {
        return None;
    }
    let mut index = BTreeMap::new();
    for record in &scan.records {
        if record.kind == "Mutation" {
            let (id, digest) = mutation_evidence(record)?;
            if index.insert(id, digest).is_some() {
                return None;
            }
        }
    }
    Some(index)
}

// Test-only storage adapter. `sync_all` and a successful reopen are NOT a claim of
// qualified APFS full-sync/power-loss safety. Unknown results always require scan.
fn append_disposable(
    file: &mut File,
    expected: &Header,
    record: &Record,
    fault: AppendFault,
) -> AppendOutcome {
    if file.seek(SeekFrom::Start(0)).is_err() {
        return AppendOutcome::Frozen;
    }
    let mut old = Vec::new();
    if file.read_to_end(&mut old).is_err() {
        return AppendOutcome::Frozen;
    }
    let Ok(previous_scan) = scan_bound(&old, expected) else {
        return AppendOutcome::Frozen;
    };
    let Some(index) = mutation_index(&previous_scan) else {
        return AppendOutcome::Frozen;
    };
    let Some((id, digest)) = mutation_evidence(record) else {
        return AppendOutcome::Frozen;
    };
    if let Some(existing) = index.get(&id) {
        if existing != &digest {
            return AppendOutcome::Conflict;
        }
        // A byte-perfect frame observed after an unknown prior write is not
        // itself a durability receipt. Retry the barrier before deduping it.
        return if file.sync_all().is_ok() {
            AppendOutcome::Existing
        } else {
            AppendOutcome::OutcomeUnknown
        };
    }
    if record.sequence != previous_scan.records.len() as u64 + 1 {
        return AppendOutcome::Conflict;
    }
    if fault == AppendFault::BeforeWrite {
        return AppendOutcome::NotPerformed;
    }
    let mut frame = Vec::new();
    if append_frame(&mut frame, previous_scan.last_checksum, record).is_err()
        || file.seek(SeekFrom::End(0)).is_err()
    {
        return AppendOutcome::NotPerformed;
    }
    let written = if fault == AppendFault::ShortWrite {
        &frame[..frame.len() / 2]
    } else {
        &frame
    };
    if file.write_all(written).is_err() {
        return AppendOutcome::OutcomeUnknown;
    }
    if matches!(fault, AppendFault::ShortWrite | AppendFault::AfterWrite) {
        return AppendOutcome::OutcomeUnknown;
    }
    if file.sync_all().is_err() || fault == AppendFault::AfterSync {
        return AppendOutcome::OutcomeUnknown;
    }
    // Even this test adapter does not acknowledge solely from write/sync success.
    let mut after = Vec::new();
    if file.seek(SeekFrom::Start(0)).is_err() || file.read_to_end(&mut after).is_err() {
        return AppendOutcome::OutcomeUnknown;
    }
    let Ok(after_scan) = scan_bound(&after, expected) else {
        return AppendOutcome::OutcomeUnknown;
    };
    if after_scan.tail != Tail::Complete || after_scan.records.last() != Some(record) {
        return AppendOutcome::OutcomeUnknown;
    }
    AppendOutcome::Acknowledged
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs::OpenOptions;

    fn fixture() -> (Header, Vec<Record>) {
        let id = Uuid::parse_str("7d3dc89a-93d6-4d7c-a3d8-c60c9d2d5971").unwrap();
        let header = Header {
            format_version: 1,
            journal_id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            project_id: id,
            library_id: Uuid::new_v4(),
            incarnation_id: Uuid::new_v4(),
            manifest_sha256: "a".repeat(64),
            base_head_sha256: "b".repeat(64),
        };
        let records = (1..=2)
            .map(|sequence| Record {
                format_version: 1,
                sequence,
                id: Uuid::new_v4(),
                session_generation: Uuid::new_v4(),
                project_id: id,
                incarnation_id: header.incarnation_id,
                kind: "Mutation".to_owned(),
                body: json!({
                    "operation_id": Uuid::new_v4().to_string(),
                    "request_digest": "c".repeat(64)
                }),
            })
            .collect();
        (header, records)
    }
    fn encoded(header: &Header, records: &[Record]) -> Vec<u8> {
        let mut bytes = header_bytes(header).unwrap();
        let (_, _, mut previous) = read_header(&bytes).unwrap();
        for record in records {
            previous = append_frame(&mut bytes, previous, record).unwrap();
        }
        bytes
    }

    #[test]
    fn complete_chain_has_exact_sequence_and_checksum() {
        let (header, records) = fixture();
        let bytes = encoded(&header, &records);
        let result = scan(&bytes).unwrap();
        assert_eq!(result.records, records);
        assert_eq!(result.verified_len, bytes.len());
        assert_eq!(result.tail, Tail::Complete);
        assert_ne!(result.last_checksum, [0; CHECKSUM_SIZE]);
    }

    #[test]
    fn every_torn_suffix_preserves_only_the_verified_prefix() {
        let (header, records) = fixture();
        let bytes = encoded(&header, &records);
        let header_end = header_bytes(&header).unwrap().len();
        let one_end = encoded(&header, &records[..1]).len();
        for cut in header_end..bytes.len() {
            let result = scan(&bytes[..cut]).unwrap();
            assert_eq!(
                result.tail,
                if cut == header_end || cut == one_end {
                    Tail::Complete
                } else {
                    Tail::Incomplete
                },
                "cut={cut}"
            );
            assert_eq!(
                result.records.len(),
                usize::from(cut >= one_end),
                "cut={cut}"
            );
            assert_eq!(
                result.verified_len,
                if cut >= one_end { one_end } else { header_end }
            );
        }
    }

    #[test]
    fn corruption_is_not_misreported_as_a_torn_write() {
        let (header, records) = fixture();
        let mut bytes = encoded(&header, &records);
        let one_end = encoded(&header, &records[..1]).len();
        bytes[one_end + 15] ^= 1;
        let result = scan(&bytes).unwrap();
        assert_eq!(result.tail, Tail::Corrupt);
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.verified_len, one_end);
    }

    #[test]
    fn sequence_gap_and_oversized_frame_refuse_replay() {
        let (header, mut records) = fixture();
        records[1].sequence = 3;
        assert_eq!(
            scan(&encoded(&header, &records)).unwrap().tail,
            Tail::Corrupt
        );
        let mut bytes = header_bytes(&header).unwrap();
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(scan(&bytes).unwrap().tail, Tail::Corrupt);
    }

    #[test]
    fn duplicate_record_id_refuses_replay_even_with_valid_checksums() {
        let (header, mut records) = fixture();
        records[1].id = records[0].id;
        let first_end = encoded(&header, &records[..1]).len();
        let result = scan(&encoded(&header, &records)).unwrap();
        assert_eq!(result.tail, Tail::Corrupt);
        assert_eq!(result.records, records[..1]);
        assert_eq!(result.verified_len, first_end);
    }

    #[test]
    fn copied_journal_never_replays_into_another_package_incarnation() {
        let (header, records) = fixture();
        let bytes = encoded(&header, &records);
        assert_eq!(scan_bound(&bytes, &header).unwrap().records, records);
        for changed in [
            Header {
                project_id: Uuid::new_v4(),
                ..header.clone()
            },
            Header {
                library_id: Uuid::new_v4(),
                ..header.clone()
            },
            Header {
                incarnation_id: Uuid::new_v4(),
                ..header.clone()
            },
            Header {
                device_id: Uuid::new_v4(),
                ..header.clone()
            },
            Header {
                journal_id: Uuid::new_v4(),
                ..header.clone()
            },
            Header {
                manifest_sha256: "d".repeat(64),
                ..header.clone()
            },
            Header {
                base_head_sha256: "e".repeat(64),
                ..header.clone()
            },
        ] {
            assert_eq!(
                scan_bound(&bytes, &changed).unwrap_err(),
                HeaderError::Invalid
            );
        }
        assert_eq!(scan(&bytes).unwrap().tail, Tail::Complete);
    }

    #[test]
    fn disposable_append_refuses_wrong_binding_without_changing_file() {
        let (header, records) = fixture();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("journal.fixture");
        let evidence = header_bytes(&header).unwrap();
        std::fs::write(&path, &evidence).unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let wrong = Header {
            incarnation_id: Uuid::new_v4(),
            ..header.clone()
        };
        assert_eq!(
            append_disposable(&mut file, &wrong, &records[0], AppendFault::None),
            AppendOutcome::Frozen
        );
        assert_eq!(std::fs::read(&path).unwrap(), evidence);
        assert_eq!(
            append_disposable(&mut file, &header, &records[0], AppendFault::None),
            AppendOutcome::Acknowledged
        );
    }

    #[test]
    fn malformed_header_and_future_frame_version_refuse_replay() {
        let (header, mut records) = fixture();
        let mut bytes = header_bytes(&header).unwrap();
        bytes[20] ^= 1;
        assert_eq!(scan(&bytes).unwrap_err(), HeaderError::Invalid);
        records[0].format_version = 2;
        let mut bytes = header_bytes(&header).unwrap();
        let (_, _, previous) = read_header(&bytes).unwrap();
        // Build a checksum-valid future frame to prove version refusal.
        let payload = canonical(&records[0]).unwrap();
        let length = u32::try_from(payload.len()).unwrap().to_le_bytes();
        let digest = checksum(&[&previous, &length, &payload]);
        bytes.extend_from_slice(&length);
        bytes.extend_from_slice(&payload);
        bytes.extend_from_slice(&digest);
        assert_eq!(scan(&bytes).unwrap().tail, Tail::Unsupported);
    }

    #[test]
    fn disposable_file_reopen_and_torn_tail_keep_original_bytes() {
        let (header, records) = fixture();
        let complete = encoded(&header, &records);
        let first_end = encoded(&header, &records[..1]).len();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("journal.fixture");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        file.write_all(&complete[..first_end + 3]).unwrap();
        file.sync_all().unwrap();
        File::open(root.path()).unwrap().sync_all().unwrap();
        drop(file);
        let mut reopened = Vec::new();
        File::open(&path)
            .unwrap()
            .read_to_end(&mut reopened)
            .unwrap();
        let result = scan(&reopened).unwrap();
        assert_eq!(result.tail, Tail::Incomplete);
        assert_eq!(result.verified_len, first_end);
        assert_eq!(result.records, records[..1]);
        // A recovery scan never truncates the original evidence.
        assert_eq!(reopened, complete[..first_end + 3]);
        assert_eq!(std::fs::read(&path).unwrap(), reopened);
    }

    #[test]
    fn uncertain_append_reconciles_same_operation_without_duplicate() {
        let (header, records) = fixture();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("journal.fixture");
        std::fs::write(&path, header_bytes(&header).unwrap()).unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            append_disposable(&mut file, &header, &records[0], AppendFault::AfterWrite),
            AppendOutcome::OutcomeUnknown
        );
        drop(file);
        let before_retry = std::fs::read(&path).unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            append_disposable(&mut file, &header, &records[0], AppendFault::None),
            AppendOutcome::Existing
        );
        assert_eq!(std::fs::read(&path).unwrap(), before_retry);
        let mut altered = records[0].clone();
        altered.body["request_digest"] = json!("d".repeat(64));
        assert_eq!(
            append_disposable(&mut file, &header, &altered, AppendFault::None),
            AppendOutcome::Conflict
        );
        assert_eq!(
            append_disposable(&mut file, &header, &records[1], AppendFault::None),
            AppendOutcome::Acknowledged
        );
        assert_eq!(
            scan(&std::fs::read(&path).unwrap()).unwrap().records,
            records
        );
    }

    #[test]
    fn short_write_freezes_later_mutations_and_preserves_evidence() {
        let (header, records) = fixture();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("journal.fixture");
        std::fs::write(&path, header_bytes(&header).unwrap()).unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            append_disposable(&mut file, &header, &records[0], AppendFault::BeforeWrite),
            AppendOutcome::NotPerformed
        );
        assert_eq!(
            append_disposable(&mut file, &header, &records[0], AppendFault::ShortWrite),
            AppendOutcome::OutcomeUnknown
        );
        let evidence = std::fs::read(&path).unwrap();
        assert_eq!(scan(&evidence).unwrap().tail, Tail::Incomplete);
        assert_eq!(
            append_disposable(&mut file, &header, &records[1], AppendFault::None),
            AppendOutcome::Frozen
        );
        assert_eq!(std::fs::read(&path).unwrap(), evidence);
    }

    #[test]
    fn after_sync_unknown_reconciles_without_new_frame() {
        let (header, records) = fixture();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("journal.fixture");
        std::fs::write(&path, header_bytes(&header).unwrap()).unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            append_disposable(&mut file, &header, &records[0], AppendFault::AfterSync),
            AppendOutcome::OutcomeUnknown
        );
        let evidence = std::fs::read(&path).unwrap();
        assert_eq!(
            append_disposable(&mut file, &header, &records[0], AppendFault::None),
            AppendOutcome::Existing
        );
        assert_eq!(std::fs::read(&path).unwrap(), evidence);
    }
}
