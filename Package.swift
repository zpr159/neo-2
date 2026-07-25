// Neo SDK for iOS/macOS
// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "NeoSDK",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(
            name: "NeoSDK",
            targets: ["NeoSDK"]
        ),
    ],
    dependencies: [
        // Dependencies will be added as the SDK grows
    ],
    targets: [
        .target(
            name: "NeoSDK",
            dependencies: [],
            path: "sdk/swift/Sources/NeoSDK"
        ),
        .testTarget(
            name: "NeoSDKTests",
            dependencies: ["NeoSDK"],
            path: "sdk/swift/Tests/NeoSDKTests"
        ),
    ]
)
