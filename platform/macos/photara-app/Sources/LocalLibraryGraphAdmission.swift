import Foundation

/// The currently admitted native command surface. This does not grant local
/// controller insertion/connection edits any persistence authority.
enum LocalLibraryGraphAdmission {
    static func position(_ mutation: PhotaraGraphMutation) -> (id: String, x: Int64, y: Int64)? {
        guard case let .moveNode(id, point) = mutation,
              point.x.isFinite, point.y.isFinite,
              let x = Int64(exactly: (point.x * 1_000).rounded()),
              let y = Int64(exactly: (point.y * 1_000).rounded()) else { return nil }
        return (id, x, y)
    }
}
