use super::{ObjectKind, ObjectRef, PackageError, PackageLimits, PackageUuid, Sha256Hex};
use std::path::Path;

/// Checks a normalized external relative resource hint without resolving it.
///
/// # Errors
/// Rejects traversal, alternate separators, absolute/scheme-like paths and
/// package-control names. No external filesystem access occurs.
pub fn validate_resource_path(value: &str) -> Result<(), PackageError> {
    if value.is_empty()
        || value.len() > 1024
        || value.contains(['\\', ':'])
        || value.chars().any(char::is_control)
    {
        return Err(PackageError::Path);
    }
    let mut parts = value.split('/');
    let first = parts.next().ok_or(PackageError::Path)?;
    for part in std::iter::once(first).chain(parts) {
        if part.is_empty() || matches!(part, "." | "..") || part.ends_with(['.', ' ']) {
            return Err(PackageError::Path);
        }
    }
    if [
        "head.json",
        "manifest.json",
        "commits",
        "objects",
        ".staging",
        ".coordination",
    ]
    .contains(&first.to_ascii_lowercase().as_str())
    {
        return Err(PackageError::Path);
    }
    Ok(())
}

pub(super) fn internal_path(value: &str) -> Result<(), PackageError> {
    match value.split('/').collect::<Vec<_>>().as_slice() {
        ["manifest.json" | "HEAD.json"] => Ok(()),
        ["commits", file] => {
            PackageUuid::parse(file.strip_suffix(".json").ok_or(PackageError::Path)?)?;
            Ok(())
        }
        ["objects", "json", "sha256", file] => {
            Sha256Hex::parse(file.strip_suffix(".json").ok_or(PackageError::Path)?)?;
            Ok(())
        }
        ["objects", "blobs", "sha256", file] => {
            Sha256Hex::parse(file)?;
            Ok(())
        }
        _ => Err(PackageError::Path),
    }
}

#[cfg(unix)]
mod platform {
    use super::{ObjectKind, ObjectRef, PackageError, PackageLimits, Path, internal_path};
    use rustix::{
        fd::OwnedFd,
        fs::{Dir, Mode, OFlags, open, openat},
    };
    use sha2::{Digest as _, Sha256};
    use std::{
        fs::File,
        io::{ErrorKind, Read},
        os::unix::fs::MetadataExt,
    };

    pub(crate) struct Reader {
        root: OwnedFd,
        pub limits: PackageLimits,
    }
    impl Reader {
        pub fn open(root: &Path, limits: PackageLimits) -> Result<Self, PackageError> {
            let root = open(
                root,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| PackageError::Path)?;
            Ok(Self { root, limits })
        }
        fn exact_entry(&self, dir: &OwnedFd, name: &str) -> Result<(), PackageError> {
            let entries = Dir::read_from(dir).map_err(|_| PackageError::Path)?;
            for (index, entry) in entries.enumerate() {
                if index > self.limits.max_objects.saturating_add(1024) {
                    return Err(PackageError::Limit);
                }
                if entry
                    .map_err(|_| PackageError::Path)?
                    .file_name()
                    .to_bytes()
                    == name.as_bytes()
                {
                    return Ok(());
                }
            }
            Err(PackageError::Io(ErrorKind::NotFound))
        }
        fn file(&self, path: &str) -> Result<File, PackageError> {
            internal_path(path)?;
            let parts: Vec<_> = path.split('/').collect();
            let mut directories = Vec::new();
            for part in &parts[..parts.len() - 1] {
                let parent = directories.last().unwrap_or(&self.root);
                self.exact_entry(parent, part)?;
                directories.push(
                    openat(
                        parent,
                        *part,
                        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map_err(|_| PackageError::Path)?,
                );
            }
            let parent = directories.last().unwrap_or(&self.root);
            let name = parts.last().ok_or(PackageError::Path)?;
            self.exact_entry(parent, name)?;
            let fd = openat(
                parent,
                *name,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| PackageError::Path)?;
            let file = File::from(fd);
            let metadata = file.metadata().map_err(|e| PackageError::Io(e.kind()))?;
            if !metadata.is_file() || metadata.nlink() != 1 {
                return Err(PackageError::Path);
            }
            Ok(file)
        }
        pub fn json(&self, path: &str) -> Result<Vec<u8>, PackageError> {
            let mut file = self.file(path)?;
            let before = file.metadata().map_err(|e| PackageError::Io(e.kind()))?;
            if before.len() > self.limits.json.max_bytes as u64 {
                return Err(PackageError::Limit);
            }
            let mut bytes = Vec::new();
            (&mut file)
                .take((self.limits.json.max_bytes as u64).saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|e| PackageError::Io(e.kind()))?;
            if bytes.len() > self.limits.json.max_bytes {
                return Err(PackageError::Limit);
            }
            stable(&before, &file)?;
            Ok(bytes)
        }
        pub fn blob(&self, reference: &ObjectRef) -> Result<(), PackageError> {
            if reference.kind != ObjectKind::Blob {
                return Err(PackageError::Record);
            }
            if reference.byte_length.get() > self.limits.max_blob_bytes {
                return Err(PackageError::Limit);
            }
            let mut file = self.file(&reference.path())?;
            let before = file.metadata().map_err(|e| PackageError::Io(e.kind()))?;
            if before.len() != reference.byte_length.get() {
                return Err(PackageError::Integrity);
            }
            let mut digest = Sha256::new();
            let mut count = 0u64;
            let mut buffer = [0u8; 16384];
            loop {
                let n = file
                    .read(&mut buffer)
                    .map_err(|e| PackageError::Io(e.kind()))?;
                if n == 0 {
                    break;
                }
                count = count.checked_add(n as u64).ok_or(PackageError::Limit)?;
                if count > reference.byte_length.get() {
                    return Err(PackageError::Integrity);
                }
                digest.update(&buffer[..n]);
            }
            stable(&before, &file)?;
            if count != reference.byte_length.get()
                || format!("{:x}", digest.finalize()) != reference.sha256.as_str()
            {
                return Err(PackageError::Integrity);
            }
            Ok(())
        }
    }
    fn stable(before: &std::fs::Metadata, file: &File) -> Result<(), PackageError> {
        let after = file.metadata().map_err(|e| PackageError::Io(e.kind()))?;
        if before.dev() != after.dev()
            || before.ino() != after.ino()
            || before.len() != after.len()
            || before.nlink() != after.nlink()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
            || before.ctime() != after.ctime()
            || before.ctime_nsec() != after.ctime_nsec()
        {
            return Err(PackageError::ChangedDuringRead);
        }
        Ok(())
    }
}

#[cfg(not(unix))]
mod platform {
    use super::{ObjectRef, PackageError, PackageLimits, Path};
    pub(crate) struct Reader;
    impl Reader {
        pub fn open(_: &Path, _: PackageLimits) -> Result<Self, PackageError> {
            Err(PackageError::UnsupportedPlatform)
        }
        pub fn json(&self, _: &str) -> Result<Vec<u8>, PackageError> {
            Err(PackageError::UnsupportedPlatform)
        }
        pub fn blob(&self, _: &ObjectRef) -> Result<(), PackageError> {
            Err(PackageError::UnsupportedPlatform)
        }
    }
}
pub(super) use platform::Reader;
