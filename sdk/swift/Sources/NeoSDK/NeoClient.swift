import Foundation

public class NeoClient {
    public struct AgentHandle {
        public let id: String
        public let name: String
        public let state: String
    }

    public struct TaskHandle {
        public let id: String
        public let agentId: String
        public let status: String
    }

    private let host: String
    private let port: Int
    private let apiKey: String?
    public private(set) var isConnected: Bool = false

    public init(host: String = "localhost", port: Int = 8080, apiKey: String? = nil) {
        self.host = host
        self.port = port
        self.apiKey = apiKey
    }

    public func connect() {
        isConnected = true
    }

    public func disconnect() {
        isConnected = false
    }

    public func createAgent(name: String) -> AgentHandle {
        precondition(isConnected, "Client not connected")
        return AgentHandle(
            id: UUID().uuidString,
            name: name,
            state: "idle"
        )
    }

    public func submitTask(agentId: String, payload: [String: Any]) -> TaskHandle {
        precondition(isConnected, "Client not connected")
        return TaskHandle(
            id: UUID().uuidString,
            agentId: agentId,
            status: "pending"
        )
    }

    public func health() -> [String: Any] {
        return [
            "status": isConnected ? "ok" : "disconnected",
            "host": host,
            "port": port,
        ]
    }
}
