// Copyright 2026 Emilie Roberts
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

import java.io.File

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidApplication) apply false
    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
}

val hasAndroidSdk = System.getenv("ANDROID_HOME")?.let { File(it).exists() } == true ||
    System.getenv("ANDROID_SDK_ROOT")?.let { File(it).exists() } == true ||
    File(System.getProperty("user.home"), "Android/Sdk").exists()

if (hasAndroidSdk) {
    apply(plugin = "com.android.library")
}

kotlin {
    if (hasAndroidSdk) {
        androidTarget {
            compilerOptions {
                jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
            }
        }
    }
    jvm("desktop") {
        compilerOptions {
            jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
        }
    }

    sourceSets {
        val commonMain by getting {
            dependencies {
                // Pure Compose Runtime ONLY — zero dependency on compose-ui, compose-foundation, or compose-material3!
                api(compose.runtime)
                api(libs.kotlinx.coroutines.core)
                api(libs.kotlinx.serialization.json)
            }
        }
        if (hasAndroidSdk) {
            val androidMain by getting {
                dependencies {
                    implementation(libs.kotlinx.coroutines.android)
                }
            }
        }
        val desktopMain by getting {
            dependencies {
                implementation(libs.kotlinx.coroutines.swing)
            }
        }
        val desktopTest by getting {
            dependencies {
                implementation(kotlin("test"))
            }
        }
    }
}

tasks.withType<Test>().configureEach {
    val rootRepo = project.rootDir.parentFile
    systemProperty(
        "java.library.path",
        "${rootRepo.resolve("target/release").absolutePath}:${rootRepo.resolve("target/debug").absolutePath}",
    )
}

if (hasAndroidSdk) {
    extensions.configure<com.android.build.gradle.LibraryExtension>("android") {
        namespace = "iguanodon.compose"
        compileSdk = 35
        defaultConfig {
            minSdk = 26 // Android 8.0+ (Vulkan 1.1+ baseline)
            ndk {
                abiFilters += listOf("arm64-v8a", "x86_64")
            }
        }
        compileOptions {
            sourceCompatibility = JavaVersion.VERSION_17
            targetCompatibility = JavaVersion.VERSION_17
        }
    }
}
