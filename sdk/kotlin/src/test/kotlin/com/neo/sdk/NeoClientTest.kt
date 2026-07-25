package com.neo.sdk

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NeoClientTest {
    @Test
    fun testConnectionLifecycle() {
        val client = NeoClient()
        assertFalse(client.isConnected)
        client.connect()
        assertTrue(client.isConnected)
        client.disconnect()
        assertFalse(client.isConnected)
    }

    @Test
    fun testCreateAgent() {
        val client = NeoClient()
        client.connect()
        val agent = client.createAgent(name = "test-agent")
        assertEquals("test-agent", agent.name)
        assertEquals("idle", agent.state)
    }

    @Test
    fun testSubmitTask() {
        val client = NeoClient()
        client.connect()
        val agent = client.createAgent(name = "worker")
        val task = client.submitTask(agentId = agent.id, task = mapOf("action" to "process"))
        assertEquals("pending", task.status)
        assertEquals(agent.id, task.agentId)
    }

    @Test(expected = IllegalStateException::class)
    fun testCreateAgentWithoutConnection() {
        val client = NeoClient()
        client.createAgent(name = "test")
    }

    @Test
    fun testHealthCheck() {
        val client = NeoClient()
        val health = client.health()
        assertEquals("disconnected", health["status"])
        client.connect()
        val healthAfter = client.health()
        assertEquals("ok", healthAfter["status"])
    }
}
