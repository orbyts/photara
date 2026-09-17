//! Synthetic binding tests only: no registrar, authorization provider or OS locks.
#![allow(clippy::unwrap_used)]

use super::*;
use crate::package::{DecimalU64, planning::IncarnationId};

fn digest(byte: &str) -> Sha256Hex {
    Sha256Hex::parse(&byte.repeat(32)).unwrap()
}

fn expected() -> AdmissionExpectation {
    AdmissionExpectation {
        head: HeadToken {
            incarnation: IncarnationId::parse("10000000-0000-4000-8000-000000000001").unwrap(),
            manifest: b"synthetic manifest".as_slice().into(),
            head: b"synthetic HEAD".as_slice().into(),
            head_digest: digest("11"),
            manifest_digest: digest("22"),
            revision: DecimalU64::parse("1").unwrap(),
        },
        volume_identity: digest("33"),
        owner_epoch: OwnerEpoch::parse("10000000-0000-4000-8000-000000000002").unwrap(),
        protocol_digest: digest("44"),
    }
}

// Private fixture minting deliberately bypasses real admission, which does not
// exist yet. Outside callers cannot construct or deserialize this evidence.
fn fixture_lease(binding: &AdmissionExpectation) -> RegisteredCooperativeLease<()> {
    RegisteredCooperativeLease {
        platform_lease: (),
        binding: binding.clone(),
    }
}

#[test]
fn missing_admission_cannot_be_replaced_by_expected_coordinates() {
    assert_eq!(
        check_admission::<()>(None, &expected()),
        Err(AdmissionFailure::NotRegistered)
    );
}

#[test]
fn exact_registered_binding_matches_without_minting_new_authority() {
    let expected = expected();
    let admission = fixture_lease(&expected);
    assert_eq!(check_admission(Some(&admission), &expected), Ok(()));
    assert_eq!(admission.platform_lease(), &());
}

#[test]
fn changed_identity_volume_or_exact_head_refuses() {
    let expected = expected();
    let admission = fixture_lease(&expected);
    let mut changed = expected.clone();
    changed.head.incarnation =
        IncarnationId::parse("10000000-0000-4000-8000-000000000003").unwrap();
    let mut volume = expected.clone();
    volume.volume_identity = digest("55");
    let mut head = expected.clone();
    // Even unchanged claimed digest/revision does not bypass exact-byte binding.
    head.head.head = b"other HEAD".as_slice().into();
    let mut manifest = expected.clone();
    manifest.head.manifest = b"other manifest".as_slice().into();
    let mut revision = expected;
    revision.head.revision = DecimalU64::parse("2").unwrap();
    for candidate in [changed, volume, head, manifest, revision] {
        assert_eq!(
            check_admission(Some(&admission), &candidate),
            Err(AdmissionFailure::IdentityOrHeadChanged)
        );
    }
}

#[test]
fn stale_owner_or_incompatible_protocol_refuses() {
    let expected = expected();
    let admission = fixture_lease(&expected);
    let mut owner = expected.clone();
    owner.owner_epoch = OwnerEpoch::parse("10000000-0000-4000-8000-000000000004").unwrap();
    assert_eq!(
        check_admission(Some(&admission), &owner),
        Err(AdmissionFailure::OwnerChanged)
    );
    let mut protocol = expected;
    protocol.protocol_digest = digest("66");
    assert_eq!(
        check_admission(Some(&admission), &protocol),
        Err(AdmissionFailure::ProtocolMismatch)
    );
}
