package com.neo.ui

class NeoApp(private val config: AppConfig = AppConfig()) {
    private val state = AppState()
    val currentState: AppState get() = state

    fun initialize() {
        state.initialized = true
    }

    fun navigate(screen: String) {
        state.currentScreen = screen
    }

    fun shutdown() {
        state.initialized = false
    }
}

data class AppConfig(
    val title: String = "Neo AGI OS",
    val theme: String = "dark",
    val debugMode: Boolean = false,
)

class AppState {
    var initialized: Boolean = false
    var currentScreen: String = "home"
    var notifications: MutableList<String> = mutableListOf()

    fun addNotification(message: String) {
        notifications.add(message)
    }

    fun clearNotifications() {
        notifications.clear()
    }
}
