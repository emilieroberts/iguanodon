---
name: compose-to-iguanodon
description: >-
  Version 1 workflow and toolkit for porting Jetpack Compose (Android) and Compose Multiplatform
  (Android and Linux Desktop, including Navigation 3, NavigationBar, NavigationRail, and
  DraggablePanel/SplitPane) applications to Iguanodon. Use this skill whenever the user wants
  to port or migrate an Android Compose or Compose Multiplatform app to Iguanodon.
---

# Porting Jetpack Compose & Compose Multiplatform (Navigation 3) Apps to Iguanodon (Linux & Android) — v1.0

This skill guides you through migrating an existing **Jetpack Compose (Android)** or **Compose Multiplatform (Android + Linux Desktop)** project — including complex apps with **Navigation 3** (`NavDisplay`, `rememberNavBackStack`, `entryProvider`), **`NavigationBar`**, **`NavigationRail`**, and **`DraggablePanel` / `SplitPane`** — onto **Iguanodon**.

## Pure `androidx.compose.runtime` Architecture (Zero `compose-ui` Dependency)

`:iguanodon-compose` depends **only** on `androidx.compose.runtime` (`compose.runtime`) + `kotlinx.coroutines` + `kotlinx.serialization.json` — with **zero dependency on `compose-ui`, `compose-foundation`, or `compose-material3` (`LayoutNode` / Skia / Android `ComposeView`)**:
- **Custom Applier & Composition Driver**: [`IguanodonApplier`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/IguanodonApplier.kt) (`AbstractApplier<IguanodonNode>`) and [`IguanodonCompositionHost`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/IguanodonApplier.kt) (`Recomposer` + `BroadcastFrameClock` + `Composition`) drive `@Composable` trees directly into the Rust `libiguanodon_ffi.so` retained scene graph via `ComposeNode<IguanodonNode, IguanodonApplier>`.
- **Retained Binary Scene Opcodes (`0x10..0x1A`) & Reverse Event Polling**: Tree mutations (`INSERT_CHILD`, `MOVE_CHILDREN`, `SET_LAYOUT_STYLE`, `SET_VISUAL_STYLE`, `SET_SHADER_EFFECT`, `SET_TEXT`, `SET_IMAGE_BYTES`, `SET_SCROLL_STATE`, `SET_EVENT_MASK`, `REMOVE_CHILDREN`, `CLEAR_TREE`) flush in a single binary buffer per frame, and `iguanodon_engine_poll_events` (`nativePollEvents`) drains 32-byte `ReverseUiEvent` records back to Kotlin callbacks.
- **Complete Pure-Runtime Toolkit**: [`WidgetsAndModifiers.kt`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/WidgetsAndModifiers.kt) and [`ComposePortingCompat.kt`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/compat/ComposePortingCompat.kt) provide pure-runtime implementations of `Dp`, `TextUnit`, `Color`, `Modifier`, `Row`, `Column`, `Box`, `Surface`, `Card`, `Scaffold`, `NavigationSuiteScaffold`, `ScrollableNavigationBar`, `ScrollableChipRow`, `HorizontalSplitPane`, `VerticalSplitPane`, `LazyColumn`, `HorizontalPager`, `Dialog`, `ModalBottomSheet`, `DropdownMenu`, `Canvas`, `Icons`, and all 4 Iguanodon themes in Light & Dark modes (`resolveIguanodonColorScheme`).

## Workflow Steps

### 1. Scan & Analyze the Source Compose Project
Run the Compose scanner script to inventory all `@Composable` functions, `androidx.compose.*` imports, Navigation 2 (`NavHost`) or Navigation 3 (`NavDisplay` / `rememberNavBackStack`) graphs, and adaptive navigation rails/bars:

```bash
python3 skills/compose-to-iguanodon/scripts/scan_compose_project.py <path-to-compose-project>
```
- Scanner script: [scan_compose_project.py](./scripts/scan_compose_project.py)
- Automated import rewriter: [migrate_compose_imports.sh](../../../scripts/migrate_compose_imports.sh)
- Compose & Navigation 3 mapping reference: [compose-nav3-mapping.md](./references/compose-nav3-mapping.md)

### 2. Recommended Dual-Renderer Build Model (`-Piguanodon.renderer=iguanodon|compose-ui`)
When porting an existing Compose or Compose Multiplatform app, **recommend configuring build-time renderer selection** via a Gradle property (`iguanodon.renderer=iguanodon` vs `iguanodon.renderer=compose-ui`). This keeps 100% of business logic, `ViewModel`s, repositories, and state holders clean, and lets developers A/B compare startup time, memory footprint, frame pacing, and visual fidelity between Iguanodon (`IguanodonApplier` + `libiguanodon_ffi.so` Vulkan SDF) and standard `compose-ui` (Skia / Android `ComposeView`).

#### A. `gradle.properties`
```properties
# Select UI renderer at build time:
#   iguanodon  -> Pure androidx.compose.runtime + IguanodonApplier + libiguanodon_ffi.so (zero compose-ui)
#   compose-ui -> Standard Jetpack Compose / Compose Multiplatform (Skiko / Android ComposeView)
iguanodon.renderer=iguanodon
```

Switch renderers on any Gradle invocation without modifying source files:
```bash
# Build / run with Iguanodon Vulkan SDF renderer (default)
./gradlew assembleDebug -Piguanodon.renderer=iguanodon
./gradlew runDesktop -Piguanodon.renderer=iguanodon

# Build / run with stock Compose-UI renderer for side-by-side comparison
./gradlew assembleDebug -Piguanodon.renderer=compose-ui
./gradlew runDesktop -Piguanodon.renderer=compose-ui
```

#### B. Source Set Architecture & `build.gradle.kts`
Keep domain logic, state holders (`@Composable` state + `remember` / `LaunchedEffect` / `collectAsState`), and shared `@Composable` screens in `commonMain`, and conditionally wire the renderer dependency and platform host source directories in `build.gradle.kts`:

```
app/src/
├── commonMain/kotlin/              # 100% shared ViewModels, repositories, NavKey routes, and @Composable screens
├── rendererIguanodonMain/kotlin/   # Platform host glue mounting IguanodonDesktopWindow / IguanodonVulkanSurfaceView
└── rendererComposeUiMain/kotlin/   # Platform host glue mounting androidx.compose.ui Window / Activity.setContent + typealias shim
```

```kotlin
val uiRenderer = providers.gradleProperty("iguanodon.renderer")
    .orElse("iguanodon")
    .get()
    .lowercase()
val useIguanodonRenderer = (uiRenderer != "compose-ui")

kotlin {
    androidTarget()
    jvm("desktop")

    sourceSets {
        val commonMain by getting {
            kotlin.srcDir(
                if (useIguanodonRenderer) "src/rendererIguanodonMain/kotlin"
                else "src/rendererComposeUiMain/kotlin"
            )
            dependencies {
                // Always present in both modes: pure Compose runtime + coroutines + serialization
                implementation(compose.runtime)
                implementation(libs.kotlinx.coroutines.core)
                implementation(libs.kotlinx.serialization.json)

                if (useIguanodonRenderer) {
                    // Pure-runtime Iguanodon renderer (zero compose-ui / Skia dependency)
                    implementation(project(":iguanodon-compose"))
                } else {
                    // Stock Compose-UI renderer for side-by-side comparison
                    implementation(compose.ui)
                    implementation(compose.foundation)
                    implementation(compose.material3)
                    implementation(compose.materialIconsExtended)
                }
            }
        }
    }
}
```

#### C. Distributable Binary or Source Dependency Options for `:iguanodon-compose`
When `useIguanodonRenderer` is active, choose either:
1. **Drop-In Binary Libraries (`dist/`)**:
   - **Android (`androidMain` / `app`)**: Copy [`dist/iguanodon-compose-android-0.0.1-alpha.aar`](../../../dist/iguanodon-compose-android-0.0.1-alpha.aar) into `libs/` and add `implementation(files("libs/iguanodon-compose-android-0.0.1-alpha.aar"))`.
   - **Linux Desktop (`desktopMain` / `jvmMain`)**: Copy [`dist/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar`](../../../dist/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar) into `libs/` and add `implementation(files("libs/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar"))`.
2. **Gradle Submodule or Composite Build (`commonMain` Full Source Sharing)**:
   - Include `:iguanodon-compose` (from `<path-to-iguanodon>/kotlin/iguanodon-compose`) in `settings.gradle.kts` and add `implementation(project(":iguanodon-compose"))` in `commonMain.dependencies`.
   - **Offline Gradle Cache Tip**: In `iguanodon-compose/build.gradle.kts`, use `alias(libs.plugins.androidApplication) apply false` (or `id("com.android.application") apply false`) followed by `if (hasAndroidSdk) apply(plugin = "com.android.library")` so builds with `--offline` succeed even if the `com.android.library` plugin marker artifact is not cached separately from `com.android.application`.

### 3. Migrate UI Imports While Preserving `androidx.compose.runtime.*`
Run the import migration script so `androidx.compose.material3.*`, `androidx.compose.foundation.*`, and `androidx.compose.ui.*` resolve to `iguanodon.compose.*` ([`ComposePortingCompat.kt`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/compat/ComposePortingCompat.kt) and [`Navigation3.kt`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/navigation3/Navigation3.kt)):

```bash
./scripts/migrate_compose_imports.sh <path-to-compose-project> --apply
```
- **Keep 100% unchanged**: `import androidx.compose.runtime.*` (`remember`, `mutableStateOf`, `derivedStateOf`, `LaunchedEffect`, `CompositionLocalProvider`, `collectAsState`), `ViewModel`s, Coroutines, and Ktor/SQLDelight/Room repositories.

### 4. Wire Navigation 3, Adaptive Rails/Bars, and Draggable Panels
1. **Navigation 3 (`https://kotlinlang.org/docs/multiplatform/compose-navigation-3.html`)**:
   - Route keys implement [`NavKey`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/navigation3/Navigation3.kt).
   - Back stack uses `val backStack = rememberNavBackStack(HomeKey)` (`backStack.add(DetailKey(id))` to push, `backStack.removeLastOrNull()` to pop).
   - Screen host uses `NavDisplay(backStack = backStack, sceneStrategy = TwoPaneSplitSceneStrategy(), entryProvider = entryProvider { entry<HomeKey> { ... } })`.
   - If the source project still uses Navigation 2 (`NavHost` + `rememberNavController`), convert `navController.navigate(Route)` to `backStack.add(Route)` and `composable<Route>` to `entry<Route>`.
2. **Adaptive Navigation Bars & Rails**:
   - Wrap top-level destinations in [`NavigationSuiteScaffold`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/compat/ComposePortingCompat.kt), which automatically renders a bottom `NavigationBar` in Android Portrait, a side `NavigationRail` in Medium/Landscape, and an optional `PermanentNavigationDrawer` on Expanded Desktop (`>= 1100.dp`).
   - **5+ Navigation Destinations Gotcha**:
     - In Portrait, standard `NavigationBar` clips >5 items or scaled fonts. Pass `scrollableBottomBar = true` to `NavigationSuiteScaffold` or use `ScrollableNavigationBar` (`Row(Modifier.horizontalScroll(rememberScrollState()))`) when there are 5+ tabs.
     - In Phone Landscape, viewport height is only `360dp`–`420dp`. Always make `NavigationRail` vertically scrollable (`Column(Modifier.verticalScroll(rememberScrollState()))` — enabled by default in Iguanodon's `NavigationRail` and `NavigationSuiteScaffold`) so items never overlap vertically.
3. **Mobile Chip & Button Rows (Prevent Vertical 1-Char Text Wrapping)**:
   - Never place >2 `FilterChip`s, `AssistChip`s, or action `Button`s in a plain `Row` without `.horizontalScroll(rememberScrollState())`. On narrow portrait screens, an unscrollable `Row` crushes trailing chips to a few pixels wide, causing `Text` to wrap 1 character per line vertically and creating huge empty spaces in cards. Use `ScrollableChipRow` / `HorizontalActionRow` (or add `.horizontalScroll(rememberScrollState())` on the `Row` and `maxLines = 1, softWrap = false` on chip/button `Text`).
4. **Typography & Configurable Font Scaling**:
   - Avoid heavy `FontWeight.Bold` / `FontWeight.ExtraBold` defaults across body/title styles; prefer `FontWeight.Normal` and `FontWeight.Medium`.
   - Pass user-configurable `fontScale` into `MaterialExpressiveTheme(themeId, darkTheme, fontScale)` or `IguanodonTheme(themeId, mode, fontScale)` (or query `resolveIguanodonColorScheme(themeId, darkTheme)` alongside `LocalIguanodonThemeId`, `LocalIguanodonThemeMode`, and `LocalIguanodonFontScale`) so users on high-DPI Linux monitors and Android phones can scale UI text (`75%`–`200%`).
5. **Host Entrypoints & Android Edge-to-Edge**:
   - **Android (`Activity.onCreate`)**:
     - Call `enableEdgeToEdge(statusBarStyle = SystemBarStyle.auto(Color.TRANSPARENT, Color.TRANSPARENT), navigationBarStyle = SystemBarStyle.auto(Color.TRANSPARENT, Color.TRANSPARENT))` and `WindowCompat.setDecorFitsSystemWindows(window, false)`.
     - Ensure `IguanodonVulkanSurfaceView`'s `OnApplyWindowInsetsListener` returns `v.onApplyWindowInsets(insets)` (not raw `insets`) so child components receive Android `WindowInsets`.
     - `IguanodonVulkanSurfaceView` automatically syncs `WindowCompat.getInsetsController(window, view).isAppearanceLightStatusBars = !darkTheme` (and `isAppearanceLightNavigationBars = !darkTheme`) whenever `IguanodonThemeMode` toggles between Light and Dark.
   - **Linux Desktop (`main()`)**: Mount [`IguanodonDesktopWindow`](../../../kotlin/iguanodon-compose/src/desktopMain/kotlin/iguanodon/compose/desktop/IguanodonDesktopWindow.kt).

### 5. Validate & Deploy (Linux & Android)
1. Run workspace unit tests:
   ```bash
   cargo test --workspace
   ```
2. Deploy to a connected Android phone via `adb`:
   ```bash
   ./scripts/run_android_phone.sh
   ```
