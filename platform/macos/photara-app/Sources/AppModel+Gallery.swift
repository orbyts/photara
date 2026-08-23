import AppKit
import Foundation
import ImageIO
import QuickLookThumbnailing

extension AppModel {
    func requestGalleryThumbnail(assetID: String) {
        guard let project,
              let asset = snapshot?.assets.first(where: { $0.assetId == assetID }),
              let revision = asset.visualRevision
        else { return }
        // Every Gallery item ultimately enters the project-scoped proxy cache.
        // Quick Look may still win the first-pixel race for formats where it is
        // cheaper, but reopening the project can reuse the verified proxy.
        requestGalleryProxy(
            assetID: assetID,
            desiredRevision: revision,
            project: project
        )
        requestNativeThumbnail(
            assetID: assetID,
            desiredRevision: revision,
            project: project
        )
    }

    func openGalleryAsset(assetID: String) {
        guard let project else { return }
        do {
            let source = try project.nativeThumbnailSource(assetId: assetID)
            guard NSWorkspace.shared.open(URL(fileURLWithPath: source.localPath)) else {
                presentedError = "macOS could not open this asset in its default application."
                return
            }
        } catch {
            presentedError = error.localizedDescription
        }
    }

    private func requestNativeThumbnail(
        assetID: String,
        desiredRevision: String,
        project: PhotaraProject
    ) {
        if gallery.displayedRevisions[assetID] == desiredRevision {
            gallery.activities[assetID] = .ready
            return
        }
        guard gallery.pendingNativeRevisions[assetID] != desiredRevision else { return }
        gallery.pendingNativeRevisions[assetID] = desiredRevision
        gallery.activities[assetID] = gallery.displayedRevisions[assetID] == nil
            ? .loading : .updating
        let source: BridgeNativeThumbnailSourceDto
        do {
            source = try project.nativeThumbnailSource(assetId: assetID)
        } catch {
            gallery.pendingNativeRevisions.removeValue(forKey: assetID)
            requestGalleryProxy(
                assetID: assetID,
                desiredRevision: desiredRevision,
                project: project
            )
            if gallery.displayedRevisions[assetID] == nil {
                gallery.activities[assetID] = .failed
            }
            return
        }
        let sourceExtension = URL(fileURLWithPath: source.localPath)
            .pathExtension.lowercased()
        if sourceExtension == "tif" || sourceExtension == "tiff" {
            gallery.pendingNativeRevisions.removeValue(forKey: assetID)
            requestGalleryProxy(
                assetID: assetID,
                desiredRevision: desiredRevision,
                project: project
            )
            return
        }
        let scale = NSScreen.main?.backingScaleFactor ?? 2
        let lowQualityRequest = QLThumbnailGenerator.Request(
            fileAt: URL(fileURLWithPath: source.localPath),
            size: CGSize(width: 384, height: 288),
            scale: scale,
            representationTypes: .lowQualityThumbnail
        )
        let fullRequest = QLThumbnailGenerator.Request(
            fileAt: URL(fileURLWithPath: source.localPath),
            size: CGSize(width: 384, height: 288),
            scale: scale,
            representationTypes: .thumbnail
        )
        Task { [weak self] in
            guard let self else { return }
            defer {
                if gallery.pendingNativeRevisions[assetID] == desiredRevision {
                    gallery.pendingNativeRevisions.removeValue(forKey: assetID)
                }
            }
            await nativeThumbnailScheduler.acquire()
            let lowQualityRepresentation = try? await QLThumbnailGenerator.shared
                .generateBestRepresentation(for: lowQualityRequest)
            await nativeThumbnailScheduler.release()
            if let representation = lowQualityRepresentation,
               previewIsStillDesired(
                   assetID: assetID,
                   revision: desiredRevision,
                   project: project
               ), gallery.proxies[assetID] == nil
            {
                gallery.nativeThumbnails[assetID] = NSImage(
                    cgImage: representation.cgImage,
                    size: representation.contentRect.size
                )
                gallery.proxies.removeValue(forKey: assetID)
                gallery.displayedRevisions[assetID] = source.sourceFingerprint
                gallery.activities[assetID] = .updating
            }
            do {
                await nativeThumbnailScheduler.acquire()
                defer { Task { await self.nativeThumbnailScheduler.release() } }
                let representation = try await QLThumbnailGenerator.shared
                    .generateBestRepresentation(for: fullRequest)
                guard previewIsStillDesired(
                    assetID: assetID,
                    revision: desiredRevision,
                    project: project
                ), gallery.proxies[assetID] == nil
                else { return }
                gallery.nativeThumbnails[assetID] = NSImage(
                    cgImage: representation.cgImage,
                    size: representation.contentRect.size
                )
                gallery.proxies.removeValue(forKey: assetID)
                gallery.displayedRevisions[assetID] = source.sourceFingerprint
                gallery.activities[assetID] = .ready
            } catch {
                requestGalleryProxy(
                    assetID: assetID,
                    desiredRevision: desiredRevision,
                    project: project
                )
                if gallery.displayedRevisions[assetID] == nil {
                    gallery.activities[assetID] = .failed
                }
            }
        }
    }

    private func requestGalleryProxy(
        assetID: String,
        desiredRevision: String,
        project: PhotaraProject
    ) {
        guard gallery.pendingProxyRevisions[assetID] != desiredRevision else { return }
        gallery.pendingProxyRevisions[assetID] = desiredRevision
        Task { [weak self] in
            let result = await Task.detached(priority: .utility) {
                Result {
                    let reference = try project.requestGalleryThumbnail(assetId: assetID)
                    let descriptor = reference.descriptor()
                    let proxyData = try? Data(
                        contentsOf: URL(fileURLWithPath: descriptor.localPath),
                        options: .mappedIfSafe
                    )
                    let decodedImage: CGImage? = proxyData.flatMap { data in
                        guard let source = CGImageSourceCreateWithData(
                            data as CFData,
                            [kCGImageSourceShouldCache: false] as CFDictionary
                        ) else { return nil }
                        return CGImageSourceCreateImageAtIndex(
                            source,
                            0,
                            [kCGImageSourceShouldCacheImmediately: true] as CFDictionary
                        )
                    }
                    return (reference, descriptor, decodedImage)
                }
            }.value
            guard let self else { return }
            if gallery.pendingProxyRevisions[assetID] == desiredRevision {
                gallery.pendingProxyRevisions.removeValue(forKey: assetID)
            }
            guard previewIsStillDesired(
                assetID: assetID,
                revision: desiredRevision,
                project: project
            ) else { return }
            switch result {
            case let .success((reference, descriptor, decodedImage)):
                gallery.proxies[assetID] = reference
                gallery.proxyDescriptors[assetID] = descriptor
                if let decodedImage {
                    gallery.proxyImages[assetID] = NSImage(
                        cgImage: decodedImage,
                        size: NSSize(
                            width: decodedImage.width,
                            height: decodedImage.height
                        )
                    )
                }
                gallery.displayedRevisions[assetID] = desiredRevision
                gallery.activities[assetID] = .ready
                gallery.errors.removeValue(forKey: assetID)
            case let .failure(error):
                gallery.errors[assetID] = error.localizedDescription
                if gallery.displayedRevisions[assetID] == nil {
                    gallery.activities[assetID] = .failed
                }
            }
        }
    }

    private func previewIsStillDesired(
        assetID: String,
        revision: String,
        project: PhotaraProject
    ) -> Bool {
        !Task.isCancelled
            && self.project === project
            && snapshot?.assets.first(where: { $0.assetId == assetID })?.visualRevision
                == revision
    }

    func waitForInitialPreviews(
        assetIDs: Set<String>,
        project: PhotaraProject
    ) async -> Bool {
        while self.project === project, !Task.isCancelled {
            let currentAssets = snapshot?.assets.filter {
                assetIDs.contains($0.assetId)
            } ?? []
            let requestedAssets = currentAssets.filter {
                gallery.activities[$0.assetId] != nil
            }
            let requestedAreTerminal = !requestedAssets.isEmpty
                && requestedAssets.allSatisfy { asset in
                guard let desiredRevision = asset.visualRevision else { return true }
                return gallery.displayedRevisions[asset.assetId] == desiredRevision
                    || gallery.activities[asset.assetId] == .failed
            }
            let requestedIDs = Set(requestedAssets.map(\.assetId))
            let hasPendingRequest = gallery.pendingProxyRevisions.keys.contains {
                requestedIDs.contains($0)
            } || gallery.pendingNativeRevisions.keys.contains {
                requestedIDs.contains($0)
            }
            if requestedAreTerminal && !hasPendingRequest { return true }
            try? await Task.sleep(for: .milliseconds(250))
        }
        return false
    }

}
