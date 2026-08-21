// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "photara-proxy-imageio",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "photara-proxy-imageio", targets: ["photara-proxy-imageio"]),
    ],
    targets: [
        .executableTarget(name: "photara-proxy-imageio"),
    ]
)
