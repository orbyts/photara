import SwiftUI

struct DiagnosticsView: View {
    let diagnostics: [DiagnosticPresentation]

    var body: some View {
        if !diagnostics.isEmpty {
            List(diagnostics, id: \.code) { diagnostic in
                VStack(alignment: .leading) {
                    Text(diagnostic.message)
                    Text(diagnostic.code)
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                }
            }
        } else {
            ContentUnavailableView("No Diagnostics", systemImage: "checkmark.circle")
        }
    }
}
