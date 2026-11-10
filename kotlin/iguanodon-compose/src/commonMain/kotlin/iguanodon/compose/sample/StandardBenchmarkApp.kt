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

package iguanodon.compose.sample

import androidx.compose.runtime.*
import iguanodon.compose.*

/**
 * Canonical Kotlin Multiplatform `@Composable` implementation of the
 * Iguanodon Standard Benchmark Sample App (`commonMain`).
 *
 * This exact file is shared 100% unchanged between:
 *  - `android-benchmark-app` (`IguanodonBenchmarkActivity` -> `IguanodonVulkanSurfaceView`)
 *  - `desktop-benchmark-app` (`Main.kt` -> `IguanodonDesktopWindow`)
 *
 * Automatically adapts between:
 *  - **Portrait Mode (`viewportWidth < viewportHeight`)**: Single vertical `Column` stacking all main sections
 *  - **Landscape Mode (`viewportWidth >= viewportHeight`)**: 3 side-by-side columns in a `Row`
 */
@Composable
fun StandardBenchmarkApp() {
    val session = LocalIguanodonSession.current
    val isPortrait = session?.isPortrait == true

    var activeTheme by remember { mutableStateOf(IguanodonThemeId.Material3Expressive) }
    var activeMode by remember { mutableStateOf(IguanodonThemeMode.Dark) }
    var activePowerMode by remember { mutableStateOf(IguanodonPowerMode.Performance) }
    var activeRenderer by remember { mutableStateOf(IguanodonRendererBackend.Iguanodon) }
    var clicks by remember { mutableStateOf(0) }
    var searchQuery by remember { mutableStateOf("Filter 10,000 Vulkan items...") }
    var progress by remember { mutableStateOf(0.35f) }
    var droppedPayloads by remember { mutableStateOf(listOf<String>()) }
    val lazyItems = remember { List(10_000) { "Vulkan SDF Node #$it" } }

    val mainSections: @Composable () -> Unit = {
        // Section 1: Labels, Buttons (>=48dp), Edit Text Entry, Progress Bars, Image, Animations
        Column {
            Label("Clicks: $clicks | UI: ${activeRenderer.displayName} | Power: ${activePowerMode.displayName}")
            Button(
                label = "Primary Action (+Progress) [$clicks]",
                nodeId = BenchNodeIds.PRIMARY_BTN,
            ) {
                clicks++
                progress = ((progress + 0.18f) % 1.0f).coerceAtLeast(0.1f)
            }
            Button(
                label = "Toggle Fire Shader [ON]",
                nodeId = BenchNodeIds.SECONDARY_FIRE_BTN,
                modifier = Modifier.fireEmitter(intensity = 1.0f, turbulence = 0.85f),
            ) {
                clicks++
            }
            TextField(
                value = searchQuery,
                placeholder = "Type to filter 10,000 items...",
                onValueChange = { searchQuery = it },
            )
            ProgressBar(progress = progress, modifier = Modifier.fireEmitter(intensity = 0.6f))
            ProgressBar(progress = null)
            Image(
                textureId = 1,
                contentDescription = "Squircle-clipped Vulkan texture with water ripples over 1080p water drop wallpaper",
                modifier = Modifier.waterRipple(damping = 0.96f, frequency = 10f),
            )
        }

        // Section 2: Drag & Drop Source/Target, Scrollable Text, Right-Click / Long-Press Context Menu
        Column {
            Draggable(
                payloadId = "Vulkan-Shader-Asset-01",
                nodeId = BenchNodeIds.DRAGGABLE_CARD_A,
            ) {
                Label("Drag Source A: Vulkan Asset #01")
            }
            Draggable(
                payloadId = "Material3-Token-Pack-02",
                nodeId = BenchNodeIds.DRAGGABLE_CARD_B,
            ) {
                Label("Drag Source B: M3 Token Pack #02")
            }
            DropTarget(
                onDrop = { id -> droppedPayloads = droppedPayloads + id },
                nodeId = BenchNodeIds.DROP_TARGET_BUCKET,
                modifier = Modifier.frostedGlass(blurRadius = 16f),
            ) {
                Label("Drop Target Zone (${droppedPayloads.size} accepted)")
            }
            ScrollableText(
                document = "Scrollable multiline Vulkan text viewport with proportional scrollbar...",
            )
            ContextMenuRegion(
                items = listOf(
                    "Copy Widget Label",
                    "Toggle Fire Shader",
                    "Set Progress 100%",
                    "Toggle Light/Dark Mode",
                ),
                onSelectItem = { idx ->
                    when (idx) {
                        2 -> progress = 1.0f
                        3 -> activeMode = activeMode.toggle()
                    }
                },
            ) {
                Label("Right-Click (Desktop) or 450ms Long-Press (Android) for Frosted-Glass Context Menu")
            }
        }

        // Section 3: Lazy-Loaded List (10,000 items) with Proportional Scrollbar
        Column {
            LazyColumn(items = lazyItems, showScrollbar = true) { index, item ->
                Label("$index: $item")
            }
        }
    }

    IguanodonTheme(themeId = activeTheme, mode = activeMode) {
        Column {
            // Top Header Bar with Renderer Toggle (Iguanodon vs Compose-UI), Power Mode Toggle (120FPS vs 30FPS), Theme Switcher, Light/Dark Mode Switcher & Performance Telemetry Indicator
            Row(modifier = Modifier.frostedGlass(blurRadius = 18f)) {
                Label(
                    text = "Iguanodon Studio — Standard Benchmark App",
                    nodeId = BenchNodeIds.HEADER_LABEL,
                )
                Button(
                    label = "UI: ${activeRenderer.displayName}",
                    nodeId = BenchNodeIds.RENDERER_BACKEND_BTN,
                ) {
                    activeRenderer = activeRenderer.toggle()
                }
                Button(
                    label = "Pwr: ${activePowerMode.displayName}",
                    nodeId = BenchNodeIds.POWER_MODE_BTN,
                ) {
                    activePowerMode = activePowerMode.toggle()
                }
                Button(
                    label = "Theme: ${activeTheme.displayName}",
                    nodeId = BenchNodeIds.THEME_SWITCHER_BTN,
                ) {
                    val all = IguanodonThemeId.entries
                    activeTheme = all[(activeTheme.ordinal + 1) % all.size]
                }
                Button(
                    label = "Mode: ${activeMode.displayName}",
                    nodeId = BenchNodeIds.MODE_SWITCHER_BTN,
                ) {
                    activeMode = activeMode.toggle()
                }
                PerfOverlay()
            }

            if (isPortrait) {
                // Portrait Orientation: Single Vertical Column
                Column {
                    mainSections()
                }
            } else {
                // Landscape Orientation: 3 Side-by-Side Columns
                Row {
                    mainSections()
                }
            }
        }
    }
}
