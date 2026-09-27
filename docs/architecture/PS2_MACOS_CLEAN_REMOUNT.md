# PS2 disposable APFS clean-remount observation

On 2026-09-27 Suhail explicitly authorized a private temporary disk-image
clean-remount trial on this Mac, with no power interruption. The trial passed
on macOS 27.0 build 26A428, arm64. This is **clean-remount evidence only**;
successful detach can flush outstanding writes. Abrupt power/kernel/storage
interruption, provider exclusion and production storage qualification remain open.

The existing split-phase harness was used unchanged from
`a062a6e1baaffa89c3dc8bf90b8d0aa45ad63ec9`. A private mode-0700 directory,
`/private/tmp/photara-ps2-remount-n755j3yr`, owns one 128 MiB APFS sparse image,
the manifest and all control logs. The initial sandboxed image creation failed
with `Device not configured` before creating an image. The explicitly authorized
scoped rerun succeeded. No live package or existing volume was used.

1. Create the image with `hdiutil create -size 128m -fs APFS -type SPARSE`,
   then attach it with `-nobrowse` at the owned `mount` child directory.
2. Run `planning::macos_remount_harness::remount_dry_run_manifest` to create
   its exact private manifest. Bind device/inode and mount source to the
   image's returned attach record; the mount is a different device from scratch.
3. Run `remount_phase` in separate processes for `prepare` and `publish` with
   binding generation 1. Both pass the real file/directory/full-flush path.
4. Verify `hdiutil info -plist` associates the exact image and mount with
   `/dev/disk6`, then cleanly detach that device. Reattach the same image and
   supply freshly observed binding generation 2.
5. Run the independent `verify` phase. It validates the exact candidate HEAD,
   complete package closure, original intent and matching receipt. The HEAD
   SHA-256 equals the hash recorded before detach.
6. Recheck image/device association, cleanly detach again and verify that the
   image is absent from `hdiutil info`. Its temporary mountpoint is back on the
   host filesystem. The detached image and logs are retained in private scratch.

The [controller evidence](verification/ps2-macos-clean-remount-20260927.json)
records both bindings, source and raw-output hashes, exit outcomes and exact
reader result. The harness deliberately continues to report
`remount_tested: false`: it does not operate mount tools or independently attest
to the controller's detach/attach. The separate controller observation records
the actual clean remount. Both retain `qualified: false` and no `Saved` claim.

This successful publication/remount case does not replace the separate
seven-cut process matrix, exercise remount at every publication cut, qualify
the actual-v3 backend or establish hardware power-loss durability. No permanent
format or production reader/writer is enabled by this result.

Two additional controlled process-exit cases passed on the same Mac using the
unchanged harness and separate private 128 MiB APFS images. The
[cut/remount evidence](verification/ps2-macos-clean-remount-cuts-20260927.json)
records exit 81 immediately before `publish HEAD.json`, followed by a clean
detach/attach and exact old HEAD with valid closure; exit 82 immediately after
that rename likewise reopens the exact candidate HEAD with valid closure.
Both generation-2 verifications found the original intent and no receipt.
Both HEAD hashes were unchanged across remount. The controller checked each
exact image/device association before detach; both final clean detaches passed,
both images were absent from `hdiutil info`, and detached images and logs remain
in `/private/tmp/photara-ps2-remount-x98sheak` and
`/private/tmp/photara-ps2-remount-ig6wqv83`, respectively. These are exactly two
process exits followed by **clean** remounts: the post-HEAD exit precedes the
harness's directory barriers, and clean detach may flush that metadata. They
add no abrupt-interruption, power-loss, complete-cut-matrix or production
qualification claim.
