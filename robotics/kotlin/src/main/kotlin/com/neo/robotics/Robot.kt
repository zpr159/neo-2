package com.neo.robotics

import kotlinx.serialization.Serializable
import java.util.UUID

@Serializable
data class RobotConfig(
    val name: String,
    val type: String = "generic",
    val maxSpeed: Double = 1.0,
    val jointCount: Int = 6,
)

enum class RobotState {
    INITIALIZING, IDLE, MOVING, STOPPED, ERROR
}

class Robot(val id: String = UUID.randomUUID().toString()) {
    private var state: RobotState = RobotState.INITIALIZING
    private val joints: MutableList<Joint> = mutableListOf()

    val currentState: RobotState get() = state
    val jointCount: Int get() = joints.size

    fun initialize(config: RobotConfig) {
        repeat(config.jointCount) { i ->
            joints.add(Joint(name = "joint_$i"))
        }
        state = RobotState.IDLE
    }

    fun moveJoint(jointIndex: Int, angle: Double) {
        require(jointIndex in joints.indices) { "Invalid joint index: $jointIndex" }
        state = RobotState.MOVING
        joints[jointIndex].setAngle(angle)
        state = RobotState.IDLE
    }

    fun stop() {
        state = RobotState.STOPPED
    }

    fun getStatus(): Map<String, Any> = mapOf(
        "id" to id,
        "state" to state.name,
        "joints" to joints.map { it.toMap() },
    )
}
