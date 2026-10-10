//! Opt-in immutable capture benchmark; never opens a native package for writing.
use super::*;

#[test]
#[ignore = "Requires an explicit read-only copy of existing registered package bytes"]
fn captured_original_compiler_exact_output() {
    let path = std::path::PathBuf::from(std::env::var("PHOTARA_COMPILER_CAPTURE").unwrap());
    let registration: Value =
        serde_json::from_slice(&std::fs::read(path.join("repeatable-registration.json")).unwrap())
            .unwrap();
    let mut base = support::Fixture::from_value(registration["data"].clone());
    for (id, a) in &mut base.allocations {
        if let Some(charge) = registration["tip_charges"][id.to_string()].as_str() {
            a.charge = charge.parse().unwrap();
        }
    }
    let mut current = package(&base);
    let read = |name: &str| std::fs::read(path.join("package").join(name)).unwrap();
    current.head = serde_json::from_slice(&read("HEAD.json")).unwrap();
    current.commit = serde_json::from_slice(&read(&format!(
        "{}.commit",
        current.head["commit_sha256"].as_str().unwrap()
    )))
    .unwrap();
    let overlay = current.commit["root_set"]["inventory"]["sha256"]
        .as_str()
        .unwrap();
    let overlay_bytes = read(&format!("{overlay}.control"));
    let body: Value = serde_json::from_slice(&overlay_bytes).unwrap();
    current.loose.clear();
    current.loose.insert(overlay.into(), overlay_bytes);
    for r in body["controls"].as_array().unwrap() {
        let sha = r["sha256"].as_str().unwrap();
        current
            .loose
            .insert(sha.into(), read(&format!("{sha}.control")));
    }
    for (id, a) in &mut current.allocations {
        a.bytes = read(&format!("pack-{id}"));
    }
    let mut observed = snapshot(&base, &current);
    for (id, a) in &mut observed.allocations {
        a.charge = base.allocations[id].charge.max(a.charge);
    }
    let identity = identity(&current);
    let reader = support::limits();
    let context = RestartContext {
        inspection: &observed,
        registration: &base.registration,
        directory: v1_3::DirectoryObservation {
            device: registration["package_pin"][0].as_u64().unwrap(),
            inode: registration["package_pin"][1].as_u64().unwrap(),
        },
        identity: &identity,
        frames: frames(),
        reader: &reader,
        planner: budget(),
        max_originals: 256,
    };
    let bytes = std::fs::read(path.join("checkpoint.phpsj")).unwrap();
    let j = journal::Journal::read(
        &bytes,
        &registration["journal_header"],
        journal::JournalLimits {
            json: reader.json,
            max_file_bytes: 4 << 20,
            max_records: 4096,
            max_work_bytes: 4 << 20,
        },
    )
    .unwrap();
    let record = j
        .records()
        .iter()
        .rev()
        .find(|r| r.value()["kind"] == "CheckpointIntent")
        .unwrap();
    let body = &record.value()["body"];
    let old = recovery::reconstruct_original(&current, &body["original"], budget()).unwrap();
    let verified = restart::verify_original_prefix(old, &context).unwrap();
    layout::take_load_work();
    let before = std::time::Instant::now();
    let plan = recovery::recover(
        &verified,
        &body["original"],
        &body["semantic_intent"],
        &body["operation_receipt"],
        budget(),
    )
    .unwrap();
    let elapsed = before.elapsed();
    let work = layout::take_load_work();
    assert_eq!(work.0, 1);
    eprintln!("compiler load work={work:?}");
    for (id, bytes) in &plan.files {
        assert_eq!(bytes, &current.allocations[id].bytes);
    }
    assert_eq!(plan.stages[6].head, current.head);
    eprintln!(
        "captured compiler: elapsed={elapsed:?} vector={} bytes={}",
        vector(&plan),
        plan.files.values().map(Vec::len).sum::<usize>()
    );
}
