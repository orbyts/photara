//! Descriptor-relative, no-replace publication. Pins are device-local journal data.
use super::{InitialPackage, PackageError};
use rustix::{
    fd::OwnedFd,
    fs::{
        AtFlags, Mode, OFlags, RenameFlags, fsync, mkdirat, open, openat, renameat_with, unlinkat,
    },
};
use serde::{Deserialize, Serialize};
use std::os::unix::fs::MetadataExt as _;
use std::{
    fs::File,
    io::Write,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryPin {
    pub path: PathBuf,
    pub device: u64,
    pub inode: u64,
}
fn io(e: rustix::io::Errno) -> PackageError {
    PackageError::Io(std::io::Error::from(e).kind())
}
const FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

impl DirectoryPin {
    /// Capture a destination without following any symlink component.
    /// # Errors
    /// Refuses absent, non-directory, relative or unsafe destinations.
    pub fn inspect(path: &Path) -> Result<Self, PackageError> {
        let fd = open_directory(path)?;
        let (device, inode) = directory_identity(&fd)?;
        Ok(Self {
            path: path.into(),
            device,
            inode,
        })
    }
    fn open(&self) -> Result<OwnedFd, PackageError> {
        let fd = open_directory(&self.path)?;
        self.check(&fd)?;
        Ok(fd)
    }
    fn check(&self, fd: &OwnedFd) -> Result<(), PackageError> {
        let (device, inode) = directory_identity(fd)?;
        if device != self.device || inode != self.inode {
            return Err(PackageError::ChangedDuringRead);
        }
        Ok(())
    }
    /// Create a private staging directory exclusively. Persist the returned pin
    /// before writing contents. An occupied stage is never adopted by name.
    /// # Errors
    /// Returns destination change, collision or filesystem errors.
    pub fn create_stage(&self, package: &InitialPackage) -> Result<Self, PackageError> {
        let parent = self.open()?;
        let name = format!("{}-{}", stage_prefix(package), uuid::Uuid::new_v4());
        mkdirat(&parent, &name, Mode::RWXU).map_err(io)?;
        let stage = openat(&parent, &name, FLAGS, Mode::empty()).map_err(io)?;
        let (device, inode) = directory_identity(&stage)?;
        fsync(&parent).map_err(io)?;
        self.open()?;
        Ok(Self {
            path: self.path.join(name),
            device,
            inode,
        })
    }
    /// Writes/reconstructs only a durably reserved, pinned private staging directory.
    /// # Errors
    /// Refuses unsafe entries, hardlinks, changed destinations and concurrent writers.
    pub fn materialize(&self, package: &InitialPackage) -> Result<(), PackageError> {
        let root = self.open()?;
        let lock = File::from(
            openat(
                &root,
                ".creation-lock",
                OFlags::CREATE | OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(io)?,
        );
        lock.try_lock()
            .map_err(|_| PackageError::ChangedDuringRead)?;
        for dir in ["commits", "objects", "objects/json", "objects/json/sha256"] {
            make_dirs(&root, dir)?;
        }
        for (path, bytes) in &package.files {
            super::super::reader::internal_path(path)?;
            let (parent, name) = child_parent(&root, path)?;
            let mut file = File::from(
                openat(
                    &parent,
                    &name,
                    OFlags::CREATE
                        | OFlags::RDWR
                        | OFlags::NOFOLLOW
                        | OFlags::NONBLOCK
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )
                .map_err(io)?,
            );
            let meta = file.metadata().map_err(|e| PackageError::Io(e.kind()))?;
            if !meta.is_file() || meta.nlink() != 1 {
                return Err(PackageError::Path);
            }
            file.set_len(0)
                .and_then(|()| file.write_all(bytes))
                .and_then(|()| file.sync_all())
                .map_err(|e| PackageError::Io(e.kind()))?;
            fsync(&parent).map_err(io)?;
        }
        fsync(&root).map_err(io)?;
        self.open()?;
        let verified = super::super::v1_1::validate_directory(
            &self.path,
            super::super::PackageLimits::default(),
        )?;
        if verified.head.commit_sha256 != package.commit_sha256 {
            return Err(PackageError::Integrity);
        }
        Ok(())
    }
    /// Publish a completed stage using the OS exclusive rename operation.
    /// # Errors
    /// Never replaces any existing path, including empty folders and symlinks.
    pub fn publish(&self, stage: &Self, package: &InitialPackage) -> Result<PathBuf, PackageError> {
        let parent = self.open()?;
        let name = stage_child(stage, self, package)?;
        stage.open()?;
        let verified = super::super::v1_1::validate_directory(
            &stage.path,
            super::super::PackageLimits::default(),
        )?;
        if verified.head.commit_sha256 != package.commit_sha256 {
            return Err(PackageError::Integrity);
        }
        let target = format!("{}.photara", package.spec.title);
        renameat_with(&parent, &name, &parent, &target, RenameFlags::NOREPLACE).map_err(io)?;
        fsync(&parent).map_err(io)?;
        self.open()?;
        Ok(self.path.join(target))
    }
    /// Remove only files in the exact owned staging closure. Unknown entries make
    /// cleanup incomplete rather than being recursively deleted.
    /// # Errors
    /// Refuses replaced staging directories and reports incomplete cleanup.
    pub fn discard_stage(
        &self,
        destination: &Self,
        package: &InitialPackage,
    ) -> Result<(), PackageError> {
        let name = stage_child(self, destination, package)?;
        let root = self.open()?;
        for path in package
            .files
            .keys()
            .chain(std::iter::once(&".creation-lock".to_owned()))
        {
            if let Ok((parent, name)) = child_parent(&root, path) {
                match unlinkat(&parent, &name, AtFlags::empty()) {
                    Ok(()) | Err(rustix::io::Errno::NOENT) => (),
                    Err(e) => return Err(io(e)),
                }
            }
        }
        for path in ["objects/json/sha256", "objects/json", "objects", "commits"] {
            if let Ok((parent, name)) = child_parent(&root, path) {
                match unlinkat(&parent, &name, AtFlags::REMOVEDIR) {
                    Ok(()) | Err(rustix::io::Errno::NOENT) => (),
                    Err(e) => return Err(io(e)),
                }
            }
        }
        let parent = destination.open()?;
        self.open()?;
        unlinkat(&parent, name, AtFlags::REMOVEDIR).map_err(io)?;
        fsync(&parent).map_err(io)
    }
}
fn stage_prefix(package: &InitialPackage) -> String {
    format!(".photara-create-{}", package.spec.operation_id)
}
fn stage_child(
    stage: &DirectoryPin,
    destination: &DirectoryPin,
    package: &InitialPackage,
) -> Result<String, PackageError> {
    if stage.path.parent() != Some(destination.path.as_path()) {
        return Err(PackageError::Path);
    }
    let name = stage
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(PackageError::Path)?;
    let prefix = format!("{}-", stage_prefix(package));
    let suffix = name.strip_prefix(&prefix).ok_or(PackageError::Path)?;
    if uuid::Uuid::parse_str(suffix).is_err() {
        return Err(PackageError::Path);
    }
    Ok(name.to_owned())
}
fn open_directory(path: &Path) -> Result<OwnedFd, PackageError> {
    if !path.is_absolute() {
        return Err(PackageError::Path);
    }
    let mut fd = open("/", FLAGS, Mode::empty()).map_err(io)?;
    for part in path.components() {
        match part {
            Component::RootDir => (),
            Component::Normal(name) => fd = openat(&fd, name, FLAGS, Mode::empty()).map_err(io)?,
            _ => return Err(PackageError::Path),
        }
    }
    Ok(fd)
}
fn make_dirs(root: &OwnedFd, path: &str) -> Result<(), PackageError> {
    let mut parent = rustix::io::dup(root).map_err(io)?;
    for name in path.split('/') {
        match mkdirat(&parent, name, Mode::RWXU) {
            Ok(()) | Err(rustix::io::Errno::EXIST) => (),
            Err(e) => return Err(io(e)),
        }
        let child = openat(&parent, name, FLAGS, Mode::empty()).map_err(io)?;
        fsync(&parent).map_err(io)?;
        parent = child;
    }
    Ok(())
}
fn child_parent(root: &OwnedFd, path: &str) -> Result<(OwnedFd, String), PackageError> {
    let mut parent = rustix::io::dup(root).map_err(io)?;
    let mut parts = path.split('/').peekable();
    while let Some(name) = parts.next() {
        if parts.peek().is_none() {
            return Ok((parent, name.to_owned()));
        }
        parent = openat(&parent, name, FLAGS, Mode::empty()).map_err(io)?;
    }
    Err(PackageError::Path)
}

fn directory_identity(fd: &OwnedFd) -> Result<(u64, u64), PackageError> {
    let file = File::from(rustix::io::dup(fd).map_err(io)?);
    let m = file.metadata().map_err(|e| PackageError::Io(e.kind()))?;
    Ok((m.dev(), m.ino()))
}
