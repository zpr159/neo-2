package com.neo.sdk

class NeoClient(
    private val host: String = "localhost",
    private val port: Int = 8080,
    private val apiKey: String? = null,
) {
    private var connected: Boolean = false
    val isConnected: Boolean get() = connected

    fun connect() {
        connected = true
    }

    fun disconnect() {
        connected = false
    }

    fun createAgent(name: String, config: Map<String, Any> = emptyMap()): AgentHandle {
        require(connected) { "Client not connected" }
        return AgentHandle(
            id = java.util.UUID.randomUUID().toString(),
            name = name,
            state = "idle",
            config = config,
        )
    }

    fun submitTask(agentId: String, task: Map<String, Any>): TaskHandle {
        require(connected) { "Client not connected" }
        return TaskHandle(
            id = java.util.UUID.randomUUID().toString(),
            agentId = agentId,
            status = "pending",
            payload = task,
        )
    }

    fun health(): Map<String, Any> = mapOf(
        "status" to if (connected) "ok" else "disconnected",
        "host" to host,
        "port" to port,
    )
}

data class AgentHandle(
    val id: String,
    val name: String,
    val state: String,
    val config: Map<String, Any> = emptyMap(),
)

data class TaskHandle(
    val id: String,
    val agentId: String,
    val status: String,
    val payload: Map<String, Any> = emptyMap(),
    val result: Any? = null,
)
