plugins {
    kotlin("jvm") version "1.9.22"
    kotlin("plugin.serialization") version "1.9.22"
    kotlin("plugin.spring") version "1.9.22"
    id("com.android.application") version "8.2.2" apply false
    id("com.android.library") version "8.2.2" apply false
}

allprojects {
    group = "org.neo-agi"
    version = "0.1.0"

    repositories {
        mavenCentral()
        google()
    }
}

subprojects {
    apply(plugin = "org.jetbrains.kotlin.jvm")

    kotlin {
        jvmToolchain(21)
    }

    tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
        kotlinOptions {
            jvmTarget = "21"
            freeCompilerArgs += listOf(
                "-opt-in=kotlin.RequiresOptIn",
                "-Xjsr305=strict",
            )
        }
    }

    tasks.withType<Test> {
        useJUnitPlatform()
    }
}
