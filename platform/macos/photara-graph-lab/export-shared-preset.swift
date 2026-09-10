#!/usr/bin/env swift
import Foundation

let defaults = UserDefaults(suiteName: "com.photara.graph-lab")!
guard let authoredData = defaults.data(forKey: "graph-lab.visual-preferences.v1"),
      let authored = try JSONSerialization.jsonObject(with: authoredData) as? [String: Any]
else { fatalError("Save Graph Lab preferences before exporting the shared preset.") }

let script = URL(fileURLWithPath: #filePath)
let destination = script.deletingLastPathComponent().deletingLastPathComponent()
    .appending(path: "photara-graph/Resources/photara-graph-presentation-v1.json")
let existingData = try Data(contentsOf: destination)
var preset = try JSONSerialization.jsonObject(with: existingData) as! [String: Any]
let directKeys = ["pattern", "gridSpacing", "minorOpacity", "minorLineWidth", "majorInterval",
    "majorOpacity", "majorLineWidth", "selectedStrokeWidth", "cornerRadius", "portOffset",
    "portGlassTintOpacity", "portCoreSize", "inactivePortSaturation", "inactivePortShowsStroke",
    "inactivePortStrokeWidth", "activePortShowsShadow", "activePortShadowOpacity",
    "activePortShadowBlur", "activePortShadowOffsetY", "lightPortCoreBrightness",
    "darkPortCoreBrightness", "lightActivePortCoreBrightness", "darkActivePortCoreBrightness",
    "overviewSizeFraction", "overviewCornerRadius"]
for key in directKeys where authored[key] != nil { preset[key] = authored[key] }
preset["light"] = authored["lightColors"]
preset["dark"] = authored["darkColors"]
preset["nodeShadowBlur"] = authored["shadowBlur"]
preset["nodeShadowOffsetY"] = authored["shadowOffsetY"]
preset["lightNodeShadowOpacity"] = authored["lightRestingShadowOpacity"] ?? authored["shadowOpacity"]
preset["darkNodeShadowOpacity"] = authored["darkRestingShadowOpacity"] ?? authored["shadowOpacity"]
let mappings = ["toolRailCornerRadius": "graph-lab.tool-rail-corner-radius",
    "toolRailLightShadowOpacity": "graph-lab.tool-rail-light-shadow-opacity",
    "toolRailDarkShadowOpacity": "graph-lab.tool-rail-dark-shadow-opacity",
    "toolRailShadowBlur": "graph-lab.tool-rail-shadow-blur",
    "toolRailShadowOffsetY": "graph-lab.tool-rail-shadow-offset-y"]
for (presetKey, defaultsKey) in mappings {
    if let value = defaults.object(forKey: defaultsKey) { preset[presetKey] = value }
}
preset["schemaVersion"] = 1
let output = try JSONSerialization.data(withJSONObject: preset, options: [.prettyPrinted, .sortedKeys])
try output.write(to: destination, options: .atomic)
print(destination.path)
