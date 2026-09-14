import AppKit
import SwiftUI

struct AccountAvatar: View {
    let profile: NativeAccountProfile?
    var size: CGFloat = 20
    var body: some View {
        Group {
            if let data = profile?.avatar, let image = Self.circularImage(data, size: size) {
                Image(nsImage: image).renderingMode(.original).resizable().scaledToFit()
            } else {
                Image(systemName: "person.crop.circle").resizable().scaledToFit()
                    .foregroundStyle(.secondary)
            }
        }
        .frame(width: size, height: size)
        .clipShape(Circle())
        .accessibilityHidden(true)
    }

    // Native Menu labels may use the NSImage directly, bypassing SwiftUI's
    // frame and clip modifiers. Bound and mask the underlying image as well.
    private static func circularImage(_ data: Data, size: CGFloat) -> NSImage? {
        guard let source = NSImage(data: data)?.cgImage(forProposedRect: nil, context: nil, hints: nil) else { return nil }
        let pixels = Int((size * 2).rounded(.up))
        guard pixels > 0, let context = CGContext(data: nil, width: pixels, height: pixels,
            bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        let side = CGFloat(pixels)
        let scale = side / CGFloat(min(source.width, source.height))
        let width = CGFloat(source.width) * scale, height = CGFloat(source.height) * scale
        context.addEllipse(in: CGRect(x: 0, y: 0, width: side, height: side))
        context.clip()
        context.interpolationQuality = .high
        context.draw(source, in: CGRect(x: (side - width) / 2, y: (side - height) / 2, width: width, height: height))
        guard let image = context.makeImage() else { return nil }
        return NSImage(cgImage: image, size: NSSize(width: size, height: size))
    }
}

struct AccountsMenu: View {
    @ObservedObject var cloud: OpeningCloudModel
    var showLibrarySettings: (() -> Void)?
    @Environment(\.openSettings) private var openSettings

    var body: some View {
        Menu {
            if cloud.isSignedIn {
                Section {
                    if let name = cloud.account?.name { Text(name) }
                    if let email = cloud.account?.email { Text(email) }
                    Label(ReleaseConfiguration.current.identity.defaultLibraryName, systemImage: "books.vertical")
                }
                if let showLibrarySettings {
                    Section { Button("Library & Sync…", action: showLibrarySettings) }
                }
                Section { Button("Settings…") { openSettings() } }
                Section {
                    Button("Sign Out") { cloud.signOut() }
                        .disabled(cloud.isWorking)
                }
            } else {
                Button(cloud.signInTitle) { cloud.signIn() }
                    .disabled(cloud.availability != .available || cloud.isWorking || !cloud.loadedSavedState)
            }
            if let message = cloud.failureMessage {
                Section { Text(message) }
            }
        } label: {
            HStack(spacing: 8) {
                AccountAvatar(profile: cloud.isSignedIn ? cloud.account : nil)
                Text(cloud.account?.name ?? "Account")
                    .lineLimit(1).truncationMode(.tail)
            }
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize(horizontal: false, vertical: true)
        .frame(height: 32, alignment: .leading)
        .help("Accounts")
        .accessibilityLabel("Accounts")
        .accessibilityIdentifier("accounts-menu")
    }
}

/// The same compact account entry anchors both the Library sidebar and editor.
struct SidebarAccountControls: View {
    @ObservedObject var cloud: OpeningCloudModel
    var showLibrarySettings: (() -> Void)?

    var body: some View {
        HStack(spacing: 8) {
            if cloud.isSignedIn {
                AccountsMenu(cloud: cloud, showLibrarySettings: showLibrarySettings)
            } else if cloud.showsSignIn {
                Button { cloud.signIn() } label: {
                    HStack(spacing: 8) {
                        AccountAvatar(profile: nil)
                        Text(cloud.isWorking ? "Signing in…" : "Sign in")
                    }
                    .frame(height: 32)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .foregroundStyle(.primary)
                .accessibilityIdentifier("opening-sign-in-google")
                .accessibilityLabel(cloud.signInTitle)
                .help(cloud.signInTitle)
                .disabled(cloud.availability != .available || cloud.isWorking)
                if cloud.isWorking {
                    ProgressView().controlSize(.mini)
                    Button("Cancel") { cloud.cancel() }.buttonStyle(.plain)
                }
            } else {
                // Reserve the row during the passive read without a sign-in flash.
                Color.clear.frame(width: 20, height: 32).accessibilityHidden(true)
            }
            Spacer(minLength: 0)
        }
        .font(.callout)
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityIdentifier("sidebar-account-controls")
    }
}

struct CloudLibraryPicker: View {
    let choices: [NativeLibraryChoice]
    let select: (String) -> Void
    let cancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Choose a library").font(.title2.bold())
                Text("Select the library you’d like to open.")
                    .foregroundStyle(.secondary)
            }
            VStack(spacing: 8) {
                ForEach(choices) { library in
                    Button { select(library.id) } label: {
                        HStack(spacing: 12) {
                            Image(systemName: "books.vertical").font(.title2)
                                .frame(width: 36, height: 36)
                            VStack(alignment: .leading, spacing: 3) {
                                Text(library.name).fontWeight(.semibold)
                                Text("Cloud Library").font(.caption).foregroundStyle(.secondary)
                            }
                            Spacer()
                            Image(systemName: "chevron.right").foregroundStyle(.secondary)
                        }
                        .padding(12)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.bordered)
                    .accessibilityLabel("Open \(library.name)")
                    .accessibilityIdentifier("choose-cloud-library")
                }
            }
            HStack { Spacer(); Button("Cancel", action: cancel).keyboardShortcut(.cancelAction) }
        }
        .padding(28)
        .frame(width: 420)
        .accessibilityIdentifier("cloud-library-picker")
    }
}

/// Hosted once in the shell so the picker also works with an open project.
struct CloudAccountPresentation: ViewModifier {
    @ObservedObject var cloud: OpeningCloudModel
    func body(content: Content) -> some View {
        content.sheet(isPresented: Binding(get: { !cloud.libraryChoices.isEmpty }, set: {
            if !$0 && !cloud.libraryChoices.isEmpty { cloud.cancel() }
        })) {
            CloudLibraryPicker(choices: cloud.libraryChoices, select: cloud.chooseLibrary, cancel: cloud.cancel)
                .interactiveDismissDisabled()
        }
    }
}

struct OptionalCloudAccountPresentation: ViewModifier {
    let cloud: OpeningCloudModel?
    @ViewBuilder func body(content: Content) -> some View {
        if let cloud { content.modifier(CloudAccountPresentation(cloud: cloud)) }
        else { content }
    }
}
