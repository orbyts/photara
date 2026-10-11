//! One closed, same-device control installation group. No journal/payload grouping.
use super::{
    File, IoResult, Mode, OFlags, Pin, Read, SelectedStage, Write, barrier, canonical,
    fcntl_fullfsync, fsync, hash, invalid, openat, pin, read, regular, renameat,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Step {
    FileSync(usize),
    DirectorySync,
    DeviceSync,
    Rename,
    PublishedBarrier,
}
struct Installed {
    name: String,
    file: File,
    identity: Pin,
}
impl Installed {
    fn open(root: &File, name: String, bytes: &[u8]) -> IoResult<Self> {
        let file = match regular(root, &name, true) {
            Ok(mut file) => {
                // Read the retained descriptor, not a second pathname lookup.
                let mut actual = Vec::new();
                std::io::Read::by_ref(&mut file)
                    .take(bytes.len() as u64 + 1)
                    .read_to_end(&mut actual)?;
                if actual != bytes {
                    return Err(invalid());
                }
                file
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut file = File::from(openat(
                    root,
                    name.as_str(),
                    OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::RDWR
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )?);
                file.write_all(bytes)?;
                file
            }
            Err(error) => return Err(error),
        };
        let identity = pin(&file)?;
        let installed = Self {
            name,
            file,
            identity,
        };
        installed.check(root)?;
        Ok(installed)
    }
    fn check(&self, root: &File) -> IoResult<()> {
        if self.identity[0] != pin(root)?[0]
            || pin(&self.file)? != self.identity
            || pin(&regular(root, &self.name, false)?)? != self.identity
        {
            return Err(invalid());
        }
        Ok(())
    }
}
fn check(root: &File, root_pin: Pin, files: &[Installed]) -> IoResult<()> {
    if pin(root)? != root_pin {
        return Err(invalid());
    }
    for file in files {
        file.check(root)?;
    }
    Ok(())
}

pub(super) fn publish(
    root: &File,
    expected: &serde_json::Value,
    stage: &SelectedStage,
    mut event: impl FnMut(Step, bool) -> IoResult<()>,
) -> IoResult<()> {
    let _timing = super::Timing::new("select-group");
    let root_pin = pin(root)?;
    if read(root, "HEAD.json")? != canonical(expected) {
        return Err(invalid());
    }
    let mut files = Vec::with_capacity(stage.loose.len() + 2);
    for (sha, bytes) in &stage.loose {
        files.push(Installed::open(root, format!("{sha}.control"), bytes)?);
    }
    let commit = canonical(&stage.commit);
    files.push(Installed::open(
        root,
        format!("{}.commit", hash(&commit)),
        &commit,
    )?);
    files.push(Installed::open(
        root,
        "HEAD.next".into(),
        &canonical(&stage.head),
    )?);
    for (index, file) in files.iter().enumerate() {
        event(Step::FileSync(index), false)?;
        check(root, root_pin, std::slice::from_ref(file))?;
        {
            let _timing = super::Timing::new("select-file-fsync");
            fsync(&file.file)?;
        }
        event(Step::FileSync(index), true)?;
    }
    // Every new name and every file fsync precedes the device-wide flush.
    event(Step::DirectorySync, false)?;
    check(root, root_pin, &files)?;
    {
        let _timing = super::Timing::new("select-directory-fsync");
        fsync(root)?;
    }
    event(Step::DirectorySync, true)?;
    event(Step::DeviceSync, false)?;
    check(root, root_pin, &files)?;
    {
        let _timing = super::Timing::new("select-device-fullsync");
        fcntl_fullfsync(root)?;
    }
    event(Step::DeviceSync, true)?;
    event(Step::Rename, false)?;
    check(root, root_pin, &files)?;
    if read(root, "HEAD.json")? != canonical(expected) {
        return Err(invalid());
    }
    renameat(root, "HEAD.next", root, "HEAD.json")?;
    event(Step::Rename, true)?;
    event(Step::PublishedBarrier, false)?;
    barrier(root)?;
    event(Step::PublishedBarrier, true)
}

#[cfg(test)]
#[path = "native_select_tests.rs"]
mod tests;
