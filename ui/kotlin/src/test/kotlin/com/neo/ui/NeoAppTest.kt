package com.neo.ui

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NeoAppTest {
    @Test
    fun testInitialization() {
        val app = NeoApp()
        assertFalse(app.currentState.initialized)
        app.initialize()
        assertTrue(app.currentState.initialized)
    }

    @Test
    fun testNavigation() {
        val app = NeoApp()
        app.initialize()
        assertEquals("home", app.currentState.currentScreen)
        app.navigate("settings")
        assertEquals("settings", app.currentState.currentScreen)
    }

    @Test
    fun testShutdown() {
        val app = NeoApp()
        app.initialize()
        assertTrue(app.currentState.initialized)
        app.shutdown()
        assertFalse(app.currentState.initialized)
    }

    @Test
    fun testNotifications() {
        val app = NeoApp()
        app.currentState.addNotification("Hello")
        assertEquals(1, app.currentState.notifications.size)
        app.currentState.clearNotifications()
        assertTrue(app.currentState.notifications.isEmpty())
    }

    @Test
    fun testConfigDefaults() {
        val config = AppConfig()
        assertEquals("Neo AGI OS", config.title)
        assertEquals("dark", config.theme)
        assertFalse(config.debugMode)
    }
}
