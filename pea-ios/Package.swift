// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "PeaPodIos",
    platforms: [.iOS(.v17)],
    products: [
        .library(name: "PeaPodIos", targets: ["PeaPodIos"]),
    ],
    targets: [
        .target(
            name: "PeaPodIos",
            path: "Sources/PeaPodIos"
        ),
    ]
)
