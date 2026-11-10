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

package iguanodon.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.ComposeNode
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import kotlin.math.roundToInt

/**
 * Pure `compose.runtime` drop-in Jetpack Compose, Compose Multiplatform (including Navigation 3,
 * NavigationBar, NavigationRail, DraggablePanel, SplitPane, ScrollableChipRow, and ScrollableNavigationBar),
 * Qt 6 (`QWidgets` / `QML`), and GTK 4 / Libadwaita source-compatibility layer for Iguanodon.
 *
 * Depends ONLY on `androidx.compose.runtime` — zero `compose-ui`, `compose-foundation`, or `compose-material3` imports.
 */

@Composable
fun isSystemInDarkTheme(): Boolean = LocalIguanodonThemeMode.current == IguanodonThemeMode.Dark

@Composable
fun MaterialTheme(
    themeId: IguanodonThemeId = IguanodonThemeId.Material3Expressive,
    darkTheme: Boolean = isSystemInDarkTheme(),
    fontScale: Float = 1.0f,
    content: @Composable () -> Unit,
) {
    MaterialExpressiveTheme(
        themeId = themeId,
        darkTheme = darkTheme,
        fontScale = fontScale,
        content = content,
    )
}

@Composable
fun MaterialTheme(
    colorScheme: ColorScheme = MaterialTheme.colorScheme,
    typography: Typography = MaterialTheme.typography,
    shapes: Shapes = MaterialTheme.shapes,
    content: @Composable () -> Unit,
) {
    CompositionLocalProvider(
        LocalColorScheme provides colorScheme,
        LocalTypography provides typography,
        LocalShapes provides shapes,
        LocalContentColor provides colorScheme.onSurface,
    ) {
        content()
    }
}

// ============================================================================
// WindowInsets & Edge-to-Edge Safe Area Modifiers
// ============================================================================

@Immutable
data class WindowInsets(
    val leftDp: Float = 0f,
    val topDp: Float = 0f,
    val rightDp: Float = 0f,
    val bottomDp: Float = 0f,
) {
    fun union(other: WindowInsets): WindowInsets = WindowInsets(
        leftDp = maxOf(leftDp, other.leftDp),
        topDp = maxOf(topDp, other.topDp),
        rightDp = maxOf(rightDp, other.rightDp),
        bottomDp = maxOf(bottomDp, other.bottomDp),
    )

    fun only(sides: Int): WindowInsets = WindowInsets(
        leftDp = if ((sides and WindowInsetsSides.Horizontal) != 0) leftDp else 0f,
        topDp = if ((sides and WindowInsetsSides.Top) != 0) topDp else 0f,
        rightDp = if ((sides and WindowInsetsSides.Horizontal) != 0) rightDp else 0f,
        bottomDp = if ((sides and WindowInsetsSides.Bottom) != 0) bottomDp else 0f,
    )

    companion object {
        val statusBars: WindowInsets
            @Composable get() {
                val s = LocalIguanodonSession.current
                return WindowInsets(topDp = s?.safeInsetTopDp ?: 0f)
            }
        val navigationBars: WindowInsets
            @Composable get() {
                val s = LocalIguanodonSession.current
                return WindowInsets(bottomDp = s?.safeInsetBottomDp ?: 0f)
            }
        val displayCutout: WindowInsets
            @Composable get() {
                val s = LocalIguanodonSession.current
                return WindowInsets(
                    leftDp = s?.safeInsetLeftDp ?: 0f,
                    topDp = s?.safeInsetTopDp ?: 0f,
                    rightDp = s?.safeInsetRightDp ?: 0f,
                )
            }
        val ime: WindowInsets
            @Composable get() = WindowInsets()
        val systemBars: WindowInsets
            @Composable get() = statusBars.union(navigationBars)
    }
}

object WindowInsetsSides {
    const val Left: Int = 1 shl 0
    const val Top: Int = 1 shl 1
    const val Right: Int = 1 shl 2
    const val Bottom: Int = 1 shl 3
    const val Horizontal: Int = Left or Right
    const val Vertical: Int = Top or Bottom
}

fun Modifier.windowInsetsPadding(insets: WindowInsets): Modifier = padding(
    start = insets.leftDp.dp,
    top = insets.topDp.dp,
    end = insets.rightDp.dp,
    bottom = insets.bottomDp.dp,
)

fun Modifier.statusBarsPadding(): Modifier = this
fun Modifier.navigationBarsPadding(): Modifier = this
fun Modifier.systemBarsPadding(): Modifier = this
fun Modifier.imePadding(): Modifier = this
fun Modifier.displayCutoutPadding(): Modifier = this

// ============================================================================
// 1. Navigation Bars, Navigation Rails, Drawers & Adaptive Navigation Suites
// ============================================================================

@Composable
fun NavigationBar(
    modifier: Modifier = Modifier,
    containerColor: Color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
    contentColor: Color = MaterialTheme.colorScheme.onSurface,
    tonalElevation: Dp = 0.dp,
    windowInsets: WindowInsets = WindowInsets(),
    scrollable: Boolean = false,
    content: @Composable RowScope.() -> Unit,
) {
    Surface(
        modifier = modifier
            .fillMaxWidth()
            .windowInsetsPadding(windowInsets)
            .heightIn(min = 64.dp)
            .frostedGlass(blurRadius = 20f),
        color = containerColor,
        contentColor = contentColor,
    ) {
        val rowMod = if (scrollable) {
            Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = 8.dp, vertical = 6.dp)
        } else {
            Modifier
                .fillMaxWidth()
                .padding(horizontal = 8.dp, vertical = 6.dp)
        }
        Row(
            modifier = rowMod,
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically,
            content = content,
        )
    }
}

/**
 * Horizontally scrollable bottom NavigationBar for apps with 5+ top-level destinations
 * or scaled UI typography (`fontScale > 1.0f`) so trailing tabs are never clipped in portrait.
 */
@Composable
fun ScrollableNavigationBar(
    modifier: Modifier = Modifier,
    content: @Composable RowScope.() -> Unit,
) = NavigationBar(
    modifier = modifier,
    scrollable = true,
    content = content,
)

@Composable
fun NavigationBarItem(
    selected: Boolean,
    onClick: () -> Unit,
    icon: @Composable () -> Unit = {},
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    label: (@Composable () -> Unit)? = null,
    alwaysShowLabel: Boolean = true,
) {
    Button(
        onClick = onClick,
        enabled = enabled,
        modifier = modifier,
    ) {
        icon()
        if ((alwaysShowLabel || selected) && label != null) {
            label()
        }
    }
}

@Composable
fun NavigationRail(
    modifier: Modifier = Modifier,
    containerColor: Color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
    contentColor: Color = MaterialTheme.colorScheme.onSurface,
    header: (@Composable ColumnScope.() -> Unit)? = null,
    windowInsets: WindowInsets = WindowInsets(),
    content: @Composable ColumnScope.() -> Unit,
) {
    Surface(
        modifier = modifier
            .fillMaxHeight()
            .windowInsetsPadding(windowInsets)
            .width(88.dp)
            .frostedGlass(blurRadius = 20f),
        color = containerColor,
        contentColor = contentColor,
    ) {
        Column(
            modifier = Modifier
                .fillMaxHeight()
                .verticalScroll(rememberScrollState())
                .padding(vertical = 8.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            header?.invoke(this)
            content()
        }
    }
}

@Composable
fun NavigationRailItem(
    selected: Boolean,
    onClick: () -> Unit,
    icon: @Composable () -> Unit = {},
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    label: (@Composable () -> Unit)? = null,
    alwaysShowLabel: Boolean = true,
) {
    Button(
        onClick = onClick,
        enabled = enabled,
        modifier = modifier,
    ) {
        icon()
        if ((alwaysShowLabel || selected) && label != null) {
            label()
        }
    }
}

/**
 * 3-Tier Adaptive Navigation Suite Scaffold:
 * - **Expanded Desktop (`usePermanentDrawer && drawerContent != null`, or `maxWidth >= 1100.dp`)**:
 *   Renders a left-side permanent drawer alongside `content()`.
 * - **Compact Portrait (`maxWidth < 600.dp` or `session.isPortrait`)**:
 *   Renders `content()` above a bottom `NavigationBar` (or custom `bottomBarContent`),
 *   honoring `showBottomBar` and `scrollableBottomBar` (for 5+ tabs).
 * - **Medium / Phone Landscape (`600.dp <= maxWidth < 1100.dp`)**:
 *   Renders a vertically scrollable `NavigationRail` (or custom `railContent`) so 5+ destinations
 *   never overlap in `360dp`–`420dp`-tall phone landscape viewports.
 */
@Composable
fun NavigationSuiteScaffold(
    navigationSuiteItems: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    bottomBarContent: (@Composable () -> Unit)? = null,
    railContent: (@Composable () -> Unit)? = null,
    drawerContent: (@Composable () -> Unit)? = null,
    usePermanentDrawer: Boolean = false,
    showBottomBar: Boolean = true,
    scrollableBottomBar: Boolean = false,
    content: @Composable () -> Unit,
) {
    val session = LocalIguanodonSession.current
    BoxWithConstraints(modifier = modifier.fillMaxSize()) {
        val isCompactPortrait = maxWidth < 600.dp || (session?.isPortrait == true && maxWidth < 700.dp)
        when {
            usePermanentDrawer && drawerContent != null -> {
                Row(modifier = Modifier.fillMaxSize()) {
                    drawerContent()
                    Box(modifier = Modifier.weight(1f).fillMaxHeight()) {
                        content()
                    }
                }
            }
            isCompactPortrait -> {
                Column(modifier = Modifier.fillMaxSize()) {
                    Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
                        content()
                    }
                    if (showBottomBar) {
                        if (bottomBarContent != null) {
                            bottomBarContent()
                        } else {
                            NavigationBar(
                                modifier = Modifier.frostedGlass(blurRadius = 20f),
                                scrollable = scrollableBottomBar,
                            ) {
                                navigationSuiteItems()
                            }
                        }
                    }
                }
            }
            else -> {
                Row(modifier = Modifier.fillMaxSize()) {
                    if (railContent != null) {
                        railContent()
                    } else {
                        NavigationRail(
                            modifier = Modifier.frostedGlass(blurRadius = 20f),
                        ) {
                            Column(
                                modifier = Modifier.verticalScroll(rememberScrollState()),
                                horizontalAlignment = Alignment.CenterHorizontally,
                            ) {
                                navigationSuiteItems()
                            }
                        }
                    }
                    Box(modifier = Modifier.weight(1f).fillMaxHeight()) {
                        content()
                    }
                }
            }
        }
    }
}

/**
 * Horizontally scrollable row for chips, tabs, or action buttons so items never wrap
 * into 1-character-per-line vertical columns on narrow portrait screens.
 */
@Composable
fun ScrollableChipRow(
    modifier: Modifier = Modifier,
    horizontalArrangement: Arrangement.Horizontal = Arrangement.spacedBy(8.dp),
    verticalAlignment: Alignment.Vertical = Alignment.CenterVertically,
    content: @Composable RowScope.() -> Unit,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState()),
        horizontalArrangement = horizontalArrangement,
        verticalAlignment = verticalAlignment,
        content = content,
    )
}

@Composable
fun HorizontalActionRow(
    modifier: Modifier = Modifier,
    horizontalArrangement: Arrangement.Horizontal = Arrangement.spacedBy(8.dp),
    verticalAlignment: Alignment.Vertical = Alignment.CenterVertically,
    content: @Composable RowScope.() -> Unit,
) = ScrollableChipRow(
    modifier = modifier,
    horizontalArrangement = horizontalArrangement,
    verticalAlignment = verticalAlignment,
    content = content,
)

@Composable
fun ModalNavigationDrawer(
    drawerContent: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    drawerOpen: Boolean = false,
    content: @Composable () -> Unit,
) {
    Row(modifier = modifier.fillMaxSize()) {
        if (drawerOpen) {
            Column(modifier = Modifier.frostedGlass(blurRadius = 24f)) {
                drawerContent()
            }
        }
        Box(modifier = Modifier.weight(1f).fillMaxHeight()) {
            content()
        }
    }
}

@Composable
fun PermanentNavigationDrawer(
    drawerContent: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Row(modifier = modifier.fillMaxSize()) {
        Column(modifier = Modifier.frostedGlass(blurRadius = 20f)) {
            drawerContent()
        }
        Box(modifier = Modifier.weight(1f).fillMaxHeight()) {
            content()
        }
    }
}

@Composable
fun PermanentDrawerSheet(
    modifier: Modifier = Modifier,
    drawerContainerColor: Color = MaterialTheme.colorScheme.surface,
    drawerContentColor: Color = MaterialTheme.colorScheme.onSurface,
    windowInsets: WindowInsets = WindowInsets(),
    content: @Composable ColumnScope.() -> Unit,
) {
    Surface(
        modifier = modifier
            .fillMaxHeight()
            .windowInsetsPadding(windowInsets)
            .frostedGlass(blurRadius = 20f),
        color = drawerContainerColor,
        contentColor = drawerContentColor,
    ) {
        Column(modifier = Modifier.fillMaxHeight(), content = content)
    }
}

@Composable
fun NavigationDrawerItem(
    label: @Composable () -> Unit,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    icon: (@Composable () -> Unit)? = null,
    badge: (@Composable () -> Unit)? = null,
) {
    Button(onClick = onClick, modifier = modifier) {
        icon?.invoke()
        label()
        badge?.invoke()
    }
}

@Composable
fun TabRow(
    selectedTabIndex: Int,
    modifier: Modifier = Modifier,
    tabs: @Composable () -> Unit,
) {
    Row(modifier = modifier.horizontalScroll(rememberScrollState())) { tabs() }
}

@Composable
fun PrimaryTabRow(
    selectedTabIndex: Int,
    modifier: Modifier = Modifier,
    tabs: @Composable () -> Unit,
) = TabRow(selectedTabIndex = selectedTabIndex, modifier = modifier, tabs = tabs)

@Composable
fun ScrollableTabRow(
    selectedTabIndex: Int,
    modifier: Modifier = Modifier,
    tabs: @Composable () -> Unit,
) = TabRow(selectedTabIndex = selectedTabIndex, modifier = modifier, tabs = tabs)

@Composable
fun Tab(
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    text: (@Composable () -> Unit)? = null,
    icon: (@Composable () -> Unit)? = null,
) {
    Button(onClick = onClick, modifier = modifier) {
        icon?.invoke()
        text?.invoke()
    }
}

// ============================================================================
// 2. Draggable Panels, Split Panes, Docks & Sheets
// ============================================================================

@Composable
fun DraggablePanel(
    title: String,
    modifier: Modifier = Modifier,
    initialDocked: Boolean = false,
    onClose: (() -> Unit)? = null,
    content: @Composable () -> Unit,
) {
    var offsetX by remember { mutableStateOf(0f) }
    var offsetY by remember { mutableStateOf(0f) }
    var isDocked by remember { mutableStateOf(initialDocked) }
    val scheme = MaterialTheme.colorScheme

    Surface(
        modifier = modifier
            .offset {
                if (isDocked) IntOffset.Zero else IntOffset(offsetX.roundToInt(), offsetY.roundToInt())
            }
            .clip(RoundedCornerShape(14.dp))
            .border(1.dp, scheme.outline.copy(alpha = 0.35f), RoundedCornerShape(14.dp))
            .frostedGlass(blurRadius = 22f),
        color = scheme.surface.copy(alpha = 0.94f),
    ) {
        Column {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(min = 48.dp)
                    .background(scheme.surfaceVariant.copy(alpha = 0.65f))
                    .pointerInput(Unit) {
                        detectDragGestures { change, dragAmount ->
                            change.consume()
                            isDocked = false
                            offsetX += dragAmount.x
                            offsetY += dragAmount.y
                            if (offsetX < -120f) {
                                isDocked = true
                                offsetX = 0f
                                offsetY = 0f
                            }
                        }
                    }
                    .padding(horizontal = 12.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(
                        text = title,
                        style = MaterialTheme.typography.labelLarge,
                        fontWeight = FontWeight.Normal,
                    )
                }
                if (onClose != null) {
                    Button(label = "×", onClick = onClose)
                }
            }
            Box(modifier = Modifier.padding(12.dp)) {
                content()
            }
        }
    }
}

@Composable
fun HorizontalSplitPane(
    splitRatio: Float = 0.36f,
    onSplitRatioChange: (Float) -> Unit = {},
    modifier: Modifier = Modifier,
    first: @Composable () -> Unit,
    second: @Composable () -> Unit,
) {
    var internalRatio by remember(splitRatio) { mutableStateOf(splitRatio.coerceIn(0.18f, 0.82f)) }
    val density = LocalDensity.current
    val scheme = MaterialTheme.colorScheme
    BoxWithConstraints(modifier = modifier.fillMaxSize()) {
        val totalWidthPx = with(density) { maxWidth.toPx().coerceAtLeast(1f) }
        Row(modifier = Modifier.fillMaxSize()) {
            Box(
                modifier = Modifier
                    .weight(internalRatio)
                    .fillMaxHeight(),
            ) {
                first()
            }
            Box(
                modifier = Modifier
                    .width(16.dp)
                    .fillMaxHeight()
                    .pointerInput(totalWidthPx) {
                        detectHorizontalDragGestures { change, dragAmount ->
                            change.consume()
                            val next = (internalRatio + dragAmount / totalWidthPx).coerceIn(0.18f, 0.82f)
                            internalRatio = next
                            onSplitRatioChange(next)
                        }
                    },
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    modifier = Modifier
                        .width(4.dp)
                        .height(48.dp)
                        .clip(RoundedCornerShape(2.dp))
                        .background(scheme.outline.copy(alpha = 0.55f)),
                )
            }
            Box(
                modifier = Modifier
                    .weight(1f - internalRatio)
                    .fillMaxHeight(),
            ) {
                second()
            }
        }
    }
}

@Composable
fun VerticalSplitPane(
    splitRatio: Float = 0.55f,
    onSplitRatioChange: (Float) -> Unit = {},
    modifier: Modifier = Modifier,
    top: @Composable () -> Unit,
    bottom: @Composable () -> Unit,
) {
    var internalRatio by remember(splitRatio) { mutableStateOf(splitRatio.coerceIn(0.20f, 0.80f)) }
    val density = LocalDensity.current
    val scheme = MaterialTheme.colorScheme
    BoxWithConstraints(modifier = modifier.fillMaxSize()) {
        val totalHeightPx = with(density) { maxHeight.toPx().coerceAtLeast(1f) }
        Column(modifier = Modifier.fillMaxSize()) {
            Box(
                modifier = Modifier
                    .weight(internalRatio)
                    .fillMaxWidth(),
            ) {
                top()
            }
            Box(
                modifier = Modifier
                    .height(16.dp)
                    .fillMaxWidth()
                    .pointerInput(totalHeightPx) {
                        detectVerticalDragGestures { change, dragAmount ->
                            change.consume()
                            val next = (internalRatio + dragAmount / totalHeightPx).coerceIn(0.20f, 0.80f)
                            internalRatio = next
                            onSplitRatioChange(next)
                        }
                    },
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    modifier = Modifier
                        .width(48.dp)
                        .height(4.dp)
                        .clip(RoundedCornerShape(2.dp))
                        .background(scheme.outline.copy(alpha = 0.55f)),
                )
            }
            Box(
                modifier = Modifier
                    .weight(1f - internalRatio)
                    .fillMaxWidth(),
            ) {
                bottom()
            }
        }
    }
}

@Composable
fun ModalBottomSheet(
    onDismissRequest: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Column(modifier = modifier.frostedGlass(blurRadius = 24f)) {
        content()
    }
}

@Composable
fun ExpandableCard(
    title: String,
    expanded: Boolean,
    onExpandedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Column(modifier = modifier) {
        Button(label = title, onClick = { onExpandedChange(!expanded) })
        if (expanded) {
            content()
        }
    }
}

// ============================================================================
// 3. Standard Compose Controls, Containers & Data Views
// ============================================================================

@Composable
fun Text(
    text: String,
    modifier: Modifier = Modifier,
    color: Color = Color.Unspecified,
    fontSize: TextUnit = TextUnit.Unspecified,
    fontStyle: FontStyle? = null,
    fontWeight: FontWeight? = null,
    fontFamily: FontFamily? = null,
    textAlign: TextAlign? = null,
    lineHeight: TextUnit = TextUnit.Unspecified,
    overflow: TextOverflow = TextOverflow.Clip,
    softWrap: Boolean = true,
    maxLines: Int = Int.MAX_VALUE,
    minLines: Int = 1,
    style: TextStyle = MaterialTheme.typography.bodyMedium,
    nodeId: Int = 0,
) {
    val session = LocalIguanodonSession.current
    val resolvedId = remember(nodeId) {
        if (nodeId != 0) nodeId.toLong() else (session?.allocateNodeId() ?: 0L)
    }
    val resolvedColor = when {
        color != Color.Unspecified -> color
        style.color != Color.Unspecified -> style.color
        else -> LocalContentColor.current
    }
    val resolvedSize = if (!fontSize.value.isNaN()) fontSize.value else style.fontSize.value
    val resolvedWeight = (fontWeight ?: style.fontWeight).weight

    ComposeNode<IguanodonNode, IguanodonApplier>(
        factory = { IguanodonNode(nodeId = resolvedId, nodeKind = NodeKinds.TEXT) },
        update = {
            set(modifier) { mod ->
                session?.syncModifierToNode(this, mod)
            }
            set(text) { str ->
                this.textValue = str
                session?.encoder?.encodeSetText(
                    nodeId = this.nodeId,
                    colorRgba = resolvedColor.toArgb(),
                    fontSizeSp = resolvedSize,
                    fontWeight = resolvedWeight,
                    maxLines = maxLines.coerceIn(1, 65535),
                    text = str,
                )
            }
        },
    )
}

@Composable
fun Text(
    text: AnnotatedString,
    modifier: Modifier = Modifier,
    color: Color = Color.Unspecified,
    fontSize: TextUnit = TextUnit.Unspecified,
    fontStyle: FontStyle? = null,
    fontWeight: FontWeight? = null,
    fontFamily: FontFamily? = null,
    textAlign: TextAlign? = null,
    lineHeight: TextUnit = TextUnit.Unspecified,
    overflow: TextOverflow = TextOverflow.Clip,
    softWrap: Boolean = true,
    maxLines: Int = Int.MAX_VALUE,
    minLines: Int = 1,
    style: TextStyle = MaterialTheme.typography.bodyMedium,
) = Text(
    text = text.text,
    modifier = modifier,
    color = color,
    fontSize = fontSize,
    fontStyle = fontStyle,
    fontWeight = fontWeight,
    fontFamily = fontFamily,
    textAlign = textAlign,
    lineHeight = lineHeight,
    overflow = overflow,
    softWrap = softWrap,
    maxLines = maxLines,
    minLines = minLines,
    style = style,
)

@Composable
fun Button(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    shape: Shape = RoundedCornerShape(12.dp),
    contentPadding: PaddingValues = PaddingValues(horizontal = 12.dp, vertical = 8.dp),
    nodeId: Int = 0,
    content: @Composable RowScope.() -> Unit,
) {
    Row(
        modifier = modifier
            .heightIn(min = 48.dp)
            .clip(shape)
            .clickable(enabled = enabled, onClick = onClick)
            .padding(contentPadding),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
        content = content,
    )
}

@Composable
fun OutlinedButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    shape: Shape = RoundedCornerShape(12.dp),
    border: BorderStroke? = null,
    contentPadding: PaddingValues = PaddingValues(horizontal = 12.dp, vertical = 8.dp),
    content: @Composable RowScope.() -> Unit,
) = Button(
    onClick = onClick,
    modifier = if (border != null) modifier.border(border, shape) else modifier,
    enabled = enabled,
    shape = shape,
    contentPadding = contentPadding,
    content = content,
)

@Composable
fun FilledTonalButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable RowScope.() -> Unit,
) = Button(onClick = onClick, modifier = modifier, enabled = enabled, content = content)

@Composable
fun ElevatedButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable RowScope.() -> Unit,
) = Button(onClick = onClick, modifier = modifier, enabled = enabled, content = content)

@Composable
fun TextButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    contentPadding: PaddingValues = PaddingValues(horizontal = 8.dp, vertical = 6.dp),
    content: @Composable RowScope.() -> Unit,
) = Button(
    onClick = onClick,
    modifier = modifier,
    enabled = enabled,
    contentPadding = contentPadding,
    content = content,
)

@Composable
fun IconButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable () -> Unit,
) = Button(onClick = onClick, modifier = modifier, enabled = enabled) { content() }

@Composable
fun FloatingActionButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) = Button(onClick = onClick, modifier = modifier) { content() }

@Composable
fun ExtendedFloatingActionButton(
    text: @Composable () -> Unit,
    icon: @Composable () -> Unit = {},
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) = Button(onClick = onClick, modifier = modifier) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        icon()
        text()
    }
}

@Composable
fun Surface(
    modifier: Modifier = Modifier,
    shape: Shape = RectangleShape,
    color: Color = MaterialTheme.colorScheme.surface,
    contentColor: Color = MaterialTheme.colorScheme.onSurface,
    tonalElevation: Dp = 0.dp,
    shadowElevation: Dp = 0.dp,
    border: BorderStroke? = null,
    content: @Composable () -> Unit,
) {
    val session = LocalIguanodonSession.current
    val nodeId = remember { session?.allocateNodeId() ?: 0L }
    val surfMod = modifier
        .background(color, shape)
        .let { if (border != null) it.border(border, shape) else it }
    CompositionLocalProvider(LocalContentColor provides contentColor) {
        ComposeNode<IguanodonNode, IguanodonApplier>(
            factory = { IguanodonNode(nodeId = nodeId, nodeKind = NodeKinds.SURFACE) },
            update = {
                set(surfMod) { mod ->
                    session?.syncModifierToNode(this, mod, flexDir = 2)
                }
            },
            content = content,
        )
    }
}

@Composable
fun Surface(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    shape: Shape = RectangleShape,
    color: Color = MaterialTheme.colorScheme.surface,
    contentColor: Color = MaterialTheme.colorScheme.onSurface,
    tonalElevation: Dp = 0.dp,
    shadowElevation: Dp = 0.dp,
    border: BorderStroke? = null,
    content: @Composable () -> Unit,
) = Surface(
    modifier = modifier.clickable(enabled = enabled, onClick = onClick),
    shape = shape,
    color = color,
    contentColor = contentColor,
    tonalElevation = tonalElevation,
    shadowElevation = shadowElevation,
    border = border,
    content = content,
)

@Immutable
data class CardColors(
    val containerColor: Color,
    val contentColor: Color,
)

object CardDefaults {
    @Composable
    fun cardColors(
        containerColor: Color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.65f),
        contentColor: Color = MaterialTheme.colorScheme.onSurface,
    ): CardColors = CardColors(containerColor = containerColor, contentColor = contentColor)
}

@Composable
fun Card(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(12.dp),
    colors: CardColors = CardDefaults.cardColors(),
    border: BorderStroke? = null,
    content: @Composable ColumnScope.() -> Unit,
) {
    Surface(
        modifier = modifier.clip(shape),
        shape = shape,
        color = colors.containerColor,
        contentColor = colors.contentColor,
        border = border,
    ) {
        Column(content = content)
    }
}

@Composable
fun ElevatedCard(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(12.dp),
    colors: CardColors = CardDefaults.cardColors(),
    content: @Composable ColumnScope.() -> Unit,
) = Card(modifier = modifier, shape = shape, colors = colors, content = content)

@Composable
fun OutlinedCard(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(12.dp),
    colors: CardColors = CardDefaults.cardColors(),
    border: BorderStroke? = null,
    content: @Composable ColumnScope.() -> Unit,
) = Card(modifier = modifier, shape = shape, colors = colors, border = border, content = content)

@Composable
fun Scaffold(
    modifier: Modifier = Modifier,
    topBar: @Composable () -> Unit = {},
    bottomBar: @Composable () -> Unit = {},
    snackbarHost: @Composable () -> Unit = {},
    floatingActionButton: @Composable () -> Unit = {},
    containerColor: Color = MaterialTheme.colorScheme.background,
    contentColor: Color = MaterialTheme.colorScheme.onBackground,
    contentWindowInsets: WindowInsets = WindowInsets(),
    content: @Composable (PaddingValues) -> Unit,
) {
    Column(modifier = modifier.fillMaxSize()) {
        topBar()
        Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
            content(PaddingValues(0.dp))
        }
        snackbarHost()
        floatingActionButton()
        bottomBar()
    }
}

object TopAppBarDefaults {
    @Composable
    fun topAppBarColors(
        containerColor: Color = MaterialTheme.colorScheme.surface.copy(alpha = 0.9f),
        scrolledContainerColor: Color = containerColor,
        navigationIconContentColor: Color = MaterialTheme.colorScheme.onSurface,
        titleContentColor: Color = MaterialTheme.colorScheme.onSurface,
        actionIconContentColor: Color = MaterialTheme.colorScheme.onSurfaceVariant,
    ): Color = containerColor
}

@Composable
fun TopAppBar(
    title: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    navigationIcon: @Composable () -> Unit = {},
    actions: @Composable RowScope.() -> Unit = {},
    windowInsets: WindowInsets = WindowInsets(),
    colors: Color = MaterialTheme.colorScheme.surface.copy(alpha = 0.9f),
) {
    Surface(
        modifier = modifier
            .fillMaxWidth()
            .windowInsetsPadding(windowInsets)
            .heightIn(min = 56.dp)
            .frostedGlass(blurRadius = 18f),
        color = colors,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                navigationIcon()
                title()
            }
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                content = actions,
            )
        }
    }
}

@Composable
fun BottomAppBar(
    modifier: Modifier = Modifier,
    content: @Composable RowScope.() -> Unit,
) {
    Row(modifier = modifier.frostedGlass(blurRadius = 18f), content = content)
}

interface VisualTransformation {
    companion object {
        val None = object : VisualTransformation {}
    }
}

class PasswordVisualTransformation(val mask: Char = '\u2022') : VisualTransformation

enum class KeyboardType { Text, Ascii, Number, Phone, Uri, Email, Password, NumberPassword, Decimal }

data class KeyboardOptions(
    val keyboardType: KeyboardType = KeyboardType.Text,
) {
    companion object {
        val Default = KeyboardOptions()
    }
}

@Composable
fun OutlinedTextField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    readOnly: Boolean = false,
    textStyle: TextStyle = MaterialTheme.typography.bodyMedium,
    label: (@Composable () -> Unit)? = null,
    placeholder: (@Composable () -> Unit)? = null,
    leadingIcon: (@Composable () -> Unit)? = null,
    trailingIcon: (@Composable () -> Unit)? = null,
    visualTransformation: VisualTransformation = VisualTransformation.None,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    singleLine: Boolean = true,
    maxLines: Int = if (singleLine) 1 else Int.MAX_VALUE,
    shape: Shape = RoundedCornerShape(12.dp),
) {
    Column(modifier = modifier) {
        label?.invoke()
        Row(verticalAlignment = Alignment.CenterVertically) {
            leadingIcon?.invoke()
            Box(modifier = Modifier.weight(1f)) {
                if (value.isEmpty() && placeholder != null) {
                    placeholder()
                }
                TextField(
                    value = value,
                    placeholder = "",
                    onValueChange = onValueChange,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            trailingIcon?.invoke()
        }
    }
}

@Composable
fun OutlinedTextField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String,
    singleLine: Boolean = true,
) {
    TextField(
        value = value,
        placeholder = placeholder,
        onValueChange = onValueChange,
        modifier = modifier,
    )
}

@Composable
fun SearchBar(
    query: String,
    onQueryChange: (String) -> Unit,
    onSearch: (String) -> Unit = {},
    active: Boolean = false,
    onActiveChange: (Boolean) -> Unit = {},
    placeholder: String = "Search...",
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit = {},
) {
    Column(modifier = modifier) {
        TextField(value = query, placeholder = placeholder, onValueChange = onQueryChange)
        if (active) {
            content()
        }
    }
}

@Composable
fun LinearProgressIndicator(
    progress: () -> Float,
    modifier: Modifier = Modifier,
    color: Color = MaterialTheme.colorScheme.primary,
    trackColor: Color = MaterialTheme.colorScheme.surfaceVariant,
    strokeCap: StrokeCap = StrokeCap.Round,
) {
    ProgressBar(progress = progress(), modifier = modifier)
}

@Composable
fun LinearProgressIndicator(
    modifier: Modifier = Modifier,
    color: Color = MaterialTheme.colorScheme.primary,
    trackColor: Color = MaterialTheme.colorScheme.surfaceVariant,
    strokeCap: StrokeCap = StrokeCap.Round,
) {
    ProgressBar(progress = null, modifier = modifier)
}

@Composable
fun CircularProgressIndicator(
    progress: (() -> Float)? = null,
    modifier: Modifier = Modifier,
    color: Color = MaterialTheme.colorScheme.primary,
    strokeWidth: Dp = 4.dp,
    trackColor: Color = Color.Transparent,
    strokeCap: StrokeCap = StrokeCap.Round,
) {
    ProgressBar(progress = progress?.invoke(), modifier = modifier)
}

@Composable
fun Switch(
    checked: Boolean,
    onCheckedChange: ((Boolean) -> Unit)?,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    Button(
        label = if (checked) "[ON]" else "[OFF]",
        modifier = modifier,
        onClick = { if (enabled) onCheckedChange?.invoke(!checked) },
    )
}

@Composable
fun Checkbox(
    checked: Boolean,
    onCheckedChange: ((Boolean) -> Unit)?,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    Button(
        label = if (checked) "[x]" else "[ ]",
        modifier = modifier,
        onClick = { if (enabled) onCheckedChange?.invoke(!checked) },
    )
}

@Composable
fun RadioButton(
    selected: Boolean,
    onClick: (() -> Unit)?,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    Button(
        label = if (selected) "(•)" else "( )",
        modifier = modifier,
        onClick = { if (enabled) onClick?.invoke() },
    )
}

@Composable
fun FilterChip(
    selected: Boolean,
    onClick: () -> Unit,
    label: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    leadingIcon: (@Composable () -> Unit)? = null,
    trailingIcon: (@Composable () -> Unit)? = null,
) {
    Button(onClick = onClick, enabled = enabled, modifier = modifier) {
        leadingIcon?.invoke()
        label()
        trailingIcon?.invoke()
    }
}

@Composable
fun AssistChip(
    onClick: () -> Unit,
    label: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    leadingIcon: (@Composable () -> Unit)? = null,
    trailingIcon: (@Composable () -> Unit)? = null,
) {
    Button(onClick = onClick, enabled = enabled, modifier = modifier) {
        leadingIcon?.invoke()
        label()
        trailingIcon?.invoke()
    }
}

@Composable
fun SuggestionChip(
    onClick: () -> Unit,
    label: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    icon: (@Composable () -> Unit)? = null,
) {
    Button(onClick = onClick, enabled = enabled, modifier = modifier) {
        icon?.invoke()
        label()
    }
}

@Composable
fun Slider(
    value: Float,
    onValueChange: (Float) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    valueRange: ClosedFloatingPointRange<Float> = 0f..1f,
    steps: Int = 0,
    onValueChangeFinished: (() -> Unit)? = null,
) {
    val norm = ((value - valueRange.start) / (valueRange.endInclusive - valueRange.start).coerceAtLeast(1e-4f))
        .coerceIn(0f, 1f)
    ProgressBar(progress = norm, modifier = modifier)
}

@Composable
fun DropdownMenu(
    expanded: Boolean,
    onDismissRequest: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    if (expanded) {
        Column(modifier = modifier.frostedGlass(blurRadius = 24f), content = content)
    }
}

@Composable
fun DropdownMenuItem(
    text: @Composable () -> Unit,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    leadingIcon: (@Composable () -> Unit)? = null,
    trailingIcon: (@Composable () -> Unit)? = null,
    enabled: Boolean = true,
) {
    Button(onClick = onClick, enabled = enabled, modifier = modifier) {
        leadingIcon?.invoke()
        text()
        trailingIcon?.invoke()
    }
}

@Composable
fun AlertDialog(
    onDismissRequest: () -> Unit,
    confirmButton: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    dismissButton: (@Composable () -> Unit)? = null,
    icon: (@Composable () -> Unit)? = null,
    title: (@Composable () -> Unit)? = null,
    text: (@Composable () -> Unit)? = null,
) {
    Column(modifier = modifier.frostedGlass(blurRadius = 28f)) {
        icon?.invoke()
        title?.invoke()
        text?.invoke()
        Row {
            dismissButton?.invoke()
            confirmButton()
        }
    }
}

@Composable
fun Dialog(
    onDismissRequest: () -> Unit,
    content: @Composable () -> Unit,
) {
    Box(modifier = Modifier.frostedGlass(blurRadius = 28f)) {
        content()
    }
}

@Composable
fun Badge(
    modifier: Modifier = Modifier,
    containerColor: Color = MaterialTheme.colorScheme.error,
    contentColor: Color = MaterialTheme.colorScheme.onError,
    content: (@Composable RowScope.() -> Unit)? = null,
) {
    Surface(
        modifier = modifier.clip(CircleShape),
        color = containerColor,
        contentColor = contentColor,
    ) {
        if (content != null) {
            Row(modifier = Modifier.padding(horizontal = 4.dp), content = content)
        }
    }
}

@Composable
fun BadgedBox(
    badge: @Composable BoxScope.() -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable BoxScope.() -> Unit,
) {
    Box(modifier = modifier) {
        content()
        badge()
    }
}

// ============================================================================
// Lazy Lists, Lazy Grids, Icons, Canvas & Animation Helpers
// ============================================================================

@Stable
class LazyListState(
    val firstVisibleItemIndex: Int = 0,
    val firstVisibleItemScrollOffset: Int = 0,
) {
    suspend fun scrollToItem(index: Int, scrollOffset: Int = 0) {}
    suspend fun animateScrollToItem(index: Int, scrollOffset: Int = 0) {}
}

@Composable
fun rememberLazyListState(
    initialFirstVisibleItemIndex: Int = 0,
    initialFirstVisibleItemScrollOffset: Int = 0,
): LazyListState = remember {
    LazyListState(initialFirstVisibleItemIndex, initialFirstVisibleItemScrollOffset)
}

class LazyListScope {
    internal val blocks = ArrayList<@Composable () -> Unit>()

    fun item(key: Any? = null, contentType: Any? = null, content: @Composable () -> Unit) {
        blocks.add(content)
    }

    fun items(
        count: Int,
        key: ((index: Int) -> Any)? = null,
        itemContent: @Composable (index: Int) -> Unit,
    ) {
        val cap = count.coerceAtMost(50)
        for (i in 0 until cap) {
            val idx = i
            blocks.add { itemContent(idx) }
        }
    }
}

fun <T> LazyListScope.items(
    items: List<T>,
    key: ((item: T) -> Any)? = null,
    itemContent: @Composable (item: T) -> Unit,
) {
    val cap = items.size.coerceAtMost(50)
    for (i in 0 until cap) {
        val item = items[i]
        blocks.add { itemContent(item) }
    }
}

@Composable
fun LazyColumn(
    modifier: Modifier = Modifier,
    state: LazyListState = rememberLazyListState(),
    contentPadding: PaddingValues = PaddingValues(0.dp),
    reverseLayout: Boolean = false,
    verticalArrangement: Arrangement.Vertical = Arrangement.Top,
    horizontalAlignment: Alignment.Horizontal = Alignment.Start,
    content: LazyListScope.() -> Unit,
) {
    val scope = LazyListScope().apply(content)
    Column(
        modifier = modifier.padding(contentPadding),
        verticalArrangement = verticalArrangement,
        horizontalAlignment = horizontalAlignment,
    ) {
        for (block in scope.blocks) {
            block()
        }
    }
}

@Composable
fun LazyRow(
    modifier: Modifier = Modifier,
    state: LazyListState = rememberLazyListState(),
    contentPadding: PaddingValues = PaddingValues(0.dp),
    reverseLayout: Boolean = false,
    horizontalArrangement: Arrangement.Horizontal = Arrangement.Start,
    verticalAlignment: Alignment.Vertical = Alignment.Top,
    content: LazyListScope.() -> Unit,
) {
    val scope = LazyListScope().apply(content)
    Row(
        modifier = modifier.padding(contentPadding),
        horizontalArrangement = horizontalArrangement,
        verticalAlignment = verticalAlignment,
    ) {
        for (block in scope.blocks) {
            block()
        }
    }
}

@Composable
fun <T> LazyRow(
    items: List<T>,
    modifier: Modifier = Modifier,
    itemContent: @Composable (index: Int, item: T) -> Unit,
) {
    LazyColumn(items = items, modifier = modifier, itemContent = itemContent)
}

sealed class GridCells {
    data class Fixed(val count: Int) : GridCells()
    data class Adaptive(val minSize: Dp) : GridCells()
}

@Stable
class LazyGridState(
    val firstVisibleItemIndex: Int = 0,
    val firstVisibleItemScrollOffset: Int = 0,
)

@Composable
fun rememberLazyGridState(
    initialFirstVisibleItemIndex: Int = 0,
    initialFirstVisibleItemScrollOffset: Int = 0,
): LazyGridState = remember {
    LazyGridState(initialFirstVisibleItemIndex, initialFirstVisibleItemScrollOffset)
}

typealias LazyGridScope = LazyListScope

@Composable
fun LazyVerticalGrid(
    columns: GridCells,
    modifier: Modifier = Modifier,
    state: LazyGridState = rememberLazyGridState(),
    contentPadding: PaddingValues = PaddingValues(0.dp),
    verticalArrangement: Arrangement.Vertical = Arrangement.Top,
    horizontalArrangement: Arrangement.Horizontal = Arrangement.Start,
    content: LazyGridScope.() -> Unit,
) {
    val scope = LazyGridScope().apply(content)
    Column(modifier = modifier.padding(contentPadding), verticalArrangement = verticalArrangement) {
        for (block in scope.blocks) {
            block()
        }
    }
}

@Composable
fun <T> LazyVerticalGrid(
    columns: Int = 2,
    items: List<T>,
    modifier: Modifier = Modifier,
    itemContent: @Composable (index: Int, item: T) -> Unit,
) {
    LazyColumn(items = items, modifier = modifier, itemContent = itemContent)
}

@Composable
fun <T> TreeView(
    nodes: List<T>,
    modifier: Modifier = Modifier,
    nodeContent: @Composable (index: Int, node: T) -> Unit,
) {
    LazyColumn(items = nodes, modifier = modifier, itemContent = nodeContent)
}

@Composable
fun <T> DataTable(
    headers: List<String>,
    rows: List<T>,
    modifier: Modifier = Modifier,
    rowContent: @Composable (index: Int, row: T) -> Unit,
) {
    Column(modifier = modifier) {
        Row {
            for (h in headers) {
                Label(h)
            }
        }
        LazyColumn(items = rows, itemContent = rowContent)
    }
}

@Composable
fun Spacer(modifier: Modifier = Modifier) {
    val session = LocalIguanodonSession.current
    val nodeId = remember { session?.allocateNodeId() ?: 0L }
    ComposeNode<IguanodonNode, IguanodonApplier>(
        factory = { IguanodonNode(nodeId = nodeId, nodeKind = NodeKinds.SPACER) },
        update = {
            set(modifier) { mod ->
                session?.syncModifierToNode(this, mod)
            }
        },
    )
}

@Composable
fun HorizontalDivider(
    modifier: Modifier = Modifier,
    thickness: Dp = 1.dp,
    color: Color = MaterialTheme.colorScheme.outlineVariant,
) {
    Box(modifier = modifier.fillMaxWidth().height(thickness).background(color))
}

@Composable
fun VerticalDivider(
    modifier: Modifier = Modifier,
    thickness: Dp = 1.dp,
    color: Color = MaterialTheme.colorScheme.outlineVariant,
) {
    Box(modifier = modifier.fillMaxHeight().width(thickness).background(color))
}

@Immutable
data class ImageVector(val name: String, val glyph: String = "•")

object Icons {
    object Filled {
        val AttachFile = ImageVector("AttachFile", "📎")
        val Check = ImageVector("Check", "✓")
        val Close = ImageVector("Close", "✕")
        val ContentCopy = ImageVector("ContentCopy", "⧉")
        val Delete = ImageVector("Delete", "🗑")
        val Info = ImageVector("Info", "ℹ")
        val Lock = ImageVector("Lock", "🔒")
        val MoreVert = ImageVector("MoreVert", "⋮")
        val Notifications = ImageVector("Notifications", "🔔")
        val Phone = ImageVector("Phone", "📞")
        val Refresh = ImageVector("Refresh", "↻")
        val Search = ImageVector("Search", "🔍")
        val Settings = ImageVector("Settings", "⚙")
        val Star = ImageVector("Star", "★")
        val Visibility = ImageVector("Visibility", "👁")
        val VisibilityOff = ImageVector("VisibilityOff", "🚫")
    }

    object AutoMirrored {
        object Filled {
            val ArrowBack = ImageVector("ArrowBack", "←")
            val Send = ImageVector("Send", "➤")
        }
    }

    val Default = Filled
}

@Composable
fun Icon(
    imageVector: ImageVector,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    tint: Color = LocalContentColor.current,
) {
    Text(text = imageVector.glyph, modifier = modifier, color = tint)
}

class ImageBitmap(
    val width: Int,
    val height: Int,
    val rgbaBytes: ByteArray = ByteArray(0),
)

@Composable
fun Image(
    bitmap: ImageBitmap,
    contentDescription: String?,
    modifier: Modifier = Modifier,
) {
    val session = LocalIguanodonSession.current
    val nodeId = remember { session?.allocateNodeId() ?: 0L }
    ComposeNode<IguanodonNode, IguanodonApplier>(
        factory = { IguanodonNode(nodeId = nodeId, nodeKind = NodeKinds.IMAGE) },
        update = {
            set(modifier) { mod ->
                session?.syncModifierToNode(this, mod)
            }
            set(bitmap) { bmp ->
                if (bmp.rgbaBytes.isNotEmpty()) {
                    session?.encoder?.encodeSetImageBytes(
                        nodeId = this.nodeId,
                        width = bmp.width,
                        height = bmp.height,
                        rgbaBytes = bmp.rgbaBytes,
                    )
                }
            }
        },
    )
}

enum class StrokeCap { Butt, Round, Square }

data class Stroke(
    val width: Float = 1f,
    val cap: StrokeCap = StrokeCap.Butt,
)

class Path {
    fun moveTo(x: Float, y: Float) {}
    fun lineTo(x: Float, y: Float) {}
    fun quadraticTo(x1: Float, y1: Float, x2: Float, y2: Float) {}
    fun close() {}
}

class DrawScope(val size: Size = Size(24f, 24f)) {
    fun drawCircle(color: Color, radius: Float = size.width * 0.5f, center: Offset = Offset.Zero, style: Stroke? = null) {}
    fun drawRoundRect(color: Color, topLeft: Offset = Offset.Zero, size: Size = this.size, cornerRadius: CornerRadius = CornerRadius.Zero, style: Stroke? = null) {}
    fun drawPath(path: Path, color: Color, style: Stroke? = null) {}
    fun drawLine(color: Color, start: Offset, end: Offset, strokeWidth: Float = 1f, cap: StrokeCap = StrokeCap.Butt) {}
}

@Composable
fun Canvas(
    modifier: Modifier,
    onDraw: DrawScope.() -> Unit,
) {
    Box(modifier = modifier)
}

object Spring {
    const val DampingRatioHighBouncy = 0.2f
    const val DampingRatioMediumBouncy = 0.5f
    const val DampingRatioLowBouncy = 0.75f
    const val DampingRatioNoBouncy = 1f
    const val StiffnessHigh = 10_000f
    const val StiffnessMedium = 1500f
    const val StiffnessMediumLow = 400f
    const val StiffnessLow = 200f
}

class SpringSpec<T>(
    val dampingRatio: Float = Spring.DampingRatioNoBouncy,
    val stiffness: Float = Spring.StiffnessMedium,
    val visibilityThreshold: T? = null,
)

fun <T> spring(
    dampingRatio: Float = Spring.DampingRatioNoBouncy,
    stiffness: Float = Spring.StiffnessMedium,
    visibilityThreshold: T? = null,
): SpringSpec<T> = SpringSpec(dampingRatio, stiffness, visibilityThreshold)

class EnterTransition
class ExitTransition

fun fadeIn(): EnterTransition = EnterTransition()
fun fadeOut(): ExitTransition = ExitTransition()

@Composable
fun AnimatedVisibility(
    visible: Boolean,
    modifier: Modifier = Modifier,
    enter: EnterTransition = fadeIn(),
    exit: ExitTransition = fadeOut(),
    content: @Composable () -> Unit,
) {
    if (visible) {
        Box(modifier = modifier) {
            content()
        }
    }
}

@Composable
fun SelectionContainer(
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Box(modifier = modifier) { content() }
}

@Composable
fun FlowRow(
    modifier: Modifier = Modifier,
    horizontalArrangement: Arrangement.Horizontal = Arrangement.Start,
    verticalArrangement: Arrangement.Vertical = Arrangement.Top,
    content: @Composable RowScope.() -> Unit,
) {
    Row(
        modifier = modifier.horizontalScroll(rememberScrollState()),
        horizontalArrangement = horizontalArrangement,
        verticalAlignment = Alignment.CenterVertically,
        content = content,
    )
}

// ============================================================================
// 4. Qt 6 (`QWidgets` / `QML`) Direct Porting Primitives
// ============================================================================

@Composable
fun QMainWindow(
    windowTitle: String = "",
    menuBar: @Composable () -> Unit = {},
    toolBar: @Composable () -> Unit = {},
    statusBar: @Composable () -> Unit = {},
    dockWidgets: @Composable () -> Unit = {},
    centralWidget: @Composable () -> Unit,
) {
    Scaffold(
        topBar = {
            Column {
                if (windowTitle.isNotEmpty()) Label(windowTitle)
                menuBar()
                toolBar()
            }
        },
        bottomBar = statusBar,
    ) {
        Row {
            dockWidgets()
            Box(modifier = Modifier.weight(1f)) {
                centralWidget()
            }
        }
    }
}

@Composable
fun QSplitter(
    horizontal: Boolean = true,
    splitRatio: Float = 0.5f,
    onSplitRatioChange: (Float) -> Unit = {},
    first: @Composable () -> Unit,
    second: @Composable () -> Unit,
) {
    if (horizontal) {
        HorizontalSplitPane(
            splitRatio = splitRatio,
            onSplitRatioChange = onSplitRatioChange,
            first = first,
            second = second,
        )
    } else {
        VerticalSplitPane(
            splitRatio = splitRatio,
            onSplitRatioChange = onSplitRatioChange,
            top = first,
            bottom = second,
        )
    }
}

@Composable
fun QDockWidget(
    title: String,
    floating: Boolean = false,
    content: @Composable () -> Unit,
) = DraggablePanel(title = title, initialDocked = !floating, content = content)

@Composable
fun QTabWidget(
    currentIndex: Int,
    tabTitles: List<String>,
    onTabSelected: (Int) -> Unit,
    pageContent: @Composable (Int) -> Unit,
) {
    Column {
        TabRow(selectedTabIndex = currentIndex) {
            tabTitles.forEachIndexed { idx, title ->
                Tab(
                    selected = idx == currentIndex,
                    onClick = { onTabSelected(idx) },
                    text = { Text(title) },
                )
            }
        }
        pageContent(currentIndex)
    }
}

@Composable
fun QGroupBox(title: String, content: @Composable () -> Unit) {
    Card {
        Column {
            Label(title)
            content()
        }
    }
}

@Composable
fun QPushButton(text: String, onClicked: () -> Unit) = Button(label = text, onClick = onClicked)

@Composable
fun QCheckBox(text: String, checked: Boolean, onToggled: (Boolean) -> Unit) {
    Row {
        Checkbox(checked = checked, onCheckedChange = onToggled)
        Label(text)
    }
}

@Composable
fun QLineEdit(text: String, placeholderText: String = "", onTextChanged: (String) -> Unit) =
    TextField(value = text, placeholder = placeholderText, onValueChange = onTextChanged)

@Composable
fun QSpinBox(
    value: Int,
    range: IntRange = 0..100,
    onValueChanged: (Int) -> Unit,
) {
    Row {
        Button(label = "-", onClick = { onValueChanged((value - 1).coerceIn(range)) })
        Label("$value")
        Button(label = "+", onClick = { onValueChanged((value + 1).coerceIn(range)) })
    }
}

@Composable
fun QComboBox(
    items: List<String>,
    currentIndex: Int,
    onCurrentIndexChanged: (Int) -> Unit,
) {
    val currentLabel = items.getOrElse(currentIndex) { "" }
    Button(
        label = "$currentLabel ▾",
        onClick = {
            if (items.isNotEmpty()) {
                onCurrentIndexChanged((currentIndex + 1) % items.size)
            }
        },
    )
}

@Composable
fun QProgressBar(value: Int, minimum: Int = 0, maximum: Int = 100) {
    val norm = ((value - minimum).toFloat() / (maximum - minimum).coerceAtLeast(1).toFloat())
        .coerceIn(0f, 1f)
    ProgressBar(progress = norm)
}

// ============================================================================
// 5. GTK 4 & Libadwaita (`Gtk` / `Adw`) Direct Porting Primitives
// ============================================================================

@Composable
fun AdwApplicationWindow(
    title: String = "",
    headerBar: @Composable () -> Unit = { AdwHeaderBar(title = title) },
    content: @Composable () -> Unit,
) {
    Scaffold(topBar = headerBar) { content() }
}

@Composable
fun AdwHeaderBar(
    title: String,
    startWidgets: @Composable () -> Unit = {},
    endWidgets: @Composable RowScope.() -> Unit = {},
) {
    TopAppBar(
        title = { Label(title) },
        navigationIcon = startWidgets,
        actions = endWidgets,
    )
}

@Composable
fun AdwNavigationSplitView(
    sidebar: @Composable () -> Unit,
    content: @Composable () -> Unit,
    splitRatio: Float = 0.28f,
    onSplitRatioChange: (Float) -> Unit = {},
) {
    val session = LocalIguanodonSession.current
    if (session?.isPortrait == true) {
        Column {
            sidebar()
            content()
        }
    } else {
        HorizontalSplitPane(
            splitRatio = splitRatio,
            onSplitRatioChange = onSplitRatioChange,
            first = sidebar,
            second = content,
        )
    }
}

@Composable
fun GtkPaned(
    horizontal: Boolean = true,
    splitRatio: Float = 0.5f,
    onSplitRatioChange: (Float) -> Unit = {},
    startChild: @Composable () -> Unit,
    endChild: @Composable () -> Unit,
) = QSplitter(
    horizontal = horizontal,
    splitRatio = splitRatio,
    onSplitRatioChange = onSplitRatioChange,
    first = startChild,
    second = endChild,
)

@Composable
fun AdwFlap(
    flap: @Composable () -> Unit,
    content: @Composable () -> Unit,
    revealFlap: Boolean = true,
) {
    Row {
        if (revealFlap) {
            DraggablePanel(title = "Flap") { flap() }
        }
        Box(modifier = Modifier.weight(1f)) { content() }
    }
}

@Composable
fun AdwPreferencesPage(content: @Composable ColumnScope.() -> Unit) = Column(content = content)

@Composable
fun AdwPreferencesGroup(
    title: String,
    description: String = "",
    content: @Composable ColumnScope.() -> Unit,
) {
    Card {
        Column {
            Label(title)
            if (description.isNotEmpty()) Label(description)
            content()
        }
    }
}

@Composable
fun AdwActionRow(
    title: String,
    subtitle: String = "",
    suffix: @Composable () -> Unit = {},
) {
    Row {
        Column(modifier = Modifier.weight(1f)) {
            Label(title)
            if (subtitle.isNotEmpty()) Label(subtitle)
        }
        suffix()
    }
}

@Composable
fun AdwSwitchRow(
    title: String,
    subtitle: String = "",
    active: Boolean,
    onActiveChanged: (Boolean) -> Unit,
) {
    AdwActionRow(title = title, subtitle = subtitle) {
        Switch(checked = active, onCheckedChange = onActiveChanged)
    }
}

@Composable
fun AdwEntryRow(
    title: String,
    text: String,
    onTextChanged: (String) -> Unit,
) {
    AdwActionRow(title = title) {
        TextField(value = text, placeholder = title, onValueChange = onTextChanged)
    }
}

@Composable
fun AdwComboRow(
    title: String,
    model: List<String>,
    selected: Int,
    onSelectedChanged: (Int) -> Unit,
) {
    AdwActionRow(title = title) {
        QComboBox(items = model, currentIndex = selected, onCurrentIndexChanged = onSelectedChanged)
    }
}
