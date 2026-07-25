package com.neo.robotics

data class Joint(
    val name: String,
    var angle: Double = 0.0,
    var velocity: Double = 0.0,
    val minAngle: Double = -180.0,
    val maxAngle: Double = 180.0,
) {
    fun setAngle(newAngle: Double) {
        angle = newAngle.coerceIn(minAngle, maxAngle)
    }

    fun toMap(): Map<String, Any> = mapOf(
        "name" to name,
        "angle" to angle,
        "velocity" to velocity,
    )
}
