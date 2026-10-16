//! Existing integrated bytes through actual file/CAS primitives, not a native
//! storage qualification or an assertion that a synthetic snapshot is Saved.
use super::*;

pub(super) fn setup() -> (tempfile::TempDir, Namespace, Vec<u8>, Vec<u8>) {
    let temp = tempfile::tempdir().unwrap();
    let parent = File::from(super::super::open(temp.path(), DIRECTORY, Mode::empty()).unwrap());
    super::super::mkdirat(&parent, EVIDENCE, Mode::RWXU).unwrap();
    let root = File::from(openat(&parent, EVIDENCE, DIRECTORY, Mode::empty()).unwrap());
    let fixture = support::Fixture::load("integrated");
    let package = super::super::package(&fixture);
    let old = canonical(&package.head);
    let new = canonical(&package.commit);
    let file = super::super::create(&root, CHECKPOINT, &old).unwrap();
    super::super::create(&root, "selected-checkpoint", &old).unwrap();
    barrier(&root).unwrap();
    let namespace = Namespace::open(
        parent,
        root,
        "selected-checkpoint",
        PackageUuid::parse("10000000-0000-4000-8000-000000000010").unwrap(),
        pin(&file).unwrap(),
    )
    .unwrap();
    (temp, namespace, old, new)
}

#[test]
fn session_tests_native_checkpoint_custody_cut_retry_and_orphans() {
    let (_temp, namespace, _, _) = setup();
    let usage = v1_3::native_profile::measure_allocation(
        &regular(&namespace.root, CHECKPOINT, false).unwrap(),
    )
    .unwrap();
    let charge =
        v1_3::native_profile::check_charge(&[usage], 4096, u64::MAX / 4096 * 4096).unwrap();
    let footprint = serde_json::json!({"standing_observation":charge});
    assert_eq!(
        standing_charge(&footprint).unwrap(),
        charge.rounded_required_charge
    );
    assert!(preflight_charge(&footprint, 4096, charge.rounded_required_charge + 4096).is_ok());
    assert!(preflight_charge(&footprint, 4097, charge.rounded_required_charge + 4096).is_err());
    assert!(
        standing_charge(
            &serde_json::json!({"standing_observation":charge.rounded_required_charge})
        )
        .is_err()
    );
    let cuts = [
        Step::Write,
        Step::FileBarrier,
        Step::DirectoryBarrier,
        Step::Rename,
        Step::PublishedBarrier,
    ];
    for cut in cuts {
        for after in [false, true] {
            let (_temp, namespace, old, new) = setup();
            // Retained originals may precede selection and remain unselected.
            namespace.retain(&new, |_, _| Ok(())).unwrap();
            let count = namespace.retained().unwrap().len();
            assert_eq!(namespace.selector().unwrap(), old);
            let mut hit = false;
            assert!(
                namespace
                    .select(&old, &new, |step, done| {
                        if (step, done) == (cut, after) {
                            hit = true;
                            return Err(rustix::io::Errno::IO.into());
                        }
                        Ok(())
                    })
                    .is_err()
            );
            assert!(hit);
            assert!(namespace.retained().unwrap().len() >= count);
            namespace.select(&old, &new, |_, _| Ok(())).unwrap();
            assert_eq!(namespace.selector().unwrap(), new);
            let mut retry = Vec::new();
            namespace
                .select(&old, &new, |step, after| {
                    retry.push((step, after));
                    Ok(())
                })
                .unwrap();
            assert_eq!(
                retry,
                [
                    (Step::PublishedBarrier, false),
                    (Step::PublishedBarrier, true)
                ]
            );
            assert!(
                namespace
                    .select(b"unrelated", b"replacement", |_, _| Ok(()))
                    .is_err()
            );
        }
    }
    let (_temp, namespace, _, new) = setup();
    let name = record_name(&new);
    super::super::create(&namespace.root, &name, &new[..new.len() / 2]).unwrap();
    // Exact original retry can finish its torn immutable creation. Its retained
    // prefix is checked before append, and its own barriers are reissued.
    namespace.retain(&new, |_, _| Ok(())).unwrap();
    assert_eq!(read(&namespace.root, &name).unwrap(), new);
    let stream = PackageUuid::parse("10000000-0000-4000-8000-000000000011").unwrap();
    namespace
        .install_segment(stream, &new, |_, _| Ok(()))
        .unwrap();
    let records = namespace.records().unwrap();
    assert_eq!(records.frame(stream, 3, 9).unwrap(), new[3..12]);
    assert!(records.frame(stream, new.len() as u64, 1).is_err());
    assert!(
        records
            .frame(
                PackageUuid::parse("10000000-0000-4000-8000-000000000012").unwrap(),
                0,
                1
            )
            .is_err()
    );
    let value: Value = serde_json::from_slice(&new).unwrap();
    let reference = super::super::reference(&value);
    assert_eq!(records.record(&reference, new.len()).unwrap(), new);
    assert!(records.record(&reference, new.len() - 1).is_err());
    let mut file = regular(&namespace.root, &name, true).unwrap();
    file.write_all(b"!").unwrap();
    barrier(&file).unwrap();
    assert!(namespace.retain(&new, |_, _| Ok(())).is_err());
    assert!(records.record(&reference, new.len()).is_err());
}

#[test]
fn session_tests_native_checkpoint_namespace_binding_and_no_reset() {
    let (_temp, namespace, old, new) = setup();
    // A torn orphan is charged, not decoded or promoted to authority.
    super::super::create(&namespace.root, &record_name(&new), b"{").unwrap();
    assert_eq!(namespace.retained().unwrap().len(), 3);
    assert_eq!(namespace.selector().unwrap(), old);
    super::super::create(&namespace.root, "unexpected", b"{}").unwrap();
    assert!(namespace.retained().is_err());
    let (temp, namespace, _, _) = setup();
    let records = namespace.records().unwrap();
    renameat(
        &namespace.parent,
        EVIDENCE,
        &namespace.parent,
        "held-evidence",
    )
    .unwrap();
    super::super::mkdirat(&namespace.parent, EVIDENCE, Mode::RWXU).unwrap();
    assert!(namespace.selector().is_err());
    assert!(records.frame(namespace.initial_stream, 0, 1).is_err());
    assert!(temp.path().join("held-evidence").exists());
    let (_temp, namespace, _, _) = setup();
    let mut records = namespace.records().unwrap();
    let parent = namespace.parent.try_clone().unwrap();
    let calls = std::sync::atomic::AtomicUsize::new(0);
    // Keep the authority result unchanged while replacing the namespace after
    // bytes were read. The independent post-read name binding must reject it.
    records.authority = Some((
        std::sync::Arc::new(move || {
            if calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 1 {
                renameat(&parent, EVIDENCE, &parent, "held-evidence")?;
                super::super::mkdirat(&parent, EVIDENCE, Mode::RWXU)?;
            }
            Ok(Value::Null)
        }),
        Value::Null,
    ));
    assert!(records.frame(namespace.initial_stream, 0, 1).is_err());
    let (_temp, namespace, _, _) = setup();
    renameat(
        &namespace.root,
        &namespace.selector,
        &namespace.root,
        "retained-selector",
    )
    .unwrap();
    assert!(namespace.selector().is_err());
    assert!(
        Namespace::open(
            namespace.parent.try_clone().unwrap(),
            namespace.root.try_clone().unwrap(),
            &namespace.selector,
            namespace.initial_stream,
            namespace.initial_pin
        )
        .is_err()
    );
    assert!(read(&namespace.root, &namespace.selector).is_err());
}
