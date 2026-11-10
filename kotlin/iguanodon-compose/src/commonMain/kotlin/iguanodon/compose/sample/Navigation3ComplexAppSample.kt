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
 * Type-safe Compose Multiplatform **Navigation 3** route keys (`NavKey`)
 * (`https://kotlinlang.org/docs/multiplatform/compose-navigation-3.html`).
 */
sealed interface StudioNavKey : NavKey {
    data object DashboardKey : StudioNavKey
    data class InspectorDetailKey(val assetId: String) : StudioNavKey
    data object ShaderGraphKey : StudioNavKey
    data object PreferencesKey : StudioNavKey
}

/**
 * Reference implementation of a complex multi-screen Compose Multiplatform application on Iguanodon
 * combining:
 * 1. **Navigation 3** (`rememberNavBackStack`, `NavDisplay`, `entryProvider`, `TwoPaneSplitSceneStrategy`)
 * 2. **Adaptive `NavigationSuiteScaffold`** (`NavigationRail` on Linux/Landscape, `NavigationBar` on Android Portrait)
 * 3. **Interactive `DraggablePanel` & `HorizontalSplitPane`** (`QSplitter` / `GtkPaned` / `QDockWidget` equivalents)
 * 4. **All 4 Themes in both Light & Dark Modes**
 */
@Composable
fun Navigation3ComplexAppSample() {
    var activeTheme by remember { mutableStateOf(IguanodonThemeId.Material3Expressive) }
    var darkTheme by remember { mutableStateOf(true) }
    var splitRatio by remember { mutableStateOf(0.35f) }
    var showFloatingInspector by remember { mutableStateOf(true) }

    // 1. Initialize Navigation 3 back stack with a starting destination
    val backStack = rememberNavBackStack<StudioNavKey>(StudioNavKey.DashboardKey)
    val currentKey = backStack.currentKey ?: StudioNavKey.DashboardKey

    MaterialTheme(themeId = activeTheme, darkTheme = darkTheme) {
        NavigationSuiteScaffold(
            navigationSuiteItems = {
                NavigationRailItem(
                    selected = currentKey is StudioNavKey.DashboardKey || currentKey is StudioNavKey.InspectorDetailKey,
                    onClick = { backStack.replaceTopLevel(StudioNavKey.DashboardKey) },
                    label = { Text("Dashboard") },
                )
                NavigationRailItem(
                    selected = currentKey is StudioNavKey.ShaderGraphKey,
                    onClick = { backStack.replaceTopLevel(StudioNavKey.ShaderGraphKey) },
                    label = { Text("Shaders") },
                )
                NavigationRailItem(
                    selected = currentKey is StudioNavKey.PreferencesKey,
                    onClick = { backStack.replaceTopLevel(StudioNavKey.PreferencesKey) },
                    label = { Text("Settings") },
                )
            },
        ) {
            Scaffold(
                topBar = {
                    TopAppBar(
                        title = { Text("Iguanodon Navigation 3 Studio (${if (darkTheme) "Dark" else "Light"})") },
                        actions = {
                            if (backStack.canGoBack) {
                                Button(label = "← Back") {
                                    backStack.removeLastOrNull()
                                }
                            }
                            Button(label = "Theme: ${activeTheme.displayName}") {
                                val all = IguanodonThemeId.entries
                                activeTheme = all[(activeTheme.ordinal + 1) % all.size]
                            }
                            Button(label = if (darkTheme) "Mode: Dark" else "Mode: Light") {
                                darkTheme = !darkTheme
                            }
                        },
                    )
                },
            ) {
                // 2. Render active destination(s) using Navigation 3 NavDisplay + entryProvider
                NavDisplay(
                    backStack = backStack,
                    onBack = { backStack.removeLastOrNull() },
                    sceneStrategy = TwoPaneSplitSceneStrategy(initialSplitRatio = splitRatio),
                    entryProvider = entryProvider {
                        entry<StudioNavKey.DashboardKey> {
                            HorizontalSplitPane(
                                splitRatio = splitRatio,
                                onSplitRatioChange = { splitRatio = it },
                                first = {
                                    Column {
                                        Text("Scene Hierarchy (Click to Push Detail):")
                                        Button(label = "Inspect Asset #01 (Vulkan SDF)") {
                                            backStack.add(StudioNavKey.InspectorDetailKey("Vulkan-SDF-01"))
                                        }
                                        Button(label = "Inspect Asset #02 (Water Drop 1080p)") {
                                            backStack.add(StudioNavKey.InspectorDetailKey("Water-Drop-1080p"))
                                        }
                                    }
                                },
                                second = {
                                    Column {
                                        StandardBenchmarkApp()
                                        if (showFloatingInspector) {
                                            DraggablePanel(
                                                title = "Draggable Telemetry Dock",
                                                onClose = { showFloatingInspector = false },
                                            ) {
                                                PerfOverlay()
                                            }
                                        }
                                    }
                                },
                            )
                        }

                        entry<StudioNavKey.InspectorDetailKey> { key ->
                            Card {
                                Column {
                                    Text("Inspector Detail Route: ${key.assetId}")
                                    Button(label = "Pop Detail Route (backStack.removeLastOrNull())") {
                                        backStack.removeLastOrNull()
                                    }
                                }
                            }
                        }

                        entry<StudioNavKey.ShaderGraphKey> {
                            DraggablePanel(title = "Procedural Fire & Frosted Glass Node Graph") {
                                Text("Drag panel header to reposition or snap to left dock edge.")
                            }
                        }

                        entry<StudioNavKey.PreferencesKey> {
                            AdwPreferencesPage {
                                AdwPreferencesGroup(
                                    title = "Appearance & Theme Polarity",
                                    description = "Supports Material 3 Expressive, Fluent 2, iOS Liquid Glass, and Linux Elite in Light & Dark modes",
                                ) {
                                    AdwSwitchRow(
                                        title = "Dark Mode",
                                        subtitle = "Toggle between Dark and Light palettes",
                                        active = darkTheme,
                                        onActiveChanged = { darkTheme = it },
                                    )
                                }
                            }
                        }
                    },
                )
            }
        }
    }
}
