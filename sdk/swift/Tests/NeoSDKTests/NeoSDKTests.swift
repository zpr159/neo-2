import XCTest
@testable import NeoSDK

final class NeoSDKTests: XCTestCase {
    func testConnectionLifecycle() {
        let client = NeoClient()
        XCTAssertFalse(client.isConnected)
        client.connect()
        XCTAssertTrue(client.isConnected)
        client.disconnect()
        XCTAssertFalse(client.isConnected)
    }

    func testCreateAgent() {
        let client = NeoClient()
        client.connect()
        let agent = client.createAgent(name: "test-agent")
        XCTAssertEqual(agent.name, "test-agent")
        XCTAssertEqual(agent.state, "idle")
    }
}
