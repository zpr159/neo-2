package com.neo.robotics

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class RobotTest {
    @Test
    fun testRobotInitialization() {
        val robot = Robot()
        robot.initialize(RobotConfig(name = "test", jointCount = 3))
        assertEquals(RobotState.IDLE, robot.currentState)
        assertEquals(3, robot.jointCount)
    }

    @Test
    fun testJointMovement() {
        val robot = Robot()
        robot.initialize(RobotConfig(name = "test", jointCount = 2))
        robot.moveJoint(0, 45.0)
        assertEquals(RobotState.IDLE, robot.currentState)
    }

    @Test
    fun testJointLimits() {
        val joint = Joint(name = "test", minAngle = -90.0, maxAngle = 90.0)
        joint.setAngle(100.0)
        assertEquals(90.0, joint.angle)
        joint.setAngle(-100.0)
        assertEquals(-90.0, joint.angle)
    }

    @Test
    fun testRobotStop() {
        val robot = Robot()
        robot.initialize(RobotConfig(name = "test"))
        robot.stop()
        assertEquals(RobotState.STOPPED, robot.currentState)
    }
}
