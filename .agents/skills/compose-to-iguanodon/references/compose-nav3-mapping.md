# Compose Multiplatform & Navigation 3 to Iguanodon Mapping Reference

## 1. Navigation 3 (`androidx.navigation3` $\leftrightarrow$ `iguanodon.compose`)

| Compose Navigation 3 API | Iguanodon Equivalent (`Navigation3.kt`) | Behavior |
| :--- | :--- | :--- |
| `interface NavKey` | `iguanodon.compose.NavKey` | Marker interface for `@Serializable` route keys (`data object`, `data class`). |
| `rememberNavBackStack(StartKey)` | `rememberNavBackStack(StartKey)` | Returns observable `NavBackStack<T>` implementing `MutableList<T>`. |
| `backStack.add(DetailKey(id))` | `backStack.add(DetailKey(id))` | Pushes a new route onto the back stack with 120Hz forward spring transition. |
| `backStack.removeLastOrNull()` | `backStack.removeLastOrNull()` | Pops the top route when `size > 1`. |
| `entryProvider { entry<Key> { ... } }` | `entryProvider { entry<Key> { ... } }` | Type-safe mapping from `KClass<out T>` to `@Composable (K) -> Unit`. |
| `NavDisplay(backStack, onBack, entryProvider)` | `NavDisplay(backStack, onBack, sceneStrategy, entryProvider)` | Renders active entry or adaptive two-pane master-detail split via `TwoPaneSplitSceneStrategy`. |

### Migrating Navigation 2 (`NavHost`) to Navigation 3 on Iguanodon
```kotlin
// BEFORE (Navigation 2)
val navController = rememberNavController()
NavHost(navController, startDestination = HomeRoute) {
    composable<HomeRoute> { HomeScreen(onOpen = { navController.navigate(DetailRoute(it)) }) }
    composable<DetailRoute> { entry -> DetailScreen(entry.toRoute<DetailRoute>().id) }
}

// AFTER (Navigation 3 on Iguanodon)
val backStack = rememberNavBackStack<AppKey>(HomeKey)
NavDisplay(
    backStack = backStack,
    sceneStrategy = TwoPaneSplitSceneStrategy(),
    entryProvider = entryProvider {
        entry<HomeKey> { HomeScreen(onOpen = { backStack.add(DetailKey(it)) }) }
        entry<DetailKey> { key -> DetailScreen(key.id, onBack = { backStack.removeLastOrNull() }) }
    }
)
```

## 2. Navigation Bars, Rails, Draggable Panels, Chips & Themes

| Compose Component | Iguanodon `@Composable` (`ComposePortingCompat.kt` / `WidgetsAndModifiers.kt`) | Notes |
| :--- | :--- | :--- |
| `NavigationBar` + `NavigationBarItem` | `NavigationBar(scrollable = false)` / `ScrollableNavigationBar` | Frosted-glass bottom bar ($\ge 64\text{dp}$ height, $\ge 48\text{dp}$ items); set `scrollable = true` when there are 5+ tabs. |
| `NavigationRail` + `NavigationRailItem` | `NavigationRail` + `NavigationRailItem` | Frosted-glass side rail ($\ge 88\text{dp}$ width, $\ge 48\text{dp}$ items) with built-in `.verticalScroll(rememberScrollState())` for `360dp`–`420dp` phone landscape. |
| `NavigationSuiteScaffold` | `NavigationSuiteScaffold(..., bottomBarContent, railContent, drawerContent, usePermanentDrawer, showBottomBar, scrollableBottomBar)` | 3-tier adaptive scaffold (`NavigationBar` in portrait, vertically scrollable `NavigationRail` in medium/landscape, and optional permanent drawer on expanded desktop) with `showBottomBar` toggle for full-screen mobile detail/chat views. |
| `Row` with multiple `FilterChip`s / `Button`s | `ScrollableChipRow` / `HorizontalActionRow` | Horizontally scrollable `Row` (`Modifier.fillMaxWidth().horizontalScroll(rememberScrollState())`) preventing 1-character-per-line vertical wrapping on narrow portrait screens. |
| `HorizontalSplitPane` / `VerticalSplitPane` | `HorizontalSplitPane(splitRatio, onSplitRatioChange)` / `VerticalSplitPane(splitRatio, onSplitRatioChange)` | 48dp interactive draggable divider sash (`DraggableSplitPane`) with `remember(splitRatio)` and `onSplitRatioChange` callback for persisting split state across rotations/routes. |
| `DraggablePanel` / `ModalBottomSheet` | `DraggablePanel` / `ModalBottomSheet` | 48dp draggable header bar (`FontWeight.Normal` title) + snap-to-edge docking (`DraggableFloatingPanel`). |
| `MaterialTheme(colorScheme)` | `MaterialExpressiveTheme(themeId, darkTheme, fontScale)` / `IguanodonTheme(themeId, mode, fontScale)` | Supports all 4 themes (`Material3Expressive`, `WindowsFluent2`, `IosLiquidGlass`, `LinuxEliteObsidian`) in `Dark` & `Light` modes with built-in `fontScale` (`0.75f..2.0f`) via `LocalDensity`, public `resolveIguanodonColorScheme(themeId, darkTheme)`, and `LocalIguanodonThemeId` / `LocalIguanodonThemeMode` / `LocalIguanodonFontScale`. |
| `SurfaceView` / `ComposeView` host | `IguanodonVulkanSurfaceView` (`androidMain`) | Forwards `WindowInsets` (`v.onApplyWindowInsets(insets)`) to child `ComposeView`, clamps landscape viewport height to `240dp` min, and automatically syncs status/navigation bar icon polarity (`isAppearanceLightStatusBars = !darkTheme`) on theme changes. |

## 3. Dual-Renderer (`-Piguanodon.renderer=iguanodon|compose-ui`) Host & Bridge Pattern

To compare **Iguanodon** (`IguanodonApplier` + `libiguanodon_ffi.so`) and **Compose-UI** (`androidx.compose.ui`) from the exact same codebase without duplicating business logic or `@Composable` screens:

### A. Unified Host Entrypoints (`mountAppDesktop` / `mountAppAndroid`)
Define a small platform launcher function in `rendererIguanodonMain` and `rendererComposeUiMain`:

```kotlin
// src/rendererIguanodonMain/kotlin/app/host/AppRendererHost.desktop.kt
package app.host

import androidx.compose.runtime.Composable
import iguanodon.compose.desktop.IguanodonDesktopWindow
import javax.swing.SwingUtilities

fun mountDesktopApp(title: String, content: @Composable () -> Unit) {
    SwingUtilities.invokeLater {
        IguanodonDesktopWindow(title = title, content = content)
    }
}
```

```kotlin
// src/rendererComposeUiMain/kotlin/app/host/AppRendererHost.desktop.kt
package app.host

import androidx.compose.runtime.Composable
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application

fun mountDesktopApp(title: String, content: @Composable () -> Unit) = application {
    Window(onCloseRequest = ::exitApplication, title = title) {
        content()
    }
}
```

And on Android:
```kotlin
// src/rendererIguanodonMain/kotlin/app/host/AppRendererHost.android.kt
package app.host

import android.app.Activity
import androidx.compose.runtime.Composable
import iguanodon.compose.android.IguanodonVulkanSurfaceView

fun Activity.mountAndroidApp(content: @Composable () -> Unit) {
    setContentView(IguanodonVulkanSurfaceView(this, content = content))
}
```

```kotlin
// src/rendererComposeUiMain/kotlin/app/host/AppRendererHost.android.kt
package app.host

import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable

fun ComponentActivity.mountAndroidApp(content: @Composable () -> Unit) {
    setContent(content = content)
}
```

### B. Sharing `@Composable` Screens Across Both Renderers (`rendererComposeUiMain` Typealias Shim)
When `@Composable` screens in `commonMain` import `iguanodon.compose.*`, `src/rendererComposeUiMain/kotlin/iguanodon/compose/ComposeUiBridge.kt` can re-export standard `androidx.compose.*` primitives under `package iguanodon.compose` (with no-op fallbacks for Iguanodon-specific Vulkan shader modifiers like `Modifier.smokeBorder` / `Modifier.fireBorder`):

```kotlin
// src/rendererComposeUiMain/kotlin/iguanodon/compose/ComposeUiBridge.kt
package iguanodon.compose

import androidx.compose.runtime.Composable

typealias Modifier = androidx.compose.ui.Modifier
typealias Alignment = androidx.compose.ui.Alignment
typealias Arrangement = androidx.compose.foundation.layout.Arrangement
typealias Color = androidx.compose.ui.graphics.Color
typealias Dp = androidx.compose.ui.unit.Dp
typealias TextUnit = androidx.compose.ui.unit.TextUnit
typealias RowScope = androidx.compose.foundation.layout.RowScope
typealias ColumnScope = androidx.compose.foundation.layout.ColumnScope
typealias BoxScope = androidx.compose.foundation.layout.BoxScope

// Vulkan shader modifiers gracefully degrade to no-ops when comparing on stock compose-ui:
fun Modifier.smokeBorder(
    borderWidthDp: Float = 4f,
    plumeReachPx: Float = 24f,
    dissipation: Float = 0.55f,
): Modifier = this

fun Modifier.fireBorder(
    borderWidthDp: Float = 4f,
    coronaReachPx: Float = 28f,
    intensity: Float = 1.15f,
): Modifier = this
```
