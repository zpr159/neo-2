import Foundation

public class NeoApp {
    public struct Config {
        public let title: String
        public let theme: String
        public let debugMode: Bool

        public init(title: String = "Neo AGI OS", theme: String = "dark", debugMode: Bool = false) {
            self.title = title
            self.theme = theme
            self.debugMode = debugMode
        }
    }

    public struct State {
        public var initialized: Bool = false
        public var currentScreen: String = "home"
        public var notifications: [String] = []

        public init() {}
    }

    private let config: Config
    public private(set) var state: State

    public init(config: Config = Config()) {
        self.config = config
        self.state = State()
    }

    public func initialize() {
        state.initialized = true
    }

    public func navigate(to screen: String) {
        state.currentScreen = screen
    }

    public func notify(_ message: String) {
        state.notifications.append(message)
    }

    public func shutdown() {
        state.initialized = false
        state.notifications.removeAll()
    }
}
