//! Bounded in-memory semantic planning for the disposable Graph integration.
#[allow(
    clippy::wildcard_imports,
    reason = "Companion shares disposable Store primitives"
)]
use super::*;
const LIMIT: u64 = 8 * 1024 * 1024;
pub(super) struct Preview {
    pub(super) head: Head,
    pub(super) end: u64,
    base: u64,
    objects: BTreeMap<u64, Vec<u8>>,
}
impl Preview {
    pub(super) fn put(&mut self, bytes: Vec<u8>) -> Result<Ref> {
        let end = self
            .end
            .checked_add(bytes.len() as u64)
            .ok_or("preview overflow")?;
        ensure(end - self.base <= LIMIT, "bounded semantic preview memory")?;
        let r = Ref {
            offset: self.end,
            len: bytes.len() as u64,
            sha: hash(&bytes),
        };
        self.objects.insert(self.end, bytes);
        self.end = end;
        Ok(r)
    }
    pub(super) fn bytes(&self, r: &Ref) -> Option<&Vec<u8>> {
        self.objects.get(&r.offset)
    }
    pub(super) fn tail(&self) -> Vec<u8> {
        self.objects.values().flatten().copied().collect()
    }
    pub(super) fn verify_present(&self, source: &mut Store) -> Result<usize> {
        let tail = self.tail();
        let len = source.file.metadata().map_err(|e| e.to_string())?.len();
        ensure(
            len >= self.base && len <= self.end,
            "source tail outside original plan",
        )?;
        let present = usize::try_from(len - self.base).map_err(|e| e.to_string())?;
        let mut observed = vec![0; present];
        source
            .file
            .seek(SeekFrom::Start(self.base))
            .and_then(|_| source.file.read_exact(&mut observed))
            .map_err(|e| e.to_string())?;
        ensure(
            observed == tail[..present],
            "source tail differs from original plan",
        )?;
        Ok(present)
    }
    pub(super) fn persist(self, source: &mut Store, partial: bool) -> Result<()> {
        let present = self.verify_present(source)?;
        let tail = self.tail();
        let len = self.base + present as u64;
        let remaining = &tail[present..];
        let write = if partial {
            &remaining[..remaining.len() / 2]
        } else {
            remaining
        };
        source
            .file
            .seek(SeekFrom::Start(len))
            .and_then(|_| source.file.write_all(write))
            .and_then(|()| source.file.sync_all())
            .map_err(|e| e.to_string())?;
        source.stats.object_write_bytes += write.len() as u64;
        source.stats.sync_calls += 1;
        if partial {
            return Err("injected partial original source tail".into());
        }
        Ok(())
    }
}
impl Store {
    pub(super) fn begin_graph_preview(&mut self, head: Head, end: u64) -> Result<()> {
        ensure(
            self.preview.is_none() && self.staging.is_none() && self.packed.is_none(),
            "plain-source preview only",
        )?;
        self.preview = Some(Preview {
            head,
            end,
            base: end,
            objects: BTreeMap::new(),
        });
        Ok(())
    }
    pub(super) fn take_graph_preview(&mut self) -> Result<Preview> {
        self.preview.take().ok_or("missing Graph preview".into())
    }
    pub(super) fn planned_bytes(&mut self, r: &Ref) -> Result<Vec<u8>> {
        if let Some(bytes) = self.preview.as_ref().and_then(|p| p.bytes(r)) {
            return Ok(bytes.clone());
        }
        let mut bytes = vec![0; usize::try_from(r.len).map_err(|e| e.to_string())?];
        self.file
            .seek(SeekFrom::Start(r.offset))
            .and_then(|_| self.file.read_exact(&mut bytes))
            .map_err(|e| e.to_string())?;
        ensure(hash(&bytes) == r.sha, "planned source hash")?;
        Ok(bytes)
    }
}
