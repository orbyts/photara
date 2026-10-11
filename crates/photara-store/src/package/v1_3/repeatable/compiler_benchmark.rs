//! Opt-in immutable capture benchmark; never opens a native package for writing.
use super::*;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Explicit read-only captured-input comparison and refused-import sizing diagnostic"
)]
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
        a.bytes = read(&format!("pack-{id}")).into();
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
    if std::env::var_os("PHOTARA_COMPILER_LOOKUP_ONLY").is_some() {
        restart::lookup_profile(&current, frames(), &reader, &identity);
        return;
    }
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
    let old = recovery::reconstruct_shared_original(&current, &body["original"], budget()).unwrap();
    let verified = restart::verify_original_prefix(old, &context).unwrap();
    layout::take_load_work();
    super::super::shared::take_copied();
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
    eprintln!(
        "compiler load work={work:?} allocation_copy_bytes={}",
        super::super::shared::take_copied()
    );
    for (id, bytes) in &plan.files {
        assert_eq!(bytes, &current.allocations[id].bytes);
    }
    assert_eq!(plan.stages[6].head, current.head);
    eprintln!(
        "captured compiler: elapsed={elapsed:?} vector={} bytes={}",
        vector(&plan),
        plan.files.values().map(|b| b.len()).sum::<usize>()
    );
    if std::env::var_os("PHOTARA_COMPILER_COMPLETE_PROFILE").is_some() {
        let mut memo = restart::CompiledPlans::default();
        if std::env::var_os("PHOTARA_COMPILER_PREFIX_PROFILE").is_some() {
            let mut end = journal::Journal::create(
                j.header(),
                journal::JournalLimits {
                    json: reader.json,
                    max_file_bytes: 4 << 20,
                    max_records: 4096,
                    max_work_bytes: 4 << 20,
                },
            )
            .unwrap()
            .len();
            for r in j.records() {
                if r.value() == record.value() {
                    break;
                }
                end += 36 + encode(r.value()).len();
            }
            let previous_journal = journal::Journal::read(
                &bytes[..end],
                j.header(),
                journal::JournalLimits {
                    json: reader.json,
                    max_file_bytes: 4 << 20,
                    max_records: 4096,
                    max_work_bytes: 4 << 20,
                },
            )
            .unwrap();
            let mut prior_observed = snapshot(&base, verified.shared_package());
            for (id, a) in &mut prior_observed.allocations {
                a.charge = a.charge.max(base.allocations[id].charge);
            }
            let prior_context = RestartContext {
                inspection: &prior_observed,
                ..context
            };
            previous_journal
                .verify_cached(verified.shared_package(), &prior_context, &mut memo)
                .unwrap();
            let baseline = std::env::var_os("PHOTARA_COMPILER_NO_PREFIX_REUSE").is_some();
            if baseline {
                memo.journal = None;
            }
            let compiles = memo.compilations;
            let before = std::time::Instant::now();
            let proof = j.verify_cached(&current, &context, &mut memo).unwrap();
            eprintln!(
                "captured completed extension: baseline={baseline} elapsed={:?} added_compiles={} replayed={} records={} compact_bytes={}",
                before.elapsed(),
                memo.compilations - compiles,
                memo.journal_replayed,
                j.records().len(),
                proof.retained_metadata_bytes()
            );
            if !baseline {
                assert!(memo.journal_replayed <= 4);
            }
            return;
        }
        for stage in [6, 6, 5, 6] {
            let package = selected(&plan, stage);
            let mut actual = snapshot(&base, &package);
            for (id, a) in &mut actual.allocations {
                a.charge = a.charge.max(base.allocations[id].charge);
            }
            let ctx = RestartContext {
                inspection: &actual,
                ..context
            };
            let before = std::time::Instant::now();
            memo.inspect_shared_candidate(&package, &ctx, &plan, stage)
                .unwrap();
            eprintln!(
                "captured complete candidate: stage={stage} elapsed={:?} full_checks={}",
                before.elapsed(),
                memo.candidate_checks
            );
        }
        #[cfg(target_os = "macos")]
        super::super::disposable::check_borrowed_snapshot(base.data.clone(), &current);
        let before = std::time::Instant::now();
        let work = memo.reader_work();
        restart::verify_completed_plan(&current, &plan, &context, Some(&mut memo), None).unwrap();
        assert_eq!(memo.reader_work(), (work.0, work.1 + 1));
        eprintln!(
            "captured executor-to-receipt reuse: elapsed={:?} reader={:?}",
            before.elapsed(),
            memo.reader_work()
        );
        for pass in 0..2 {
            super::super::shared::take_copied();
            let before = std::time::Instant::now();
            j.verify_cached(&current, &context, &mut memo).unwrap();
            eprintln!(
                "captured complete journal: pass={pass} elapsed={:?} reader={:?} compiles={} replayed={} journal_hits={} retained={:?}",
                before.elapsed(),
                memo.reader_work(),
                memo.compilations,
                memo.journal_replayed,
                memo.journal_hits,
                memo.retained()
            );
            eprintln!(
                "journal allocation_copy_bytes={}",
                super::super::shared::take_copied()
            );
        }
        if std::env::var_os("PHOTARA_COMPILER_MEMORY_PROFILE").is_some() {
            memory_profile("retained");
            drop(memo);
            memory_profile("released-memo");
        }
        return;
    }
    let v3 = std::env::var_os("PHOTARA_COMPILER_V3").is_some();
    if v3 || std::env::var_os("PHOTARA_COMPILER_V2").is_some() {
        let directory = context.directory;
        let mut old = plan
            .verify_next_shared_original(
                current.clone(),
                super::identity(&current),
                &observed,
                &base.registration,
                directory,
                frames(),
                &reader,
            )
            .unwrap();
        let mut intent = body["semantic_intent"].clone();
        let mut receipt = body["operation_receipt"].clone();
        let mut memo = restart::CompiledPlans::default();
        for n in 0..2u64 {
            let mut inputs = attempt(&old, 900_000 + n * 2000);
            inputs.project_limit = plan.original["project_limit"].as_str().unwrap().into();
            let coordinate = core::original_coordinate(&old, &inputs.planner, budget()).unwrap();
            intent["expected"] = coordinate.clone();
            intent["operation_id"] = json!(uid(990_000 + n));
            intent["command"]["envelope"]["command_id"] = intent["operation_id"].clone();
            let graph = intent["command"]["envelope"]["graph_id"]
                .as_str()
                .unwrap()
                .to_owned();
            intent["command"]["envelope"]["expected_revision"] = json!(
                coordinate["graphs"][&graph]["revision"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            );
            receipt["operation_id"] = intent["operation_id"].clone();
            receipt["before"] = receipt["after"].clone();
            receipt["acceptance_ordinal"] = json!(
                (receipt["acceptance_ordinal"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
                    + 1)
                .to_string()
            );
            receipt["journal_sequence"] = json!(
                (receipt["journal_sequence"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
                    + 1)
                .to_string()
            );
            receipt["request_sha256"] = json!(hash(&encode(&intent)));
            if n == 0 {
                let before = std::time::Instant::now();
                let legacy = compile(&old, &intent, &receipt, inputs.clone(), budget());
                eprintln!(
                    "grown-capture v1 next: elapsed={:?} outcome={:?}",
                    before.elapsed(),
                    legacy.as_ref().map(vector)
                );
            }
            inputs.planner.codec = if v3 {
                "photara.codec.ps2-repeatable-layout-v3"
            } else {
                "photara.codec.ps2-repeatable-layout-v2"
            }
            .into();
            layout::take_load_work();
            let before = std::time::Instant::now();
            let attempted = memo.compile(&old, &intent, &receipt, inputs.clone(), budget(), 256);
            if let Err(error) = &attempted {
                eprintln!("UNCHANGED CORRIDOR REFUSAL overlay {n}: {error}");
            }
            let next = match attempted {
                Ok(plan) => plan,
                Err(error) if !v3 && std::env::var_os("PHOTARA_COMPILER_SIZE_IMPORT").is_some() => {
                    eprintln!(
                        "DIAGNOSTIC ONLY: measuring complete append bytes after {error}; no native/admitted input changes"
                    );
                    for corridor in &mut inputs.planner.corridors {
                        corridor.maximum_end =
                            (corridor.original_end.parse::<u64>().unwrap() + 1_048_576).to_string();
                    }
                    memo.compile(&old, &intent, &receipt, inputs.clone(), budget(), 256)
                        .unwrap()
                }
                Err(error) => panic!("unchanged overlay corridor: {error}"),
            };
            let elapsed = before.elapsed();
            let work = layout::take_load_work();
            let exact = compile(&old, &intent, &receipt, inputs, budget()).unwrap();
            let uncached_work = layout::take_load_work();
            assert_eq!(vector(&next), vector(&exact));
            let mut append = BTreeMap::new();
            let mut composition = BTreeMap::<String, BTreeMap<String, (usize, usize)>>::new();
            for (id, bytes) in &next.files {
                let mut at = old.package.allocations[id].bytes.len();
                let mut live = 0;
                while at < bytes.len() {
                    let payload =
                        u32::from_le_bytes(bytes[at + 12..at + 16].try_into().unwrap()) as usize;
                    let end = at + 16 + payload;
                    assert!(end <= bytes.len());
                    if bytes[at + 8] != 0 {
                        live += end - at;
                        let body: Value = serde_json::from_slice(&bytes[at + 16..end]).unwrap();
                        let kind = body["schema"]["id"].as_str().unwrap_or("intent").to_owned();
                        let stat = composition
                            .entry(id.clone())
                            .or_default()
                            .entry(kind)
                            .or_default();
                        stat.0 += 1;
                        stat.1 += end - at;
                    }
                    at = end;
                }
                append.insert(id.clone(), live);
            }
            eprintln!(
                "grown-capture overlay {n}: elapsed={elapsed:?} cached_work={work:?} uncached_work={uncached_work:?} original_bytes={} append_nonpadding={append:?} vector={}",
                next.original_bytes().len(),
                vector(&next)
            );
            eprintln!(
                "overlay codec={} append composition {n}: {composition:?}",
                next.original["original_codec"]
            );
            if std::env::var_os("PHOTARA_COMPILER_WRITER_ONLY").is_some() {
                return;
            }
            current = selected(&next, 6);
            let mut observed = snapshot(&base, &current);
            for (id, a) in &mut observed.allocations {
                a.charge = base.allocations[id].charge.max(a.charge);
            }
            old = next
                .verify_next_shared_original(
                    current.clone(),
                    super::identity(&current),
                    &observed,
                    &base.registration,
                    directory,
                    frames(),
                    &reader,
                )
                .unwrap();
        }
    }
}

fn memory_profile(label: &str) {
    let output = std::process::Command::new("/usr/bin/heap")
        .args(["-s", "--noContent", &std::process::id().to_string()])
        .output()
        .unwrap();
    std::fs::write(
        format!("/private/tmp/ll2a-v3-capture-heap-{label}.log"),
        [&output.stdout[..], &output.stderr[..]].concat(),
    )
    .unwrap();
    eprintln!("capture heap {label}: {}", output.status);
}
