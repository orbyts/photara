import AppKit
import SwiftUI

@MainActor
func verifyUI0Theme(_ url: URL) throws {
  let document = try PhotaraThemeDocument.load(from: url)
  require(document.neutralityWarnings().isEmpty, "Default ladder is not neutral or ordered")
  require(
    document.contrastWarnings().isEmpty,
    "Default text/status contrast failed: \(document.contrastWarnings())")
  require(PhotaraSurfaceLevel.allCases.count == 3, "The shared ladder grew beyond three levels")
  for appearance in PhotaraThemeAppearance.allCases {
    let palette = document.resolved(for: appearance)
    for role in PhotaraThemeRole.allCases {
      require(palette.rgba(role) != nil, "Unresolved semantic role \(role)")
    }
    for role in PhotaraThemeRole.aliases {
      require(
        palette.rgba(role) == palette.rgba(role.canonical), "Compatibility role diverged: \(role)")
      require(
        document.mode(appearance).colors[role.rawValue] == nil, "Duplicate authored background")
    }
    // Change the source: every alias must follow it, regardless of stale
    // independently colored literal values in a schema-1 import.
    var legacy = document
    for role in PhotaraThemeRole.aliases {
      if appearance == .dark {
        legacy.modes.dark.colors[role.rawValue] = "#FF00FF"
      } else {
        legacy.modes.light.colors[role.rawValue] = "#FF00FF"
      }
      require(
        legacy.resolved(for: appearance).rgba(role) == palette.rgba(role.canonical),
        "Legacy value overrode the ladder")
    }
    legacy.setColor("#454545", for: .graphBackground, appearance: appearance)
    require(
      legacy.resolved(for: appearance).rgba(.surfaceCanvas)?.hex == "#454545",
      "Alias writes bypassed their owner")
  }
  let encoded = try JSONEncoder().encode(document)
  let roundTrip = try JSONDecoder().decode(PhotaraThemeDocument.self, from: encoded)
  require(roundTrip == document, "Canonical theme round trip failed")
  // Build actual old wire bytes (the new encoder deliberately omits aliases).
  var legacyObject = try JSONSerialization.jsonObject(with: encoded) as! [String: Any]
  var modes = legacyObject["modes"] as! [String: [String: [String: String]]]
  for appearance in ["light", "dark"] {
    for alias in PhotaraThemeRole.aliases {
      modes[appearance]!["colors"]![alias.rawValue] = "#FF00FF"
    }
  }
  legacyObject["modes"] = modes
  let imported = try JSONDecoder().decode(
    PhotaraThemeDocument.self, from: JSONSerialization.data(withJSONObject: legacyObject))
  try imported.validate()
  require(imported == document, "Legacy literal aliases were not migrated")
  var missing = document
  missing.modes.light.colors.removeValue(forKey: "surface.canvas")
  missing.modes.dark.colors.removeValue(forKey: "surface.canvas")
  var unknown = document
  unknown.modes.light.colors["feature.background"] = "#123456"
  unknown.modes.dark.colors["feature.background"] = "#123456"
  var mismatch = document
  mismatch.modes.light.colors.removeValue(forKey: "text.primary")
  var invalid = document
  invalid.modes.dark.colors["text.primary"] = "blue"
  var wrongSchema = document
  wrongSchema.schemaVersion = 2
  for bad in [missing, unknown, mismatch, invalid, wrongSchema] {
    do {
      try bad.validate()
      fatalError("Invalid theme accepted")
    } catch {}
  }
  var tinted = document
  tinted.setColor("#332244", for: .surfaceCanvas, appearance: .dark)
  require(!tinted.neutralityWarnings().isEmpty, "Chromatic authoring escaped neutrality validation")
  tinted.setColor("#33333380", for: .surfaceCanvas, appearance: .dark)
  require(!tinted.neutralityWarnings().isEmpty, "Translucent content escaped neutrality validation")
  var lowContrast = document
  lowContrast.setColor("#EEEEEE", for: .textSecondary, appearance: .light)
  require(!lowContrast.contrastWarnings().isEmpty, "Secondary text contrast was not checked")
  print(
    "PASS: UI0 canonical/legacy parser, all role aliases, three neutral levels and 50 text/status contrast pairs"
  )
}

@MainActor
func verifyUI0Scenarios(_ model: ShellLabModel) {
  for from in ShellScenario.allCases {
    for to in ShellScenario.allCases {
      model.scenario = from
      model.session.show(.diagnostics)
      model.session.show(.people)
      model.session.requestNodeMenu()
      model.scenario = to
      let expected = to.presentation
      require(
        model.presentation.projectID == expected.projectID
          && model.presentation.nodeIDs == expected.nodeIDs
          && model.presentation.hasAssets == expected.hasAssets
          && model.presentation.isEvaluating == expected.isEvaluating
          && model.presentation.syncLabel == expected.syncLabel,
        "Scenario presentation leaked from \(from) to \(to)")
      require(
        !model.session.consumeNodeMenuRequest(), "Inactive scenario left a node catalog request")
      require(model.session.selectedAssetID == nil, "Inactive scenario left an asset selection")
      require(
        to.authorsModuleGeometry == model.presentation.hasOpenProject,
        "Inactive controls leaked into \(to)")
    }
  }
  print(
    "PASS: UI0 all \(ShellScenario.allCases.count * ShellScenario.allCases.count) scenario transitions isolate presentation, controls and pending commands"
  )
}

@MainActor
func probeOpeningAccessibility(_ mode: String) async throws {
  let process = Process()
  process.executableURL = Bundle.main.executableURL!.deletingLastPathComponent().appending(
    path: "OpeningAccessibilityProbe")
  process.arguments = [String(ProcessInfo.processInfo.processIdentifier), mode]
  try process.run()
  while process.isRunning { try await Task.sleep(for: .milliseconds(20)) }
  require(process.terminationStatus == 0, "External Opening accessibility probe failed")
}

@MainActor
func verifyUI0Opening(directory: URL) async throws {
  let model = ShellLabModel()
  for dark in [false, true] {
    for (label, size) in [
      ("narrow", NSSize(width: 760, height: 560)), ("standard", NSSize(width: 1280, height: 820)),
    ] {
      model.scenario = .opening
      model.dark = dark
      try await capture(
        ShellLabPreview(model: model, session: model.session),
        name: "production-opening-ui0-\(dark ? "dark" : "light")-\(label)", size: size,
        directory: directory,
        inspect: { _, _ in try await probeOpeningAccessibility("inspect") })
    }
  }
  // Exercise the actual native action, with a fixture callback and no storage.
  try await capture(
    ShellLabPreview(model: model, session: model.session),
    name: "ui0-opening-action", size: .init(width: 1280, height: 820), directory: directory,
    inspect: { _, _ in
      try await probeOpeningAccessibility("create")
      try await Task.sleep(for: .milliseconds(80))
      require(model.scenario == .createProject, "Opening did not dispatch the typed create intent")
    })
  model.scenario = .opening
  try await capture(
    ShellLabPreview(model: model, session: model.session),
    name: "ui0-opening-open-action", size: .init(width: 1280, height: 820), directory: directory,
    inspect: { _, _ in
      try await probeOpeningAccessibility("open")
      try await Task.sleep(for: .milliseconds(80))
      require(model.scenario == .assets, "Opening did not dispatch the typed open intent")
    })
  print("PASS: UI0 production Opening native actions, accessibility and narrow/standard bounds")
}

/// Visual checkpoint only: all actions target fixture callbacks, never storage.
@MainActor
func verifyUI1CreateProject(directory: URL) async throws {
  require(CreateProjectPresentation.shipped == .compact, "Compact is not the shipped default")
  require(
    CreateProjectPresentation.allCases == [.compact, .balanced, .spacious],
    "Lab comparison options changed")
  let model = ShellLabModel()
  require(model.createProjectPresentation == .compact, "Lab default diverged from shared default")
  model.send(.newProject)
  require(
    model.scenario == .createProject && !model.presentation.hasOpenProject,
    "Create intent prematurely opened a project")
  var draft = CreateProjectDraft()
  require(
    !draft.canCreate && draft.packageName == "Untitled Project.photara",
    "Blank draft contract changed")
  draft.name = " \n\t "
  require(!draft.canCreate, "Whitespace-only draft enabled creation")
  draft.name = "  Coastal Studies  "
  require(
    draft.canCreate && draft.packageName == "Coastal Studies.photara",
    "Trimmed package preview changed")
  for dark in [false, true] {
    for action in ["create", "cancel"] {
      var chosen = false
      var cancelled = false
      var submitted: CreateProjectDraft?
      let view = CreateProjectView(
        libraryName: "My Library", isCloudLibrary: true,
        chooseDestination: { chosen = true }, cancel: { cancelled = true },
        create: { submitted = $0 })
      require(view.presentation == .compact, "Shared view default diverged from Compact")
      try await capture(
        LabAppearance(dark: dark, usesDevelopmentTheme: false) { view },
        name: "ui1-native-\(action)-\(dark)", size: .init(width: 760, height: 620),
        directory: directory,
        inspect: { _, _ in try await probeOpeningAccessibility("draft-\(action)") })
      require(chosen, "Choose did not reach the fixture callback")
      if action == "create" {
        require(
          !cancelled && submitted?.packageName == "Coastal Studies.photara",
          "Native confirmation lost the draft")
        require(
          submitted?.destination == "~/Pictures/Photara/Projects", "Default destination changed")
      } else {
        require(cancelled && submitted == nil, "Cancellation submitted a draft")
      }
    }
  }
  print(
    "PASS: UI1 Compact default, three Lab comparisons, draft validation and native Choose/Cancel/Create callbacks; no creation wiring"
  )
}
