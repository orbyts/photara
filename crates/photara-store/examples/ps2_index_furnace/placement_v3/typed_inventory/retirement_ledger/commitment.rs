//! Disposable compact original
//! relocation commitments. Not a permanent encoding or a second selector.
//! This module only proves original destination bytes; semantic/ownership
//! closure, exact control phases, admission, and retirement remain caller duties.
#[allow(clippy::wildcard_imports, reason = "Actual fixture commitment child")]
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Suffix {
    data: bool,
    pack: u64,
    dev: u64,
    ino: u64,
    original_end: u64,
    final_end: u64,
    original_sha: String,
    // Binds the existing opaque recipe Prefix, without changing its format.
    prefix_commitment: String,
    frames: u64,
    bytes: u64,
    // Covers exact length/key/body framing, including unreachable intermediate
    // records. It is not merely the hash of selected reachable objects.
    framed_sha: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Compact {
    version: u8,
    data: Suffix,
    meta: Suffix,
    typed_plan_sha: String,
}

fn plan_sha(plan: &TypedPlan) -> Result<String> {
    Ok(hash(
        &serde_json::to_vec(&(&plan.data, &plan.meta, &plan.continuation)).map_err(error)?,
    ))
}

impl Suffix {
    fn shape(&self, data: bool, prefix: &recipe::Prefix) -> Result<()> {
        ensure(
            self.data == data
                && self.pack == prefix.pack
                && self.original_end == prefix.end
                && self.dev == prefix.dev
                && self.ino == prefix.ino
                && self.original_sha == prefix.sha
                && self.original_end <= self.final_end
                && self.final_end <= PACK
                && self.bytes == self.final_end - self.original_end
                && self.frames <= (TX_OBJECTS * 130) as u64
                && self.original_sha.len() == 64
                && self.framed_sha.len() == 64
                && self.prefix_commitment == hash(&serde_json::to_vec(prefix).map_err(error)?),
            "compact destination commitment shape",
        )
    }

    fn expected(
        prefix: &recipe::Prefix,
        plan: &WritePlan,
        data: bool,
    ) -> Result<(u64, u64, Vec<u8>)> {
        let mut segments = recipe::segments(prefix, plan, data)?;
        ensure(segments.len() == 1, "compact relocation remains same-tip")?;
        let segment = segments.pop().ok_or("compact suffix segment")?;
        ensure(
            segment.0 == plan.pack
                && checked(segment.1, segment.2.len() as u64)? == plan.end
                && segment.2.len() as u64 == plan.bytes,
            "compact exact suffix extent",
        )?;
        Ok(segment)
    }

    fn capture(
        f: &mut Fixture,
        prefix: &recipe::Prefix,
        plan: &WritePlan,
        data: bool,
    ) -> Result<Self> {
        let (pack, original_end, framed) = Self::expected(prefix, plan, data)?;
        let arena = if data { &f.data } else { &f.meta };
        ensure(
            arena.pack == pack && arena.end == original_end,
            "compact capture precedes original append effects",
        )?;
        let path = arena.path(pack);
        let (mut file, metadata) = open_destination(&path)?;
        ensure(
            metadata.len() == original_end
                && metadata.dev() == prefix.dev
                && metadata.ino() == prefix.ino,
            "compact original complete tip and witness",
        )?;
        let mut bytes = vec![0; usize::try_from(original_end).map_err(error)?];
        file.read_exact(&mut bytes).map_err(error)?;
        ensure(
            file.metadata().map_err(error)?.len() == original_end && hash(&bytes) == prefix.sha,
            "compact original changed or differs from original prefix",
        )?;
        f.c.recipe_prefix_read_bytes += original_end;
        let proof = Self {
            data,
            pack,
            dev: metadata.dev(),
            ino: metadata.ino(),
            original_end,
            final_end: plan.end,
            original_sha: hash(&bytes),
            prefix_commitment: hash(&serde_json::to_vec(prefix).map_err(error)?),
            frames: plan.writes.len() as u64,
            bytes: plan.bytes,
            framed_sha: hash(&framed),
        };
        proof.shape(data, prefix)?;
        Ok(proof)
    }

    fn matches_plan(
        &self,
        prefix: &recipe::Prefix,
        plan: &WritePlan,
        data: bool,
    ) -> Result<Vec<u8>> {
        self.shape(data, prefix)?;
        let (pack, start, bytes) = Self::expected(prefix, plan, data)?;
        ensure(
            pack == self.pack
                && start == self.original_end
                && plan.end == self.final_end
                && plan.bytes == self.bytes
                && plan.writes.len() as u64 == self.frames
                && hash(&bytes) == self.framed_sha,
            "rebuild changed original suffix commitment",
        )?;
        Ok(bytes)
    }

    fn restore(&self, bytes: &[u8]) -> Result<WritePlan> {
        ensure(
            count_frames(bytes, self.data)? == self.frames,
            "compact restore frame count",
        )?;
        let mut writes = vec![];
        let mut position = 0usize;
        let header = if self.data { 12 } else { 4 };
        while position < bytes.len() {
            let length =
                u32::from_le_bytes(bytes[position..position + 4].try_into().map_err(error)?)
                    as usize;
            let key = if self.data {
                Some(u64::from_le_bytes(
                    bytes[position + 4..position + 12]
                        .try_into()
                        .map_err(error)?,
                ))
            } else {
                None
            };
            let body = bytes[position + header..position + header + length].to_vec();
            writes.push((
                key,
                PRef {
                    pack: self.pack,
                    offset: checked(self.original_end, (position + header) as u64)?,
                    len: length as u64,
                    sha: hash(&body),
                },
                body,
            ));
            position += header + length;
        }
        Ok(WritePlan {
            pack: self.pack,
            end: self.final_end,
            bytes: self.bytes,
            writes,
        })
    }

    /// Pure read-only proof, before dispatch cleanup or any replay mutation.
    /// Partial suffixes require exact regenerated bytes; a completed suffix can
    /// be authenticated independently after the original source is unlinked.
    fn verify(&self, f: &mut Fixture, partial_expected: Option<&[u8]>) -> Result<Vec<u8>> {
        let arena = if self.data { &f.data } else { &f.meta };
        let (mut file, metadata) = open_destination(&arena.path(self.pack))?;
        ensure(
            metadata.dev() == self.dev
                && metadata.ino() == self.ino
                && metadata.len() >= self.original_end
                && metadata.len() <= self.final_end,
            "compact destination witness or unknown tail",
        )?;
        if partial_expected.is_none() {
            ensure(
                metadata.len() == self.final_end,
                "compact completed destination truncated",
            )?;
        }
        let mut bytes = vec![0; usize::try_from(metadata.len()).map_err(error)?];
        file.read_exact(&mut bytes).map_err(error)?;
        let fresh = file.metadata().map_err(error)?;
        ensure(
            fresh.len() == metadata.len() && fresh.nlink() == 1,
            "compact destination changed during proof",
        )?;
        let split = usize::try_from(self.original_end).map_err(error)?;
        ensure(
            hash(&bytes[..split]) == self.original_sha,
            "compact original prefix corruption",
        )?;
        let suffix = &bytes[split..];
        if let Some(expected) = partial_expected {
            ensure(
                expected.len() as u64 == self.bytes
                    && hash(expected) == self.framed_sha
                    && suffix
                        == expected
                            .get(..suffix.len())
                            .ok_or("compact suffix extent")?,
                "compact partial suffix differs from original regenerated bytes",
            )?;
        } else {
            ensure(
                hash(suffix) == self.framed_sha && count_frames(suffix, self.data)? == self.frames,
                "compact complete framed suffix corruption",
            )?;
        }
        f.c.recipe_prefix_read_bytes += self.original_end;
        f.c.recipe_suffix_read_bytes += suffix.len() as u64;
        Ok(suffix.to_vec())
    }
}

fn count_frames(mut bytes: &[u8], data: bool) -> Result<u64> {
    let mut count = 0;
    let header = if data { 12 } else { 4 };
    while !bytes.is_empty() {
        let length_bytes: [u8; 4] = bytes
            .get(..4)
            .ok_or("compact partial frame length")?
            .try_into()
            .map_err(error)?;
        let length = u32::from_le_bytes(length_bytes) as usize;
        ensure(
            length as u64 <= PACK - header as u64 && (data || length <= 4092),
            "compact bounded framed body",
        )?;
        bytes = bytes
            .get(header + length..)
            .ok_or("compact partial framed body")?;
        count += 1;
        ensure(
            count <= (TX_OBJECTS * 130) as u64,
            "compact frame count bound",
        )?;
    }
    Ok(count)
}

fn open_destination(path: &Path) -> Result<(File, fs::Metadata)> {
    let named = fs::symlink_metadata(path).map_err(error)?;
    ensure(
        named.is_file() && named.nlink() == 1 && named.len() <= PACK,
        "compact private bounded destination",
    )?;
    let fd = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(error)?;
    let file = File::from(fd);
    let opened = file.metadata().map_err(error)?;
    ensure(
        opened.is_file()
            && opened.nlink() == 1
            && opened.dev() == named.dev()
            && opened.ino() == named.ino()
            && opened.len() == named.len(),
        "compact destination name/open identity changed",
    )?;
    Ok((file, opened))
}

impl Compact {
    pub(super) fn bytes(&self) -> (u64, u64) {
        (self.data.bytes, self.meta.bytes)
    }
    pub(super) fn capture(
        f: &mut Fixture,
        data: &recipe::Prefix,
        meta: &recipe::Prefix,
        plan: &TypedPlan,
    ) -> Result<Self> {
        let proof = Self {
            version: 1,
            data: Suffix::capture(f, data, &plan.data, true)?,
            meta: Suffix::capture(f, meta, &plan.meta, false)?,
            typed_plan_sha: plan_sha(plan)?,
        };
        proof.validate(data, meta)?;
        proof.validate_target(&plan.continuation)?;
        Ok(proof)
    }
    pub(super) fn validate(&self, data: &recipe::Prefix, meta: &recipe::Prefix) -> Result<()> {
        ensure(
            self.version == 1
                && self.typed_plan_sha.len() == 64
                && serde_json::to_vec(self).map_err(error)?.len() <= 4096,
            "bounded compact relocation descriptor",
        )?;
        self.data.shape(true, data)?;
        self.meta.shape(false, meta)
    }
    pub(super) fn verify_rebuild(
        &self,
        f: &mut Fixture,
        data: &recipe::Prefix,
        meta: &recipe::Prefix,
        plan: &TypedPlan,
    ) -> Result<()> {
        self.validate(data, meta)?;
        self.validate_target(&plan.continuation)?;
        ensure(
            plan_sha(plan)? == self.typed_plan_sha,
            "compact typed plan changed",
        )?;
        let a = self.data.matches_plan(data, &plan.data, true)?;
        let b = self.meta.matches_plan(meta, &plan.meta, false)?;
        self.data.verify(f, Some(&a))?;
        self.meta.verify(f, Some(&b))?;
        Ok(())
    }
    pub(super) fn validate_target(&self, target: &Continuation) -> Result<()> {
        ensure(
            self.data.pack == target.data_pack
                && self.data.final_end == target.data_end
                && self.meta.pack == target.meta_pack
                && self.meta.final_end == target.meta_end,
            "compact original target coordinates",
        )
    }
    pub(super) fn verify_complete(
        &self,
        f: &mut Fixture,
        data: &recipe::Prefix,
        meta: &recipe::Prefix,
        target: &Continuation,
    ) -> Result<()> {
        self.validate(data, meta)?;
        self.validate_target(target)?;
        let data = self.data.restore(&self.data.verify(f, None)?)?;
        let meta = self.meta.restore(&self.meta.verify(f, None)?)?;
        ensure(
            plan_sha(&TypedPlan {
                data,
                meta,
                continuation: target.clone(),
            })? == self.typed_plan_sha,
            "completed compact typed plan commitment changed",
        )
    }
}
