// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "PeaPodMacos",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "PeaPodMacos", targets: ["PeaPodMacos"]),
    ],
    targets: [
        .target(
            name: "PeaPodMacos",
            path: "Sources/PeaPodMacos"
        ),
    ]
)
