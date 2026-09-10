import SwiftUI

/// Graph Lab is an authoring host around the same production canvas used by Photara.
struct GraphLabCanvas<NodeContent: View>: View {
    @AppStorage(GraphLabSettingsKeys.toolRailPlacement) private var placementRaw = GraphLabToolRailPlacement.leftTop.rawValue
    let controller: PhotaraGraphInteractionController
    let backgroundStyle: PhotaraGraphBackgroundStyle
    let backgroundColor: Color?
    let minorColor: Color?
    let majorColor: Color?
    let noodleColor: Color
    let knifeCursorSize: Double
    let showsToolRail: Bool
    let centerScene: () -> Void
    let addNativeNode: (String) -> Void
    let nodeContent: (PhotaraGraphNode, Bool, Set<Int>, Set<Int>) -> NodeContent

    var body: some View {
        let labPlacement = GraphLabToolRailPlacement(rawValue: placementRaw) ?? .leftTop
        let placement = PhotaraGraphToolRailPlacement(rawValue: labPlacement.rawValue) ?? .leftTop
        PhotaraGraphCanvas(toolRailPlacement: placement, controller: controller,
            backgroundStyle: backgroundStyle, backgroundColor: backgroundColor,
            minorColor: minorColor, majorColor: majorColor, noodleColor: noodleColor,
            knifeCursorSize: knifeCursorSize, showsToolRail: showsToolRail,
            centerScene: centerScene, addNativeNode: addNativeNode, nodeContent: nodeContent) {
            GraphLabToolPalette(controller: controller, centerScene: centerScene, addNativeNode: addNativeNode)
        }
    }
}
