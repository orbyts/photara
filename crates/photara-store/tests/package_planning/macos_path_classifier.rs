//! Dry-run, device-local evidence model only; no writer, wire format or admission.
//! Real observations cannot produce the synthetic provider-exclusion evidence.
use super::*;
use rustix::fs::{Mode, OFlags, fstatfs, open};
use std::{fs::File, os::unix::fs::MetadataExt as _, process::Command};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct Facts {
    observer_version: u32,
    policy_version: u32,
    adapter_revision: String,
    os_build: String,
    filesystem: String,
    mount_flags: u64,
    mount_source: String,
    volume_uuid: Option<String>,
    device: u64,
    boot_session: Option<String>,
    // Supplied by a future event-aware host authority, never inferred from st_dev.
    mount_epoch: Option<u64>,
    hardware_mapping: Option<String>,
    binding_generation: u64,
    locator: String,
    parent_inode: u64,
    root_inode: u64,
    root_is_directory: bool,
    namespace_pins_verified: bool,
}
type FactChange = fn(&mut Facts);

impl Facts {
    fn digest(&self) -> String {
        hash(&canon(&serde_json::to_value(self).unwrap()))
    }
}

#[derive(Clone, Debug)]
enum ProviderEvidence {
    Managed(&'static str),
    // Neither false iCloud status nor API errors establish global exclusion.
    Unknown(Vec<&'static str>),
    // Test oracle only. No real API adapter or production constructor exists.
    SyntheticScopedExclusion {
        facts_digest: String,
        policy_version: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Refusal {
    UnsupportedObserver,
    UnsupportedPolicy,
    MissingAdapterOrOs,
    UnsupportedFilesystem,
    Nonlocal,
    ReadOnly,
    MissingPersistentVolume,
    MissingMountContinuity,
    MissingHardwareMapping,
    InvalidNamespace,
    ProviderManaged,
    ProviderUnknown,
    StaleProviderEvidence,
}

#[derive(Debug, Eq, PartialEq)]
enum Decision {
    Refused(BTreeSet<Refusal>),
    SyntheticCandidateForFurtherQualification,
}

fn classify(facts: &Facts, provider: &ProviderEvidence) -> Decision {
    let mut missing = BTreeSet::new();
    let checks = [
        (facts.observer_version != 1, Refusal::UnsupportedObserver),
        (facts.policy_version != 1, Refusal::UnsupportedPolicy),
        (
            facts.adapter_revision.is_empty() || facts.os_build.is_empty(),
            Refusal::MissingAdapterOrOs,
        ),
        (facts.filesystem != "apfs", Refusal::UnsupportedFilesystem),
        (facts.mount_flags & 0x1000 == 0, Refusal::Nonlocal),
        (facts.mount_flags & 1 != 0, Refusal::ReadOnly),
        (
            facts.volume_uuid.as_ref().is_none_or(String::is_empty),
            Refusal::MissingPersistentVolume,
        ),
        (
            facts.boot_session.as_ref().is_none_or(String::is_empty) || facts.mount_epoch.is_none(),
            Refusal::MissingMountContinuity,
        ),
        (
            facts.hardware_mapping.as_ref().is_none_or(String::is_empty),
            Refusal::MissingHardwareMapping,
        ),
        (
            !facts.root_is_directory
                || !facts.namespace_pins_verified
                || facts.binding_generation == 0,
            Refusal::InvalidNamespace,
        ),
    ];
    for (failed, reason) in checks {
        if failed {
            missing.insert(reason);
        }
    }
    match provider {
        ProviderEvidence::Managed(source) => {
            assert!(!source.is_empty());
            missing.insert(Refusal::ProviderManaged);
        }
        ProviderEvidence::Unknown(observations) => {
            // Keep observations for diagnosis; their count or wording is not authority.
            let _ = observations;
            missing.insert(Refusal::ProviderUnknown);
        }
        ProviderEvidence::SyntheticScopedExclusion {
            facts_digest,
            policy_version,
        } => {
            if *facts_digest != facts.digest() || *policy_version != facts.policy_version {
                missing.insert(Refusal::StaleProviderEvidence);
            }
        }
    }
    if missing.is_empty() {
        Decision::SyntheticCandidateForFurtherQualification
    } else {
        Decision::Refused(missing)
    }
}

struct Ticket {
    facts: Facts,
    provider: ProviderEvidence,
}
impl Ticket {
    fn capture(facts: Facts, provider: ProviderEvidence) -> Result<Self, Decision> {
        let decision = classify(&facts, &provider);
        if decision != Decision::SyntheticCandidateForFurtherQualification {
            return Err(decision);
        }
        Ok(Self { facts, provider })
    }

    fn current(&self, observed: &Facts, provider: &ProviderEvidence) -> bool {
        // Every security-relevant version/identity is bound, including event continuity.
        self.facts == *observed
            && classify(observed, provider) == Decision::SyntheticCandidateForFurtherQualification
            && classify(&self.facts, &self.provider)
                == Decision::SyntheticCandidateForFurtherQualification
    }
}

fn synthetic() -> Facts {
    Facts {
        observer_version: 1,
        policy_version: 1,
        adapter_revision: "fixture-adapter-1".into(),
        os_build: "fixture-os-build".into(),
        filesystem: "apfs".into(),
        mount_flags: 0x1000,
        mount_source: "/dev/mock-disk".into(),
        volume_uuid: Some("mock-volume-uuid".into()),
        device: 42,
        boot_session: Some("mock-boot".into()),
        mount_epoch: Some(7),
        hardware_mapping: Some("mock-volume-to-Apple-SSD/model/firmware".into()),
        binding_generation: 1,
        locator: "/mock/project".into(),
        parent_inode: 11,
        root_inode: 12,
        root_is_directory: true,
        namespace_pins_verified: true,
    }
}

fn oracle(facts: &Facts) -> ProviderEvidence {
    ProviderEvidence::SyntheticScopedExclusion {
        facts_digest: facts.digest(),
        policy_version: facts.policy_version,
    }
}

fn refused(decision: Decision, reason: Refusal) {
    let Decision::Refused(reasons) = decision else {
        panic!("unexpected candidate")
    };
    assert!(reasons.contains(&reason), "{reasons:?} lacks {reason:?}");
}

#[test]
fn provider_positive_negative_and_ambiguous_observations_are_not_interchangeable() {
    let facts = synthetic();
    for source in [
        "mock Dropbox legacy sync",
        "mock Dropbox File Provider",
        "mock iCloud",
        "mock other File Provider",
    ] {
        refused(
            classify(&facts, &ProviderEvidence::Managed(source)),
            Refusal::ProviderManaged,
        );
    }
    for observations in [
        vec![],
        vec!["isUbiquitousItem=false"],
        vec!["NSFileNoSuchFileError"],
        vec!["no known cloud folder prefix", "no provider xattrs"],
        vec!["domain enumeration empty", "query access denied"],
        vec!["local APFS", "Apple SSD", "F_FULLFSYNC succeeded"],
    ] {
        refused(
            classify(&facts, &ProviderEvidence::Unknown(observations)),
            Refusal::ProviderUnknown,
        );
    }
    assert_eq!(
        classify(&facts, &oracle(&facts)),
        Decision::SyntheticCandidateForFurtherQualification
    );
}

#[test]
fn unsupported_incomplete_remote_and_readonly_facts_fail_closed() {
    let changes: Vec<(FactChange, Refusal)> = vec![
        (|f| f.observer_version = 2, Refusal::UnsupportedObserver),
        (|f| f.policy_version = 2, Refusal::UnsupportedPolicy),
        (|f| f.adapter_revision.clear(), Refusal::MissingAdapterOrOs),
        (|f| f.os_build.clear(), Refusal::MissingAdapterOrOs),
        (
            |f| f.filesystem = "smbfs".into(),
            Refusal::UnsupportedFilesystem,
        ),
        (|f| f.mount_flags &= !0x1000, Refusal::Nonlocal),
        (|f| f.mount_flags |= 1, Refusal::ReadOnly),
        (|f| f.volume_uuid = None, Refusal::MissingPersistentVolume),
        (|f| f.boot_session = None, Refusal::MissingMountContinuity),
        (|f| f.mount_epoch = None, Refusal::MissingMountContinuity),
        (
            |f| f.hardware_mapping = None,
            Refusal::MissingHardwareMapping,
        ),
        (|f| f.root_is_directory = false, Refusal::InvalidNamespace),
        (
            |f| f.namespace_pins_verified = false,
            Refusal::InvalidNamespace,
        ),
        (|f| f.binding_generation = 0, Refusal::InvalidNamespace),
    ];
    for (change, reason) in changes {
        let mut facts = synthetic();
        change(&mut facts);
        refused(classify(&facts, &oracle(&facts)), reason);
    }
}

#[test]
fn remount_reboot_rebind_path_substitution_and_policy_changes_invalidate_tickets() {
    let original = synthetic();
    let evidence = oracle(&original);
    let ticket = Ticket::capture(original.clone(), evidence.clone()).unwrap();
    assert!(ticket.current(&original, &evidence));
    let changes: Vec<FactChange> = vec![
        |f| f.mount_epoch = Some(8), // Same UUID/device/inode, different mount lifetime.
        |f| f.mount_epoch = None,    // Event-stream gap; continuity cannot be reconstructed.
        |f| f.boot_session = Some("next-boot".into()),
        |f| f.mount_source = "/dev/reassigned-disk".into(),
        |f| f.volume_uuid = Some("different-volume".into()),
        |f| f.device += 1,
        |f| f.parent_inode += 1,
        |f| f.root_inode += 1,
        |f| f.locator = "/mock/moved-project".into(),
        |f| f.binding_generation += 1,
        |f| f.os_build = "new-os-build".into(),
        |f| f.adapter_revision = "fixture-adapter-2".into(),
        |f| f.policy_version += 1,
        |f| f.hardware_mapping = Some("replacement-device".into()),
        |f| f.mount_flags |= 1,
    ];
    for change in changes {
        let mut facts = original.clone();
        change(&mut facts);
        assert!(!ticket.current(&facts, &evidence));
        // Even newly scoped oracle evidence cannot resurrect an old identity ticket.
        assert!(!ticket.current(&facts, &oracle(&facts)));
        refused(classify(&facts, &evidence), Refusal::StaleProviderEvidence);
    }
    assert!(!ticket.current(
        &original,
        &ProviderEvidence::Managed("mock newly active provider")
    ));
    assert!(!ticket.current(
        &original,
        &ProviderEvidence::Unknown(vec!["provider assessment expired"])
    ));
}

#[test]
fn provider_proof_cannot_be_reused_for_another_path_or_policy() {
    let facts = synthetic();
    let mut other = facts.clone();
    other.root_inode += 1;
    refused(
        classify(&facts, &oracle(&other)),
        Refusal::StaleProviderEvidence,
    );
    let wrong = ProviderEvidence::SyntheticScopedExclusion {
        facts_digest: facts.digest(),
        policy_version: 2,
    };
    refused(classify(&facts, &wrong), Refusal::StaleProviderEvidence);
    assert!(Ticket::capture(facts, ProviderEvidence::Unknown(vec![])).is_err());
}

fn platform_text(argument: &str) -> String {
    let output = Command::new("/usr/bin/sw_vers")
        .arg(argument)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn native_text(chars: &[std::ffi::c_char]) -> String {
    String::from_utf8(
        chars
            .iter()
            .take_while(|c| **c != 0)
            .map(|c| u8::try_from(*c).unwrap())
            .collect(),
    )
    .unwrap()
}

#[test]
#[ignore = "explicit read-only native observation of a newly allocated fixture; never scans provider directories"]
fn observe_local_apfs_without_promoting_missing_provider_or_volume_evidence() {
    let temporary = tempfile::tempdir().unwrap();
    let root_path = temporary.path().join("selected-root");
    fs::create_dir(&root_path).unwrap();
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let root = File::from(open(&root_path, flags, Mode::empty()).unwrap());
    let stat = fstatfs(&root).unwrap();
    let metadata = root.metadata().unwrap();
    let facts = Facts {
        observer_version: 1,
        policy_version: 1,
        adapter_revision: "test-only-observer-1".into(),
        os_build: platform_text("-buildVersion"),
        filesystem: native_text(&stat.f_fstypename),
        mount_flags: u64::from(stat.f_flags),
        mount_source: native_text(&stat.f_mntfromname),
        volume_uuid: None,
        device: metadata.dev(),
        boot_session: None,
        mount_epoch: None,
        hardware_mapping: None,
        binding_generation: 1,
        locator: "new disposable root".into(),
        parent_inode: temporary.path().metadata().unwrap().ino(),
        root_inode: metadata.ino(),
        root_is_directory: metadata.is_dir(),
        namespace_pins_verified: false,
    };
    let decision = classify(
        &facts,
        &ProviderEvidence::Unknown(vec!["no authoritative provider exclusion adapter"]),
    );
    assert!(matches!(decision, Decision::Refused(_)));
    println!(
        "{}",
        json!({"macos":platform_text("-productVersion"),"facts":facts,"decision":format!("{decision:?}"),"production_admitted":false,"qualified":false,"power_loss_tested":false,"scope":"read-only facts from newly allocated fixture; provider data not inspected"})
    );
    // Actual pathname replacement invalidates the current path observation too.
    fs::rename(&root_path, temporary.path().join("old-root")).unwrap();
    fs::create_dir(&root_path).unwrap();
    let replacement = File::from(open(&root_path, flags, Mode::empty()).unwrap());
    assert_ne!(metadata.ino(), replacement.metadata().unwrap().ino());
    assert_eq!(metadata.ino(), root.metadata().unwrap().ino());
}
