#![cfg(unix)]
use photara_core::{contracts::*, creation::InitialProject};
use photara_store::package::{
    PackageLimits,
    creation::{DirectoryPin, InitialPackage},
    v1_1,
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
fn package() -> InitialPackage {
    InitialPackage::build(InitialProject {
        operation_id: OperationId::from_uuid(uuid::Uuid::new_v4()).unwrap(),
        library_id: LibraryId::from_uuid(uuid::Uuid::new_v4()).unwrap(),
        project_id: ProjectId::from_uuid(uuid::Uuid::new_v4()).unwrap(),
        graph_id: GraphId::from_uuid(uuid::Uuid::new_v4()).unwrap(),
        commit_id: CommitId::from_uuid(uuid::Uuid::new_v4()).unwrap(),
        title: "夏 Project".into(),
        created_at: "2026-09-14T12:00:00.000Z".into(),
    })
    .unwrap()
}
#[test]
fn published_package_has_exact_identity_graph_closure_and_replay_bytes() {
    let root = tempfile::tempdir_in("/private/tmp").unwrap();
    let p = package();
    assert_eq!(
        p.files,
        InitialPackage::build(p.spec.clone()).unwrap().files
    );
    let destination = DirectoryPin::inspect(root.path()).unwrap();
    let stage = destination.create_stage(&p).unwrap();
    stage.materialize(&p).unwrap();
    stage.materialize(&p).unwrap();
    let path = destination.publish(&stage, &p).unwrap();
    let verified = v1_1::validate_directory(&path, PackageLimits::default()).unwrap();
    assert_eq!(verified.head.commit_sha256, p.commit_sha256);
    assert_eq!(
        verified.authored.value["owning_library_id"],
        p.spec.library_id.to_string()
    );
    assert_eq!(verified.graphs.len(), 1);
    assert_eq!(
        verified.graphs[0].value["graph_id"],
        p.spec.graph_id.to_string()
    );
    assert_eq!(
        verified.graphs[0].value["graph"]["nodes"],
        serde_json::json!([])
    );
    assert_eq!(verified.verified_blobs, 0);
    assert_eq!(p.files.len(), 13);
    for (name, bytes) in &p.files {
        assert_eq!(&fs::read(path.join(name)).unwrap(), bytes);
    }
}
#[test]
fn collision_does_not_replace_empty_directory_file_or_symlink() {
    for kind in 0..3 {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let p = package();
        let destination = DirectoryPin::inspect(root.path()).unwrap();
        let stage = destination.create_stage(&p).unwrap();
        stage.materialize(&p).unwrap();
        let path = root.path().join(format!("{}.photara", p.spec.title));
        match kind {
            0 => fs::create_dir(&path).unwrap(),
            1 => fs::write(&path, b"untouched").unwrap(),
            _ => symlink("/does-not-exist", &path).unwrap(),
        }
        assert!(destination.publish(&stage, &p).is_err());
        assert!(fs::symlink_metadata(&path).is_ok());
        stage.discard_stage(&destination, &p).unwrap();
        assert!(!stage.path.exists());
        assert!(fs::symlink_metadata(&path).is_ok());
    }
}
#[test]
fn replaced_destination_and_stage_symlinks_are_refused() {
    let root = tempfile::tempdir_in("/private/tmp").unwrap();
    let dir = root.path().join("volume");
    fs::create_dir(&dir).unwrap();
    let pin = DirectoryPin::inspect(&dir).unwrap();
    fs::rename(&dir, root.path().join("detached")).unwrap();
    fs::create_dir(&dir).unwrap();
    assert!(pin.create_stage(&package()).is_err());
    let pin = DirectoryPin::inspect(&dir).unwrap();
    let p = package();
    let stage = pin.create_stage(&p).unwrap();
    symlink(root.path(), stage.path.join("objects")).unwrap();
    assert!(stage.materialize(&p).is_err());
    assert!(!root.path().join("json").exists());
}
#[test]
fn interrupted_stage_rebuilds_and_permission_failure_is_recoverable() {
    let root = tempfile::tempdir_in("/private/tmp").unwrap();
    let p = package();
    let pin = DirectoryPin::inspect(root.path()).unwrap();
    let stage = pin.create_stage(&p).unwrap();
    fs::write(stage.path.join("manifest.json"), b"partial").unwrap();
    stage.materialize(&p).unwrap();
    let blocked = root.path().join("denied");
    fs::create_dir(&blocked).unwrap();
    let denied = DirectoryPin::inspect(&blocked).unwrap();
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o500)).unwrap();
    assert!(denied.create_stage(&p).is_err());
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(denied.create_stage(&p).is_ok());
}
