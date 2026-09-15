import AppKit
import Foundation
import SwiftUI

extension AppModel {
  func chooseCreationDestination() {
    let panel = NSOpenPanel()
    panel.title = "Choose Project Location"
    panel.prompt = "Choose"
    panel.canChooseFiles = false
    panel.canChooseDirectories = true
    panel.allowsMultipleSelection = false
    panel.canCreateDirectories = true
    panel.directoryURL = creationDestination
    guard panel.runModal() == .OK, let url = panel.url else { return }
    let scoped = url.startAccessingSecurityScopedResource()
    defer { if scoped { url.stopAccessingSecurityScopedResource() } }
    do {
      creationDestinationPin = try inspectCreationDestination(path: url.standardizedFileURL.path)
      creationDestination = url.standardizedFileURL
      creationMessage = nil
    } catch { creationMessage = error.localizedDescription }
  }

  func submitProjectCreation(_ draft: CreateProjectDraft) {
    guard !isCreatingProject, let creationJournal else { return }
    isCreatingProject = true
    creationCancellationRequested = false
    creationMessage = nil
    let operation = creationOperation ?? UUID().uuidString.lowercased()
    let destination = creationDestination
    creationTask = Task {
      let scoped = destination.startAccessingSecurityScopedResource()
      defer {
        if scoped { destination.stopAccessingSecurityScopedResource() }
        isCreatingProject = false
      }
      do {
        if creationOperation == nil {
          // Only the app's explicit default may be created automatically.
          if destination == defaultCreationDestination {
            try FileManager.default.createDirectory(
              at: destination, withIntermediateDirectories: true)
          }
          let prepared = try await creationJournal.prepare(
            operation: operation, title: draft.name, destination: destination,
            destinationPin: creationDestinationPin)
          creationOperation = operation
          creationTitle = prepared.title
        }
        if creationCancellationRequested {
          try await creationJournal.cancel(operation)
        }
        var result = try await creationJournal.advance(operation)
        if result.isCloud && ["staged", "dispatching"].contains(result.state) {
          creationMessage =
            "Connecting to your Library. If the connection is interrupted, retry this same operation."
          try await creationCloudDriver.projectCreation(result, journal: creationJournal)
          result = try await creationJournal.advance(operation)
        }
        if result.state == "cancelled" {
          creationOperation = nil
          creationTitle = nil
          showsCreateProject = false
          return
        }
        guard result.state == "complete" else {
          creationMessage = "Creation is incomplete. Retry to finish this project."
          return
        }
        finishProjectCreation(result)
      } catch {
        creationMessage = error.localizedDescription
      }
    }
  }

  func cancelProjectCreation() {
    creationCancellationRequested = true
    guard let operation = creationOperation, let creationJournal else {
      if !isCreatingProject { showsCreateProject = false }
      return
    }
    Task {
      do {
        try await creationJournal.cancel(operation)
        creationOperation = nil
        creationTitle = nil
        creationMessage = nil
        showsCreateProject = false
      } catch {
        creationMessage =
          "This operation may already have been published. Retry to reconcile its saved identity before closing."
      }
    }
  }

  func finishProjectCreation(_ result: BridgeProjectCreation) {
    closeProject()
    createdProject = result
    creationOperation = nil
    creationTitle = nil
    creationMessage = nil
    showsCreateProject = false
    rememberCreatedProject(result)
  }

  func restoreCreationOperations() {
    guard let creationJournal else { return }
    Task {
      do {
        let operations = try await creationJournal.operations()
        if let pending = operations.first(where: { !["complete", "cancelled"].contains($0.state) })
        {
          creationOperation = pending.operationId
          creationTitle = pending.title
          creationDestination = URL(fileURLWithPath: pending.packagePath)
            .deletingLastPathComponent()
          creationMessage =
            "An interrupted creation is saved. Retry to finish \(pending.title) at its original destination."
          showsCreateProject = true
        }
      } catch { creationMessage = error.localizedDescription }
    }
  }

  func openCreatedPackage(_ path: String) {
    guard let creationJournal else { return }
    Task {
      do {
        guard
          let operation = try await creationJournal.operations().first(where: {
            $0.packagePath == path && $0.state == "complete"
          })
        else {
          throw CocoaError(.fileReadUnsupportedScheme)
        }
        let result = try await creationJournal.advance(operation.operationId)
        finishProjectCreation(result)
      } catch { presentedError = error.localizedDescription }
    }
  }
}
