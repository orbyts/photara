# PS2 disposable macOS syscall observation — 2026-09-16

**Observed, not qualified:** the explicit test-only
[probe](../../crates/photara-store/tests/package_planning/macos_probe.rs) succeeded
on macOS 27.0 build 26A428, architecture `aarch64`. `fstatfs` on its new temporary
package reported `apfs`, mount flags `76583040`, and `MNT_LOCAL` set. The probe
accepts no user-supplied path. It creates synthetic package bytes under a fresh
temporary root and removes only that test-owned root at completion. No live
package, installed app, provider account, migration or production adapter is used.

## Reproduce

```sh
cargo test -p photara-store --test package_v1_1 observe_local_barriers_and_exclusive_rename -- --ignored --nocapture
```

It is macOS-only and ignored in ordinary runs. It prints structured platform facts
and an ordered trace with each selected operation's errno (`null` means success).
Failed barriers are recorded, not silently replaced with another primitive. The
test intentionally can continue observing this disposable fixture after a barrier
error; a production writer may not infer permission to acknowledge from that.

## Observed trace and checks

The package planner produced four new immutable files. The following is the
observed order; each repeated row expands in ascending file-index order:

| Order | Operation(s) | Observed outcome |
| --- | --- | --- |
| 1 | Materialize synthetic base, open pinned root, query `fstatfs` | Success; local APFS |
| 2 | Create-exclusive/write immutable 0 temporary; `fsync(file)`; `F_FULLFSYNC(file)`; exclusive rename; `fsync(parent)`; `F_FULLFSYNC(parent)` | Every call succeeded |
| 3 | Create/write a different contender; `fsync`; `F_FULLFSYNC`; exclusive rename to occupied immutable 0 | Barriers succeeded; rename returned `EEXIST` (17). Both contender bytes and original destination bytes remained exact |
| 4 | Repeat the immutable create/write, two file barriers, exclusive rename and two parent barriers for files 1, 2 and 3 | Every call succeeded |
| 5 | Create-exclusive/write HEAD temporary; `fsync`; `F_FULLFSYNC`; `renameat` replacement of HEAD | Every call succeeded |
| 6 | `fsync(package root)`; `F_FULLFSYNC(package root)` | Both directory calls succeeded |
| 7 | Independent package-reader validation and exact HEAD/commit comparison | Complete candidate closure accepted; expected commit ID and exact HEAD bytes matched |

Parent directories were on the same device as the pinned root; the root's
device/inode identity was unchanged at the final check. The existing safe `rustix`
APIs wrap `fcntl(F_FULLFSYNC)` and exclusive rename, so no unsafe Rust or new
dependency was added. On this supported macOS, `renameat_with(NOREPLACE)` uses
`renameatx_np(RENAME_EXCL)`; ordinary HEAD replacement uses `renameat`.

## Stronger current SDK documentation, still bounded evidence

The installed Apple macOS SDK 27.0 (build 26A425) contains a more current `fcntl(2)`
manual than the older [online archive](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fcntl.2.html).
Its `F_FULLFSYNC` section explicitly includes APFS and states that draining the
device queue also persists data previously fsynced on that same device. Its
`F_BARRIERFSYNC` section distinguishes ordering from persistence at return. This
supports investigating a same-device multi-file flush protocol; it is stronger
than merely observing a successful syscall, but not a universal hardware promise.

Source: `usr/share/man/man2/fcntl.2` below `xcrun --show-sdk-path`, SHA-256
`9b16ec920ad681d1af2256b2c76d408c7b8506b7cad13aa00d035ba4385787a7`.
The SDK `sys/mount.h` defines `MNT_LOCAL` as `0x00001000`; its `rename(2)` manual
documents `RENAME_EXCL` returning `EEXIST` for an occupied destination. The historical
[fsync guidance](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)
and current [disk-write guidance](https://developer.apple.com/documentation/xcode/reducing-disk-writes)
remain relevant to buffered writes and residual sudden-power-loss risk.

## What remains unresolved

Successful `F_FULLFSYNC` on these directory descriptors is an observed capability,
not proof that every namespace dependency survives arbitrary interruption. This
probe does not inject lost/reordered writes, kernel crash, remount, disconnect or
power failure. It does not separately establish APFS directory-entry persistence
semantics for the entire object/commit/HEAD protocol, nor hardware behavior across
device classes. Initial fixture setup uses ordinary test materialization; it is
not itself a qualified recovery-base publication procedure.

`MNT_LOCAL` and APFS do not establish provider exclusion or absence of other writers.
Provider management was **unassessed**. There is no lock-contention, adversarial
path substitution, syscall-failure matrix or concurrency proof in this probe.
Descriptor-relative operations here use known paths inside a private fixture;
this is not a replacement for the production component-walk/pin/lease design.

The printed report therefore fixes `qualified=false`, `power_loss_tested=false`,
`directory_persistence_ordering="not demonstrated"`, and
`provider_exclusion="unassessed"`. The remaining
[qualification plan](PS2_MACOS_STORAGE_QUALIFICATION.md) and
[writer-admission decision](PS2_WRITER_ADMISSION_PROPOSAL.md) still gate production.
No new user decision is needed for further safe disposable tests; disruptive
hardware/power-loss experiments require a separately approved environment.
