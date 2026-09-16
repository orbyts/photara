import Foundation
import SwiftUI

enum CreateProjectPresentation: String, CaseIterable, Identifiable, Sendable {
  case compact
  case balanced
  case spacious

  var id: String { rawValue }
  var title: String { rawValue.capitalized }
  static let shipped: Self = .compact
}

struct CreateProjectDraft: Equatable, Sendable {
  var name = ""
  var destination = "~/Pictures/\(ReleaseConfiguration.current.identity.defaultProjectsDirectory)/Projects"

  var trimmedName: String { name.trimmingCharacters(in: .whitespacesAndNewlines) }
  var packageName: String { (trimmedName.isEmpty ? "Untitled Project" : trimmedName) + "." + ReleaseConfiguration.current.identity.projectPackageExtension }
  var canCreate: Bool { !trimmedName.isEmpty }
}

struct CreateProjectView: View {
  let libraryName: String
  let isCloudLibrary: Bool
  let chooseDestination: () -> Void
  let cancel: () -> Void
  let create: (CreateProjectDraft) -> Void
  var presentation: CreateProjectPresentation = .shipped
  var destination: String? = nil
  var isWorking = false
  var recoveryMessage: String? = nil
  var hasPendingOperation = false
  var pendingName: String? = nil
  var keepForLater: (() -> Void)? = nil
  @State private var draft = CreateProjectDraft()
  @FocusState private var focusesName: Bool

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      HStack(alignment: .top, spacing: 14) {
        Image(systemName: "folder.badge.plus")
          .font(.system(size: 30, weight: .regular))
          .foregroundStyle(.tint)
          .frame(width: 38, height: 38)
        VStack(alignment: .leading, spacing: 3) {
          Text("Create New Project").font(.title2.weight(.semibold))
          Text("Create an empty project and its first graph.")
            .foregroundStyle(.secondary)
        }
      }
      .padding(.bottom, 22)

      Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 14) {
        GridRow {
          Text("Library").foregroundStyle(.secondary).gridColumnAlignment(.trailing)
          HStack(spacing: 7) {
            Image(systemName: isCloudLibrary ? "cloud" : "laptopcomputer")
              .foregroundStyle(.secondary)
            Text(libraryName)
          }
        }
        GridRow {
          Text("Project Name").foregroundStyle(.secondary).gridColumnAlignment(.trailing)
          TextField("Project name", text: $draft.name)
            .textFieldStyle(.roundedBorder)
            .focused($focusesName)
            .accessibilityIdentifier("create-project-name")
            .disabled(isWorking || hasPendingOperation)
        }
        GridRow {
          Text("Location").foregroundStyle(.secondary).gridColumnAlignment(.trailing)
          HStack(spacing: 8) {
            Text(draft.destination)
              .lineLimit(1)
              .truncationMode(.middle)
              .frame(maxWidth: .infinity, alignment: .leading)
              .help(draft.destination)
            Button("Choose…", action: chooseDestination)
              .accessibilityIdentifier("create-project-choose")
              .disabled(isWorking || hasPendingOperation)
          }
        }
        GridRow {
          Text("Package").foregroundStyle(.secondary).gridColumnAlignment(.trailing)
          Label(draft.packageName, systemImage: "shippingbox")
            .lineLimit(1)
            .foregroundStyle(draft.trimmedName.isEmpty ? Color.secondary : Color.primary)
            .accessibilityIdentifier("create-project-package-name")
        }
      }

      if let recoveryMessage {
        Text(recoveryMessage).font(.callout).foregroundStyle(.secondary)
          .fixedSize(horizontal: false, vertical: true).padding(.top, 14)
          .accessibilityIdentifier("create-project-recovery")
      }
      if isWorking {
        ProgressView("Creating project…").controlSize(.small).padding(.top, 14)
      }

      if presentation != .compact {
        HStack(alignment: .top, spacing: 9) {
          Image(systemName: "info.circle")
            .foregroundStyle(.secondary)
          Text(
            "The package stores graphs, authored project state, manifests, and selected artifacts. Your external photo archive stays at its existing source."
          )
          .font(.callout)
          .foregroundStyle(.secondary)
          .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.top, 20)
      }

      if presentation == .spacious {
        GroupBox {
          HStack(spacing: 24) {
            Label("Initial Graph", systemImage: "point.3.connected.trianglepath.dotted")
            Label("Project Catalog", systemImage: "list.bullet.rectangle")
            Label("Portable Package", systemImage: "shippingbox")
          }
          .font(.callout)
          .foregroundStyle(.secondary)
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding(4)
        } label: {
          Text("Created with this project")
        }
        .padding(.top, 18)
      }

      Divider().padding(.vertical, 18)

      HStack {
        if hasPendingOperation, !isWorking, let keepForLater {
          Button("Keep for Later", action: keepForLater)
            .accessibilityIdentifier("create-project-defer")
        }
        Spacer()
        Button("Cancel", role: .cancel, action: cancel)
          .keyboardShortcut(.cancelAction)
          .accessibilityIdentifier("create-project-cancel")
        Button(hasPendingOperation ? "Retry" : "Create Project") { create(draft) }
          .keyboardShortcut(.defaultAction)
          .disabled(isWorking || (!hasPendingOperation && !draft.canCreate))
          .accessibilityIdentifier("create-project-confirm")
      }
    }
    .padding(presentation == .compact ? 20 : 24)
    .frame(width: sheetWidth)
    .onAppear {
      focusesName = true
      if let destination { draft.destination = destination }
      if let pendingName { draft.name = pendingName }
    }
    .onChange(of: pendingName) { if let pendingName { draft.name = pendingName } }
    .onChange(of: destination) { if let destination { draft.destination = destination } }
  }

  private var sheetWidth: CGFloat {
    switch presentation {
    case .compact: 470
    case .balanced: 560
    case .spacious: 680
    }
  }
}
