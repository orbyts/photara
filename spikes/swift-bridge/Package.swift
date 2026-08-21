// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "PhotaraSwiftBridgeSpike",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "photara-swift-bridge-spike", targets: ["PhotaraSwiftBridgeSpike"])
    ],
    targets: [
        .executableTarget(name: "PhotaraSwiftBridgeSpike")
    ]
)
