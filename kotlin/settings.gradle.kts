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

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
        maven("https://maven.pkg.jetbrains.space/public/p/compose/dev")
    }
    resolutionStrategy {
        eachPlugin {
            when (requested.id.id) {
                "org.jetbrains.kotlin.multiplatform",
                "org.jetbrains.kotlin.jvm",
                "org.jetbrains.kotlin.android" -> {
                    useModule("org.jetbrains.kotlin:kotlin-gradle-plugin:${requested.version}")
                }
            }
        }
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        maven("https://maven.pkg.jetbrains.space/public/p/compose/dev")
    }
}

rootProject.name = "iguanodon-kmp"

include(":iguanodon-compose")
val hasAndroidSdk = System.getenv("ANDROID_HOME")?.let { java.io.File(it).exists() } == true ||
    System.getenv("ANDROID_SDK_ROOT")?.let { java.io.File(it).exists() } == true ||
    java.io.File(System.getProperty("user.home"), "Android/Sdk").exists()
if (hasAndroidSdk) {
    include(":android-benchmark-app")
}
include(":desktop-benchmark-app")
