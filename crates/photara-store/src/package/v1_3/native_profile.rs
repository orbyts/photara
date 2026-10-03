//! Read-only assessment of the approved, controller-owned disposable APFS scope.
//! This is not general provider exclusion, native charge qualification, a lease,
//! a production registrar or an Accepted/Saved authority.
use rustix::fs::{Mode, OFlags, fstatfs, open, openat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const MAX_HOST_BYTES: u64 = 4 * 1024 * 1024;
type Pin = [u64; 2];
/// A refusal never authorizes fallback to a less restrictive profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    InvalidControllerScope,
    UnsafeNamespace,
    IdentityChanged,
    UnsupportedConfiguration,
    HostObservationUnavailable,
    ImageAssociationUnproved,
    ProviderScopeUnsupported,
    InvalidChargeObservation,
    RegisteredChargeExceeded,
}
type Result<T> = std::result::Result<T, Refusal>;
/// Existing experiment-controller binding. These assertions are independently
/// checked against handles and hdiutil; supplying them is not admission.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ControllerBinding {
    pub manifest_digest: String,
    pub mount_pin: Pin,
    pub image_pin: Pin,
    pub mount_source: String,
    pub generation: u64,
    pub image_association_asserted_by_controller: bool,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    fixture_version: u32,
    trial: String,
    scratch: PathBuf,
    scratch_pin: Pin,
    image: PathBuf,
    mount: PathBuf,
    nonce: String,
}
/// Observed configuration only. Controller generations are scoped experiment
/// continuity, not an event-aware production mount epoch or hardware attestation.
#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct Report {
    pub observer_revision: u32,
    pub policy_revision: u32,
    pub adapter_revision: String,
    pub os_version: String,
    pub os_build: String,
    pub boot_session: String,
    pub volume_uuid: String,
    pub filesystem: String,
    pub mount_flags: u64,
    pub mount_source: String,
    pub scratch_pin: Pin,
    pub image_pin: Pin,
    pub mount_pin: Pin,
    pub controller_generation: u64,
    pub controller_manifest_sha256: String,
    pub controller_binding_sha256: String,
    pub profile_scope: String,
    pub barrier_policy: String,
}
/// Current native allocation facts. Extent/allocated bytes are observations,
/// never a reservation of future APFS space or a permission to release charge.
#[derive(Debug, Serialize)]
pub struct AllocationUsage {
    pin: Pin,
    directory: bool,
    logical_extent: u64,
    allocated_bytes: u64,
}
impl AllocationUsage {
    #[must_use]
    pub fn pin(&self) -> Pin {
        self.pin
    }
}
/// Current measured footprint versus an independently registered experiment bound.
#[derive(Debug, Serialize)]
pub struct ChargeObservation {
    pub entries: usize,
    pub logical_bytes: u64,
    pub allocated_bytes: u64,
    pub rounded_required_charge: u64,
    pub registered_allowance: u64,
    pub quantum: u64,
}
/// Observe a pinned regular single-link file or directory, without reading data.
/// # Errors
/// Refuses unsafe types/hardlinks or checked allocated-byte overflow.
pub fn measure_allocation(file: &File) -> Result<AllocationUsage> {
    let m = file.metadata().map_err(|_| Refusal::UnsafeNamespace)?;
    if !(m.is_dir() || (m.is_file() && m.nlink() == 1)) {
        return Err(Refusal::UnsafeNamespace);
    }
    Ok(AllocationUsage {
        pin: [m.dev(), m.ino()],
        directory: m.is_dir(),
        logical_extent: m.len(),
        allocated_bytes: m
            .blocks()
            .checked_mul(512)
            .ok_or(Refusal::InvalidChargeObservation)?,
    })
}
/// Sum per-inode rounded max(logical extent, `st_blocks*512`), with no duplicate
/// identities or speculative compression/clone credits. This tests only the
/// current supplied footprint against a caller's pre-registered allowance.
/// Namespace completeness and original identity checks belong to the adapter.
/// # Errors
/// Refuses zero/invalid quantum, overflow, duplicate identities or insufficient bound.
pub fn check_charge(
    observations: &[AllocationUsage],
    quantum: u64,
    registered_allowance: u64,
) -> Result<ChargeObservation> {
    if quantum == 0 || !registered_allowance.is_multiple_of(quantum) {
        return Err(Refusal::InvalidChargeObservation);
    }
    let mut pins = std::collections::BTreeSet::new();
    let (mut logical_bytes, mut allocated_bytes, mut required) = (0u64, 0u64, 0u64);
    for observation in observations {
        if !pins.insert(observation.pin) {
            return Err(Refusal::InvalidChargeObservation);
        }
        logical_bytes = logical_bytes
            .checked_add(observation.logical_extent)
            .ok_or(Refusal::InvalidChargeObservation)?;
        allocated_bytes = allocated_bytes
            .checked_add(observation.allocated_bytes)
            .ok_or(Refusal::InvalidChargeObservation)?;
        let n = observation
            .logical_extent
            .max(observation.allocated_bytes)
            .max(1);
        let charge = n
            .checked_add(quantum - 1)
            .and_then(|n| n.checked_div(quantum))
            .and_then(|n| n.checked_mul(quantum))
            .ok_or(Refusal::InvalidChargeObservation)?;
        required = required
            .checked_add(charge)
            .ok_or(Refusal::InvalidChargeObservation)?;
    }
    if required > registered_allowance {
        return Err(Refusal::RegisteredChargeExceeded);
    }
    Ok(ChargeObservation {
        entries: observations.len(),
        logical_bytes,
        allocated_bytes,
        rounded_required_charge: required,
        registered_allowance,
        quantum,
    })
}
/// Retains actual handles. There is intentionally no conversion to qualified
/// storage or `RegisteredCooperativeLease` and no deserialization constructor.
/// ```compile_fail
/// use photara_store::package::v1_3::native_profile::DisposableAssessment;
/// let assessment = DisposableAssessment::default();
/// ```
pub struct DisposableAssessment {
    manifest_path: PathBuf,
    binding: ControllerBinding,
    report: Report,
    scratch: File,
    image: File,
    mount: File,
}
impl DisposableAssessment {
    #[must_use]
    pub fn report(&self) -> &Report {
        &self.report
    }
    /// Cheap per-effect check of the retained experiment namespace and mount.
    /// Full host configuration is reobserved at lease/run entry with `revalidate`.
    /// # Errors
    /// Refuses changed handles, controller records, permissions or mount facts.
    pub fn check_pins(&self) -> Result<()> {
        let scratch_path = self
            .manifest_path
            .parent()
            .ok_or(Refusal::InvalidControllerScope)?;
        let name = scratch_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(Refusal::InvalidControllerScope)?;
        let private = File::from(
            open("/private", DIRECTORY, Mode::empty()).map_err(|_| Refusal::UnsafeNamespace)?,
        );
        let tmp = File::from(
            openat(&private, "tmp", DIRECTORY, Mode::empty())
                .map_err(|_| Refusal::UnsafeNamespace)?,
        );
        let scratch = File::from(
            openat(&tmp, name, DIRECTORY, Mode::empty()).map_err(|_| Refusal::UnsafeNamespace)?,
        );
        let image = regular(&scratch, "fixture.sparseimage")?;
        let mount = File::from(
            openat(&scratch, "mount", DIRECTORY, Mode::empty())
                .map_err(|_| Refusal::UnsafeNamespace)?,
        );
        let facts = fstatfs(&mount).map_err(|_| Refusal::HostObservationUnavailable)?;
        let raw = small_file(&scratch, "manifest.json")?;
        let manifest: Manifest =
            serde_json::from_slice(&raw).map_err(|_| Refusal::InvalidControllerScope)?;
        let owner = serde_json::json!({"nonce":manifest.nonce,"manifest_digest":hash(&raw)});
        if pin(&scratch)? != self.report.scratch_pin
            || pin(&self.scratch)? != self.report.scratch_pin
            || pin(&image)? != self.report.image_pin
            || pin(&self.image)? != self.report.image_pin
            || pin(&mount)? != self.report.mount_pin
            || pin(&self.mount)? != self.report.mount_pin
            || scratch
                .metadata()
                .map_err(|_| Refusal::UnsafeNamespace)?
                .permissions()
                .mode()
                & 0o077
                != 0
            || hash(&raw) != self.report.controller_manifest_sha256
            || small_file(&scratch, "owner.json")? != encode(&owner)?
            || native_string(&facts.f_mntfromname)? != self.report.mount_source
            || native_string(&facts.f_fstypename)? != self.report.filesystem
            || u64::from(facts.f_flags) != self.report.mount_flags
        {
            return Err(Refusal::IdentityChanged);
        }
        Ok(())
    }
    /// Reobserve exact scope. Changed boot/mount/controller/profile/namespace
    /// coordinates invalidate the observation; no silent rebinding occurs.
    /// # Errors
    /// Returns the precise refusal or `IdentityChanged` for differing observations.
    pub fn revalidate(&self) -> Result<()> {
        let observed = assess_disposable_image(&self.manifest_path, self.binding.clone())?;
        if observed.report != self.report
            || pin(&self.scratch)? != self.report.scratch_pin
            || pin(&self.image)? != self.report.image_pin
            || pin(&self.mount)? != self.report.mount_pin
        {
            return Err(Refusal::IdentityChanged);
        }
        Ok(())
    }
}
fn pin(file: &File) -> Result<Pin> {
    let m = file.metadata().map_err(|_| Refusal::UnsafeNamespace)?;
    Ok([m.dev(), m.ino()])
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn encode(value: &impl Serialize) -> Result<Vec<u8>> {
    photara_core::canonical_json(
        &serde_json::to_value(value).map_err(|_| Refusal::InvalidControllerScope)?,
    )
    .map_err(|_| Refusal::InvalidControllerScope)
}
fn regular(parent: &File, name: &str) -> Result<File> {
    let f = File::from(
        openat(
            parent,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| Refusal::UnsafeNamespace)?,
    );
    let m = f.metadata().map_err(|_| Refusal::UnsafeNamespace)?;
    let p = parent.metadata().map_err(|_| Refusal::UnsafeNamespace)?;
    if !m.is_file() || m.nlink() != 1 || m.dev() != p.dev() || m.uid() != p.uid() {
        return Err(Refusal::UnsafeNamespace);
    }
    Ok(f)
}
fn small_file(parent: &File, name: &str) -> Result<Vec<u8>> {
    let f = regular(parent, name)?;
    let mut bytes = Vec::new();
    f.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| Refusal::UnsafeNamespace)?;
    if bytes.len() > 65536 {
        return Err(Refusal::InvalidControllerScope);
    }
    Ok(bytes)
}
fn native_string(chars: &[std::ffi::c_char]) -> Result<String> {
    let bytes = chars
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| u8::try_from(*c).map_err(|_| Refusal::HostObservationUnavailable))
        .collect::<Result<Vec<_>>>()?;
    String::from_utf8(bytes).map_err(|_| Refusal::HostObservationUnavailable)
}
/// Bounded, shell-free, read-only system observations. No credentials or command
/// output are logged. These fixed Apple tools perform no attach/detach or writes.
fn command(program: &str, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| Refusal::HostObservationUnavailable)?;
    let stdout = child
        .stdout
        .take()
        .ok_or(Refusal::HostObservationUnavailable)?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.take(MAX_HOST_BYTES + 1).read_to_end(&mut bytes);
        (result, bytes)
    });
    let writer = if let Some(bytes) = input {
        let mut stdin = child
            .stdin
            .take()
            .ok_or(Refusal::HostObservationUnavailable)?;
        let bytes = bytes.to_vec();
        Some(std::thread::spawn(move || stdin.write_all(&bytes)))
    } else {
        None
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    if let Some(writer) = writer
        && writer
            .join()
            .map_err(|_| Refusal::HostObservationUnavailable)?
            .is_err()
    {
        return Err(Refusal::HostObservationUnavailable);
    }
    let (result, bytes) = reader
        .join()
        .map_err(|_| Refusal::HostObservationUnavailable)?;
    if !success || result.is_err() || bytes.len() as u64 > MAX_HOST_BYTES {
        return Err(Refusal::HostObservationUnavailable);
    }
    Ok(bytes)
}
fn host_text(program: &str, args: &[&str]) -> Result<String> {
    String::from_utf8(command(program, args, None)?)
        .map(|s| s.trim().to_owned())
        .map_err(|_| Refusal::HostObservationUnavailable)
}
fn plist(program: &str, args: &[&str]) -> Result<Value> {
    let xml = command(program, args, None)?;
    let json = command(
        "/usr/bin/plutil",
        &["-convert", "json", "-o", "-", "-"],
        Some(&xml),
    )?;
    serde_json::from_slice(&json).map_err(|_| Refusal::HostObservationUnavailable)
}
fn identifier(value: &str) -> Result<String> {
    let id = uuid::Uuid::parse_str(value).map_err(|_| Refusal::HostObservationUnavailable)?;
    if id.is_nil() {
        return Err(Refusal::HostObservationUnavailable);
    }
    Ok(id.to_string())
}
fn association(info: &Value, image: &Path, mount: &Path, source: &str) -> Result<()> {
    let rows = info["images"]
        .as_array()
        .ok_or(Refusal::ImageAssociationUnproved)?;
    let matching = rows
        .iter()
        .filter(|row| row["image-path"].as_str() == image.to_str())
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Err(Refusal::ImageAssociationUnproved);
    }
    let entities = matching[0]["system-entities"]
        .as_array()
        .ok_or(Refusal::ImageAssociationUnproved)?;
    if entities
        .iter()
        .filter(|e| e["dev-entry"] == source && e["mount-point"].as_str() == mount.to_str())
        .count()
        != 1
    {
        return Err(Refusal::ImageAssociationUnproved);
    }
    Ok(())
}
fn supported(version: &str, build: &str, fs: &str, flags: u64) -> Result<()> {
    if version != "27.0.1"
        || build != "26A434"
        || fs != "apfs"
        || flags & 0x1000 == 0
        || flags & 1 != 0
    {
        return Err(Refusal::UnsupportedConfiguration);
    }
    Ok(())
}
/// Assess only the already controller-created private temporary image. It does
/// not create images, mutate files, exercise barriers, or admit real libraries.
/// Unknown/general provider scope has no success path through this function.
/// # Errors
/// Refuses unowned paths, unsafe files, unsupported configurations, missing native
/// facts, or an image/device relationship not independently reported by hdiutil.
#[expect(
    clippy::too_many_lines,
    reason = "Keep scoped native evidence checks together before constructing the private observation"
)]
pub fn assess_disposable_image(
    manifest_path: &Path,
    binding: ControllerBinding,
) -> Result<DisposableAssessment> {
    let scratch_path = manifest_path
        .parent()
        .ok_or(Refusal::ProviderScopeUnsupported)?;
    let name = scratch_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(Refusal::ProviderScopeUnsupported)?;
    if scratch_path.parent() != Some(Path::new("/private/tmp"))
        || !name.starts_with("photara-ps2-remount-")
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || manifest_path.file_name().and_then(|s| s.to_str()) != Some("manifest.json")
    {
        return Err(Refusal::ProviderScopeUnsupported);
    }
    let private = File::from(
        open("/private", DIRECTORY, Mode::empty()).map_err(|_| Refusal::UnsafeNamespace)?,
    );
    let tmp = File::from(
        openat(&private, "tmp", DIRECTORY, Mode::empty()).map_err(|_| Refusal::UnsafeNamespace)?,
    );
    let scratch = File::from(
        openat(&tmp, name, DIRECTORY, Mode::empty()).map_err(|_| Refusal::UnsafeNamespace)?,
    );
    if scratch
        .metadata()
        .map_err(|_| Refusal::UnsafeNamespace)?
        .permissions()
        .mode()
        & 0o077
        != 0
    {
        return Err(Refusal::UnsafeNamespace);
    }
    let raw = small_file(&scratch, "manifest.json")?;
    let manifest: Manifest =
        serde_json::from_slice(&raw).map_err(|_| Refusal::InvalidControllerScope)?;
    if encode(&manifest)? != raw
        || manifest.fixture_version != 1
        || manifest.trial != "disk-image-remount"
        || manifest.scratch != scratch_path
        || manifest.scratch_pin != pin(&scratch)?
        || manifest.image != scratch_path.join("fixture.sparseimage")
        || manifest.mount != scratch_path.join("mount")
        || binding.manifest_digest != hash(&raw)
        || binding.generation == 0
        || !binding.image_association_asserted_by_controller
        || identifier(&manifest.nonce)? != manifest.nonce
    {
        return Err(Refusal::InvalidControllerScope);
    }
    let owner = serde_json::json!({"nonce":manifest.nonce,"manifest_digest":hash(&raw)});
    if small_file(&scratch, "owner.json")? != encode(&owner)? {
        return Err(Refusal::InvalidControllerScope);
    }
    let image = regular(&scratch, "fixture.sparseimage")?;
    let mount = File::from(
        openat(&scratch, "mount", DIRECTORY, Mode::empty())
            .map_err(|_| Refusal::UnsafeNamespace)?,
    );
    let facts = fstatfs(&mount).map_err(|_| Refusal::HostObservationUnavailable)?;
    let source = native_string(&facts.f_mntfromname)?;
    if binding.image_pin != pin(&image)?
        || binding.mount_pin != pin(&mount)?
        || binding.mount_source != source
        || pin(&mount)?[0] == pin(&scratch)?[0]
    {
        return Err(Refusal::IdentityChanged);
    }
    let os_version = host_text("/usr/bin/sw_vers", &["-productVersion"])?;
    let os_build = host_text("/usr/bin/sw_vers", &["-buildVersion"])?;
    let filesystem = native_string(&facts.f_fstypename)?;
    let mount_flags = u64::from(facts.f_flags);
    supported(&os_version, &os_build, &filesystem, mount_flags)?;
    let boot_session = identifier(&host_text(
        "/usr/sbin/sysctl",
        &["-n", "kern.bootsessionuuid"],
    )?)?;
    association(
        &plist("/usr/bin/hdiutil", &["info", "-plist"])?,
        &manifest.image,
        &manifest.mount,
        &source,
    )?;
    let disk = plist("/usr/sbin/diskutil", &["info", "-plist", &source])?;
    if disk["MountPoint"].as_str() != manifest.mount.to_str() || disk["DeviceNode"] != source {
        return Err(Refusal::ImageAssociationUnproved);
    }
    let volume_uuid = identifier(
        disk["VolumeUUID"]
            .as_str()
            .ok_or(Refusal::HostObservationUnavailable)?,
    )?;
    let report = Report {
        observer_revision: 1,
        policy_revision: 1,
        adapter_revision: "photara-disposable-apfs-assessment-v1".into(),
        os_version,
        os_build,
        boot_session,
        volume_uuid,
        filesystem,
        mount_flags,
        mount_source: source,
        scratch_pin: pin(&scratch)?,
        image_pin: pin(&image)?,
        mount_pin: pin(&mount)?,
        controller_generation: binding.generation,
        controller_manifest_sha256: hash(&raw),
        controller_binding_sha256: hash(&encode(&binding)?),
        profile_scope: "controller-owned-disposable-image; no general provider exclusion".into(),
        barrier_policy: "fsync files; fsync affected directories; F_FULLFSYNC same-device after namespace barriers; unsupported/error refuses".into(),
    };
    Ok(DisposableAssessment {
        manifest_path: manifest_path.to_path_buf(),
        binding,
        report,
        scratch,
        image,
        mount,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn exact_configuration_refuses_unknown_and_weaker_facts() {
        assert!(supported("27.0.1", "26A434", "apfs", 0x1000).is_ok());
        for (version, build, fs, flags) in [
            ("27.0.2", "26A434", "apfs", 0x1000),
            ("27.0.1", "unknown", "apfs", 0x1000),
            ("27.0.1", "26A434", "nfs", 0x1000),
            ("27.0.1", "26A434", "apfs", 0),
            ("27.0.1", "26A434", "apfs", 0x1001),
        ] {
            assert_eq!(
                supported(version, build, fs, flags),
                Err(Refusal::UnsupportedConfiguration)
            );
        }
    }
    #[test]
    fn image_association_is_exact_not_a_controller_boolean() {
        let image = Path::new("/private/tmp/photara-ps2-remount-test/fixture.sparseimage");
        let mount = Path::new("/private/tmp/photara-ps2-remount-test/mount");
        let row = json!({"image-path":image,"system-entities":[{"dev-entry":"/dev/disk9s1","mount-point":mount}]});
        assert!(
            association(
                &json!({"images":[row.clone()]}),
                image,
                mount,
                "/dev/disk9s1"
            )
            .is_ok()
        );
        assert!(
            association(
                &json!({"images":[row.clone()]}),
                image,
                mount,
                "/dev/disk8s1"
            )
            .is_err()
        );
        assert!(
            association(
                &json!({"images":[row.clone(),row]}),
                image,
                mount,
                "/dev/disk9s1"
            )
            .is_err()
        );
        assert!(association(&json!({"images":[]}), image, mount, "/dev/disk9s1").is_err());
    }
    #[test]
    fn arbitrary_path_cannot_request_disposable_scope() {
        let binding = ControllerBinding {
            manifest_digest: "0".repeat(64),
            mount_pin: [1, 2],
            image_pin: [1, 3],
            mount_source: "/dev/disk9s1".into(),
            generation: 1,
            image_association_asserted_by_controller: true,
        };
        assert!(matches!(
            assess_disposable_image(Path::new("/Users/example/project/manifest.json"), binding),
            Err(Refusal::ProviderScopeUnsupported)
        ));
    }
    #[test]
    fn measured_charge_refuses_understatement_overflow_and_duplicate_pins() {
        let a = AllocationUsage {
            pin: [1, 2],
            directory: false,
            logical_extent: 5,
            allocated_bytes: 8192,
        };
        assert!(matches!(
            check_charge(&[a], 4096, 4096),
            Err(Refusal::RegisteredChargeExceeded)
        ));
        let a = AllocationUsage {
            pin: [1, 2],
            directory: false,
            logical_extent: 5,
            allocated_bytes: 8192,
        };
        let observation = check_charge(&[a], 4096, 8192).unwrap();
        assert_eq!(observation.rounded_required_charge, 8192);
        let a = AllocationUsage {
            pin: [1, 2],
            directory: false,
            logical_extent: 0,
            allocated_bytes: 0,
        };
        let b = AllocationUsage {
            pin: [1, 2],
            directory: true,
            logical_extent: 0,
            allocated_bytes: 0,
        };
        assert!(matches!(
            check_charge(&[a, b], 4096, 8192),
            Err(Refusal::InvalidChargeObservation)
        ));
        let a = AllocationUsage {
            pin: [1, 2],
            directory: false,
            logical_extent: u64::MAX,
            allocated_bytes: 0,
        };
        assert!(matches!(
            check_charge(&[a], 4096, 8192),
            Err(Refusal::InvalidChargeObservation)
        ));
        assert!(matches!(
            check_charge(&[], 0, 0),
            Err(Refusal::InvalidChargeObservation)
        ));
    }
}
