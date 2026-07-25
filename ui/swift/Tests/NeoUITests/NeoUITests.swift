import XCTest
@testable import NeoUI

final class NeoUITests: XCTestCase {
    func testInitialization() {
        let app = NeoApp()
        XCTAssertFalse(app.state.initialized)
        app.initialize()
        XCTAssertTrue(app.state.initialized)
    }

    func testNavigation() {
        let app = NeoApp()
        app.initialize()
        app.navigate(to: "settings")
        XCTAssertEqual(app.state.currentScreen, "settings")
    }

    func testNotifications() {
        let app = NeoApp()
        app.notify("Hello")
        XCTAssertEqual(app.state.notifications.count, 1)
        app.shutdown()
        XCTAssertTrue(app.state.notifications.isEmpty)
    }
}
