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
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.State
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import iguanodon.compose.bridge.IguanodonNativeBridge
import iguanodon.compose.bridge.IguanodonSession
import kotlin.math.roundToInt

/**
 * CompositionLocals providing the active [IguanodonSession], theme family, theme polarity,
 * and configurable font scale to all `@Composable` widgets on both Android and Desktop.
 */
val LocalIguanodonSession = staticCompositionLocalOf<IguanodonSession?> { null }
val LocalIguanodonThemeId = staticCompositionLocalOf { IguanodonThemeId.Material3Expressive }
val LocalIguanodonThemeMode = staticCompositionLocalOf { IguanodonThemeMode.Dark }
val LocalIguanodonFontScale = staticCompositionLocalOf { 1.0f }

/**
 * Minimum interactive touch target size in density-independent pixels (48dp),
 * matching Android Material 3 accessibility guidelines and `MIN_TOUCH_TARGET_DP` in `iguanodon-layout`.
 */
const val MIN_TOUCH_TARGET_DP: Float = 48.0f

// ============================================================================
// Pure `compose.runtime` Value Types (`Color`, `Dp`, `TextUnit`, `Density`, `FontWeight`, `Shape`)
// ============================================================================

@Immutable
data class Color(val value: Long) {
    constructor(argb: Int) : this(argb.toLong() and 0xFFFFFFFFL)
    constructor(
        red: Float,
        green: Float,
        blue: Float,
        alpha: Float = 1f,
    ) : this(
        ((alpha.coerceIn(0f, 1f) * 255f).roundToInt() shl 24) or
            ((red.coerceIn(0f, 1f) * 255f).roundToInt() shl 16) or
            ((green.coerceIn(0f, 1f) * 255f).roundToInt() shl 8) or
            (blue.coerceIn(0f, 1f) * 255f).roundToInt(),
    )
    constructor(
        red: Int,
        green: Int,
        blue: Int,
        alpha: Int = 255,
    ) : this(
        ((alpha.coerceIn(0, 255)) shl 24) or
            ((red.coerceIn(0, 255)) shl 16) or
            ((green.coerceIn(0, 255)) shl 8) or
            (blue.coerceIn(0, 255)),
    )

    val alpha: Float get() = ((value ushr 24) and 0xFFL).toFloat() / 255f
    val red: Float get() = ((value ushr 16) and 0xFFL).toFloat() / 255f
    val green: Float get() = ((value ushr 8) and 0xFFL).toFloat() / 255f
    val blue: Float get() = (value and 0xFFL).toFloat() / 255f

    fun copy(
        alpha: Float = this.alpha,
        red: Float = this.red,
        green: Float = this.green,
        blue: Float = this.blue,
    ): Color = Color(red = red, green = green, blue = blue, alpha = alpha)

    fun toArgb(): Int = (value and 0xFFFFFFFFL).toInt()

    companion object {
        val Transparent = Color(0x00000000)
        val Black = Color(0xFF000000)
        val DarkGray = Color(0xFF444444)
        val Gray = Color(0xFF888888)
        val LightGray = Color(0xFFCCCCCC)
        val White = Color(0xFFFFFFFF)
        val Red = Color(0xFFFF0000)
        val Green = Color(0xFF00FF00)
        val Blue = Color(0xFF0000FF)
        val Yellow = Color(0xFFFFFF00)
        val Cyan = Color(0xFF00FFFF)
        val Magenta = Color(0xFFFF00FF)
        val Unspecified = Color(0x00000000)
    }
}

@Immutable
data class Dp(val value: Float) : Comparable<Dp> {
    override fun compareTo(other: Dp): Int = value.compareTo(other.value)
    operator fun plus(other: Dp): Dp = Dp(value + other.value)
    operator fun minus(other: Dp): Dp = Dp(value - other.value)
    operator fun times(other: Float): Dp = Dp(value * other)
    operator fun times(other: Int): Dp = Dp(value * other)
    operator fun div(other: Float): Dp = Dp(value / other)
    operator fun div(other: Dp): Float = value / other.value
    operator fun unaryMinus(): Dp = Dp(-value)
    fun coerceAtLeast(minimumValue: Dp): Dp = Dp(value.coerceAtLeast(minimumValue.value))
    fun coerceAtMost(maximumValue: Dp): Dp = Dp(value.coerceAtMost(maximumValue.value))
    fun coerceIn(minimumValue: Dp, maximumValue: Dp): Dp =
        Dp(value.coerceIn(minimumValue.value, maximumValue.value))

    companion object {
        val Hairline = Dp(0f)
        val Infinity = Dp(Float.POSITIVE_INFINITY)
        val Unspecified = Dp(Float.NaN)
    }
}

val Int.dp: Dp get() = Dp(this.toFloat())
val Double.dp: Dp get() = Dp(this.toFloat())
val Float.dp: Dp get() = Dp(this)

@Immutable
data class TextUnit(val value: Float, val isEm: Boolean = false) {
    operator fun times(scale: Float): TextUnit = TextUnit(value * scale, isEm)
    operator fun div(scale: Float): TextUnit = TextUnit(value / scale, isEm)

    companion object {
        val Unspecified = TextUnit(Float.NaN)
    }
}

val Int.sp: TextUnit get() = TextUnit(this.toFloat(), isEm = false)
val Double.sp: TextUnit get() = TextUnit(this.toFloat(), isEm = false)
val Float.sp: TextUnit get() = TextUnit(this, isEm = false)
val Int.em: TextUnit get() = TextUnit(this.toFloat(), isEm = true)
val Double.em: TextUnit get() = TextUnit(this.toFloat(), isEm = true)
val Float.em: TextUnit get() = TextUnit(this, isEm = true)

@Immutable
data class IntOffset(val x: Int, val y: Int) {
    companion object {
        val Zero = IntOffset(0, 0)
    }
}

@Immutable
data class Offset(val x: Float, val y: Float) {
    operator fun plus(other: Offset): Offset = Offset(x + other.x, y + other.y)
    operator fun minus(other: Offset): Offset = Offset(x - other.x, y - other.y)

    companion object {
        val Zero = Offset(0f, 0f)
        val Infinite = Offset(Float.POSITIVE_INFINITY, Float.POSITIVE_INFINITY)
        val Unspecified = Offset(Float.NaN, Float.NaN)
    }
}

@Immutable
data class Size(val width: Float, val height: Float) {
    companion object {
        val Zero = Size(0f, 0f)
        val Unspecified = Size(Float.NaN, Float.NaN)
    }
}

@Immutable
data class CornerRadius(val x: Float, val y: Float = x) {
    companion object {
        val Zero = CornerRadius(0f, 0f)
    }
}

@Immutable
data class Density(
    val density: Float = 1f,
    val fontScale: Float = 1f,
) {
    fun Dp.toPx(): Float = value * density
    fun Dp.roundToPx(): Int = (value * density).roundToInt()
    fun Float.toDp(): Dp = Dp(this / density.coerceAtLeast(1e-4f))
    fun Int.toDp(): Dp = Dp(this.toFloat() / density.coerceAtLeast(1e-4f))
    fun TextUnit.toPx(): Float = value * density * fontScale
}

val LocalDensity = staticCompositionLocalOf { Density(1f, 1f) }
val LocalContentColor = compositionLocalOf { Color(0xFFE6E0E9) }

@Immutable
data class FontWeight(val weight: Int) : Comparable<FontWeight> {
    override fun compareTo(other: FontWeight): Int = weight.compareTo(other.weight)

    companion object {
        val W100 = FontWeight(100)
        val W200 = FontWeight(200)
        val W300 = FontWeight(300)
        val W400 = FontWeight(400)
        val W500 = FontWeight(500)
        val W600 = FontWeight(600)
        val W700 = FontWeight(700)
        val W800 = FontWeight(800)
        val W900 = FontWeight(900)
        val Thin = W100
        val ExtraLight = W200
        val Light = W300
        val Normal = W400
        val Medium = W500
        val SemiBold = W600
        val Bold = W700
        val ExtraBold = W800
        val Black = W900
    }
}

sealed class FontFamily(val name: String) {
    data object Default : FontFamily("Default")
    data object SansSerif : FontFamily("SansSerif")
    data object Serif : FontFamily("Serif")
    data object Monospace : FontFamily("Monospace")
    data object Cursive : FontFamily("Cursive")
}

fun FontFamily(vararg fonts: Any): FontFamily = FontFamily.Default

enum class FontStyle { Normal, Italic }
enum class TextAlign { Start, End, Left, Right, Center, Justify }
enum class TextOverflow { Clip, Ellipsis, Visible }

@Immutable
data class SpanStyle(
    val color: Color = Color.Unspecified,
    val fontSize: TextUnit = TextUnit.Unspecified,
    val fontWeight: FontWeight? = null,
    val fontFamily: FontFamily? = null,
    val fontStyle: FontStyle? = null,
    val background: Color = Color.Unspecified,
)

@Immutable
data class AnnotatedString(val text: String) : CharSequence by text {
    override fun toString(): String = text
}

class AnnotatedStringBuilder {
    private val sb = StringBuilder()
    fun append(text: String) {
        sb.append(text)
    }
    fun append(ch: Char) {
        sb.append(ch)
    }
    fun pushStyle(style: SpanStyle): Int = 0
    fun pop() {}
    fun toAnnotatedString(): AnnotatedString = AnnotatedString(sb.toString())
}

inline fun buildAnnotatedString(builder: AnnotatedStringBuilder.() -> Unit): AnnotatedString =
    AnnotatedStringBuilder().apply(builder).toAnnotatedString()

@Immutable
data class TextStyle(
    val color: Color = Color.Unspecified,
    val fontSize: TextUnit = 14.sp,
    val fontWeight: FontWeight = FontWeight.Normal,
    val fontFamily: FontFamily = FontFamily.Default,
    val fontStyle: FontStyle = FontStyle.Normal,
    val textAlign: TextAlign = TextAlign.Start,
    val lineHeight: TextUnit = TextUnit.Unspecified,
)

@Immutable
data class Typography(
    val displayLarge: TextStyle = TextStyle(fontSize = 36.sp),
    val displayMedium: TextStyle = TextStyle(fontSize = 30.sp),
    val displaySmall: TextStyle = TextStyle(fontSize = 26.sp),
    val headlineLarge: TextStyle = TextStyle(fontSize = 24.sp),
    val headlineMedium: TextStyle = TextStyle(fontSize = 20.sp),
    val headlineSmall: TextStyle = TextStyle(fontSize = 18.sp),
    val titleLarge: TextStyle = TextStyle(fontSize = 18.sp),
    val titleMedium: TextStyle = TextStyle(fontSize = 16.sp),
    val titleSmall: TextStyle = TextStyle(fontSize = 14.sp),
    val bodyLarge: TextStyle = TextStyle(fontSize = 16.sp),
    val bodyMedium: TextStyle = TextStyle(fontSize = 14.sp),
    val bodySmall: TextStyle = TextStyle(fontSize = 12.sp),
    val labelLarge: TextStyle = TextStyle(fontSize = 14.sp),
    val labelMedium: TextStyle = TextStyle(fontSize = 12.sp),
    val labelSmall: TextStyle = TextStyle(fontSize = 11.sp),
)

interface Shape {
    val cornerRadiusDp: Float get() = 0f
}

@Immutable
data class RoundedCornerShape(override val cornerRadiusDp: Float = 12f) : Shape {
    constructor(radius: Dp) : this(radius.value)
    constructor(percent: Int) : this(percent.toFloat() * 0.5f)
}

val RectangleShape: Shape = RoundedCornerShape(0f)
val CircleShape: Shape = RoundedCornerShape(999f)

@Immutable
data class Shapes(
    val extraSmall: Shape = RoundedCornerShape(4.dp),
    val small: Shape = RoundedCornerShape(8.dp),
    val medium: Shape = RoundedCornerShape(12.dp),
    val large: Shape = RoundedCornerShape(16.dp),
    val extraLarge: Shape = RoundedCornerShape(28.dp),
)

@Immutable
data class BorderStroke(val width: Dp, val color: Color)

@Immutable
data class PaddingValues(
    val start: Dp = 0.dp,
    val top: Dp = 0.dp,
    val end: Dp = 0.dp,
    val bottom: Dp = 0.dp,
) {
    constructor(all: Dp) : this(all, all, all, all)
    constructor(horizontal: Dp = 0.dp, vertical: Dp = 0.dp) : this(horizontal, vertical, horizontal, vertical)
}

// ============================================================================
// Alignment, Arrangement, ScrollState, PointerInput & Interaction Primitives
// ============================================================================

interface Alignment {
    val code: Byte

    interface Horizontal {
        val code: Byte
    }

    interface Vertical {
        val code: Byte
    }

    companion object {
        val TopStart = object : Alignment { override val code: Byte = 0 }
        val TopCenter = object : Alignment { override val code: Byte = 1 }
        val TopEnd = object : Alignment { override val code: Byte = 2 }
        val CenterStart = object : Alignment { override val code: Byte = 3 }
        val Center = object : Alignment { override val code: Byte = 4 }
        val CenterEnd = object : Alignment { override val code: Byte = 5 }
        val BottomStart = object : Alignment { override val code: Byte = 6 }
        val BottomCenter = object : Alignment { override val code: Byte = 7 }
        val BottomEnd = object : Alignment { override val code: Byte = 8 }

        val Top = object : Vertical { override val code: Byte = 0 }
        val CenterVertically = object : Vertical { override val code: Byte = 1 }
        val Bottom = object : Vertical { override val code: Byte = 2 }

        val Start = object : Horizontal { override val code: Byte = 0 }
        val CenterHorizontally = object : Horizontal { override val code: Byte = 1 }
        val End = object : Horizontal { override val code: Byte = 2 }
    }
}

object Arrangement {
    interface Horizontal {
        val code: Byte
        val spacingDp: Float get() = 0f
    }

    interface Vertical {
        val code: Byte
        val spacingDp: Float get() = 0f
    }

    interface HorizontalOrVertical : Horizontal, Vertical {
        override val spacingDp: Float get() = 0f
    }

    val Start = object : Horizontal { override val code: Byte = 0 }
    val End = object : Horizontal { override val code: Byte = 2 }
    val Top = object : Vertical { override val code: Byte = 0 }
    val Bottom = object : Vertical { override val code: Byte = 2 }
    val Center = object : HorizontalOrVertical { override val code: Byte = 1 }
    val SpaceBetween = object : HorizontalOrVertical { override val code: Byte = 3 }
    val SpaceEvenly = object : HorizontalOrVertical { override val code: Byte = 4 }
    val SpaceAround = object : HorizontalOrVertical { override val code: Byte = 5 }

    fun spacedBy(space: Dp): HorizontalOrVertical = object : HorizontalOrVertical {
        override val code: Byte = 0
        override val spacingDp: Float = space.value
    }

    fun spacedBy(space: Dp, alignment: Alignment.Horizontal): Horizontal = spacedBy(space)
    fun spacedBy(space: Dp, alignment: Alignment.Vertical): Vertical = spacedBy(space)
}

@Stable
class ScrollState(initial: Int = 0) {
    var value: Int by mutableStateOf(initial)
        internal set
    var maxValue: Int by mutableStateOf(Int.MAX_VALUE)
        internal set

    fun dispatchRawDelta(delta: Float): Float {
        value = (value + delta.roundToInt()).coerceIn(0, maxValue)
        return delta
    }

    suspend fun scrollTo(targetValue: Int): Float {
        val d = (targetValue - value).toFloat()
        value = targetValue.coerceIn(0, maxValue)
        return d
    }

    suspend fun animateScrollTo(targetValue: Int) {
        scrollTo(targetValue)
    }
}

@Composable
fun rememberScrollState(initial: Int = 0): ScrollState = remember { ScrollState(initial) }

class MutableInteractionSource {
    internal val hoveredState = mutableStateOf(false)
}

@Composable
fun MutableInteractionSource.collectIsHoveredAsState(): State<Boolean> = hoveredState

class PointerInputChange(
    val position: Offset = Offset.Zero,
    var consumed: Boolean = false,
) {
    fun consume() {
        consumed = true
    }
}

class PointerInputScope {
    internal var onDragDeltaCallback: ((Float, Float) -> Unit)? = null
    internal var onTapCallback: ((Offset) -> Unit)? = null
    internal var onLongPressCallback: ((Offset) -> Unit)? = null
}

fun PointerInputScope.detectDragGestures(
    onDragStart: (Offset) -> Unit = {},
    onDragEnd: () -> Unit = {},
    onDragCancel: () -> Unit = {},
    onDrag: (change: PointerInputChange, dragAmount: Offset) -> Unit,
) {
    onDragDeltaCallback = { dx, dy ->
        onDrag(PointerInputChange(), Offset(dx, dy))
    }
}

fun PointerInputScope.detectHorizontalDragGestures(
    onDragStart: (Offset) -> Unit = {},
    onDragEnd: () -> Unit = {},
    onDragCancel: () -> Unit = {},
    onHorizontalDrag: (change: PointerInputChange, dragAmount: Float) -> Unit,
) {
    onDragDeltaCallback = { dx, _ ->
        onHorizontalDrag(PointerInputChange(), dx)
    }
}

fun PointerInputScope.detectVerticalDragGestures(
    onDragStart: (Offset) -> Unit = {},
    onDragEnd: () -> Unit = {},
    onDragCancel: () -> Unit = {},
    onVerticalDrag: (change: PointerInputChange, dragAmount: Float) -> Unit,
) {
    onDragDeltaCallback = { _, dy ->
        onVerticalDrag(PointerInputChange(), dy)
    }
}

fun PointerInputScope.detectTapGestures(
    onDoubleTap: ((Offset) -> Unit)? = null,
    onLongPress: ((Offset) -> Unit)? = null,
    onPress: suspend (Offset) -> Unit = {},
    onTap: ((Offset) -> Unit)? = null,
) {
    onTapCallback = onTap
    onLongPressCallback = onLongPress
}

// ============================================================================
// Pure `compose.runtime` `IguanodonModifier` (`typealias Modifier = IguanodonModifier`)
// ============================================================================

typealias Modifier = IguanodonModifier

@Stable
open class IguanodonModifier(
    val padLeftDp: Float = 0f,
    val padTopDp: Float = 0f,
    val padRightDp: Float = 0f,
    val padBottomDp: Float = 0f,
    val widthDp: Float = 0f,
    val heightDp: Float = 0f,
    val minWidthDp: Float = 0f,
    val maxWidthDp: Float = Float.MAX_VALUE,
    val minHeightDp: Float = 0f,
    val maxHeightDp: Float = Float.MAX_VALUE,
    val weight: Float = 0f,
    val offsetXDp: Float = 0f,
    val offsetYDp: Float = 0f,
    val bgRgba: Int = 0,
    val borderRgba: Int = 0,
    val borderW: Float = 0f,
    val cornerRadiusDp: Float = 0f,
    val clip: Boolean = false,
    val alpha: Float = 1f,
    val shaderId: Byte = 0,
    val shaderP0: Float = 0f,
    val shaderP1: Float = 0f,
    val shaderP2: Float = 0f,
    val shaderP3: Float = 0f,
    val scrollState: ScrollState? = null,
    val scrollAxis: Byte = 0,
    val onClick: (() -> Unit)? = null,
    val onLongClick: (() -> Unit)? = null,
    val onHoverChanged: ((Boolean) -> Unit)? = null,
    val onDragDelta: ((Float, Float) -> Unit)? = null,
) {
    fun copyWith(
        padLeftDp: Float = this.padLeftDp,
        padTopDp: Float = this.padTopDp,
        padRightDp: Float = this.padRightDp,
        padBottomDp: Float = this.padBottomDp,
        widthDp: Float = this.widthDp,
        heightDp: Float = this.heightDp,
        minWidthDp: Float = this.minWidthDp,
        maxWidthDp: Float = this.maxWidthDp,
        minHeightDp: Float = this.minHeightDp,
        maxHeightDp: Float = this.maxHeightDp,
        weight: Float = this.weight,
        offsetXDp: Float = this.offsetXDp,
        offsetYDp: Float = this.offsetYDp,
        bgRgba: Int = this.bgRgba,
        borderRgba: Int = this.borderRgba,
        borderW: Float = this.borderW,
        cornerRadiusDp: Float = this.cornerRadiusDp,
        clip: Boolean = this.clip,
        alpha: Float = this.alpha,
        shaderId: Byte = this.shaderId,
        shaderP0: Float = this.shaderP0,
        shaderP1: Float = this.shaderP1,
        shaderP2: Float = this.shaderP2,
        shaderP3: Float = this.shaderP3,
        scrollState: ScrollState? = this.scrollState,
        scrollAxis: Byte = this.scrollAxis,
        onClick: (() -> Unit)? = this.onClick,
        onLongClick: (() -> Unit)? = this.onLongClick,
        onHoverChanged: ((Boolean) -> Unit)? = this.onHoverChanged,
        onDragDelta: ((Float, Float) -> Unit)? = this.onDragDelta,
    ): IguanodonModifier = IguanodonModifier(
        padLeftDp = padLeftDp,
        padTopDp = padTopDp,
        padRightDp = padRightDp,
        padBottomDp = padBottomDp,
        widthDp = widthDp,
        heightDp = heightDp,
        minWidthDp = minWidthDp,
        maxWidthDp = maxWidthDp,
        minHeightDp = minHeightDp,
        maxHeightDp = maxHeightDp,
        weight = weight,
        offsetXDp = offsetXDp,
        offsetYDp = offsetYDp,
        bgRgba = bgRgba,
        borderRgba = borderRgba,
        borderW = borderW,
        cornerRadiusDp = cornerRadiusDp,
        clip = clip,
        alpha = alpha,
        shaderId = shaderId,
        shaderP0 = shaderP0,
        shaderP1 = shaderP1,
        shaderP2 = shaderP2,
        shaderP3 = shaderP3,
        scrollState = scrollState,
        scrollAxis = scrollAxis,
        onClick = onClick,
        onLongClick = onLongClick,
        onHoverChanged = onHoverChanged,
        onDragDelta = onDragDelta,
    )

    infix fun then(other: IguanodonModifier): IguanodonModifier {
        if (other === Companion) return this
        if (this === Companion) return other
        return IguanodonModifier(
            padLeftDp = this.padLeftDp + other.padLeftDp,
            padTopDp = this.padTopDp + other.padTopDp,
            padRightDp = this.padRightDp + other.padRightDp,
            padBottomDp = this.padBottomDp + other.padBottomDp,
            widthDp = if (other.widthDp > 0f) other.widthDp else this.widthDp,
            heightDp = if (other.heightDp > 0f) other.heightDp else this.heightDp,
            minWidthDp = maxOf(this.minWidthDp, other.minWidthDp),
            maxWidthDp = minOf(this.maxWidthDp, other.maxWidthDp),
            minHeightDp = maxOf(this.minHeightDp, other.minHeightDp),
            maxHeightDp = minOf(this.maxHeightDp, other.maxHeightDp),
            weight = if (other.weight > 0f) other.weight else this.weight,
            offsetXDp = this.offsetXDp + other.offsetXDp,
            offsetYDp = this.offsetYDp + other.offsetYDp,
            bgRgba = if (other.bgRgba != 0) other.bgRgba else this.bgRgba,
            borderRgba = if (other.borderRgba != 0) other.borderRgba else this.borderRgba,
            borderW = if (other.borderW > 0f) other.borderW else this.borderW,
            cornerRadiusDp = if (other.cornerRadiusDp > 0f) other.cornerRadiusDp else this.cornerRadiusDp,
            clip = this.clip || other.clip,
            alpha = this.alpha * other.alpha,
            shaderId = if (other.shaderId != 0.toByte()) other.shaderId else this.shaderId,
            shaderP0 = if (other.shaderId != 0.toByte()) other.shaderP0 else this.shaderP0,
            shaderP1 = if (other.shaderId != 0.toByte()) other.shaderP1 else this.shaderP1,
            shaderP2 = if (other.shaderId != 0.toByte()) other.shaderP2 else this.shaderP2,
            shaderP3 = if (other.shaderId != 0.toByte()) other.shaderP3 else this.shaderP3,
            scrollState = other.scrollState ?: this.scrollState,
            scrollAxis = if (other.scrollState != null) other.scrollAxis else this.scrollAxis,
            onClick = other.onClick ?: this.onClick,
            onLongClick = other.onLongClick ?: this.onLongClick,
            onHoverChanged = other.onHoverChanged ?: this.onHoverChanged,
            onDragDelta = other.onDragDelta ?: this.onDragDelta,
        )
    }

    companion object : IguanodonModifier()
}

// ============================================================================
// Standard Layout, Visual, Gesture & WindowInsets Modifiers
// ============================================================================

fun Modifier.padding(all: Dp): Modifier = copyWith(
    padLeftDp = padLeftDp + all.value,
    padTopDp = padTopDp + all.value,
    padRightDp = padRightDp + all.value,
    padBottomDp = padBottomDp + all.value,
)

fun Modifier.padding(horizontal: Dp = 0.dp, vertical: Dp = 0.dp): Modifier = copyWith(
    padLeftDp = padLeftDp + horizontal.value,
    padTopDp = padTopDp + vertical.value,
    padRightDp = padRightDp + horizontal.value,
    padBottomDp = padBottomDp + vertical.value,
)

fun Modifier.padding(
    start: Dp = 0.dp,
    top: Dp = 0.dp,
    end: Dp = 0.dp,
    bottom: Dp = 0.dp,
): Modifier = copyWith(
    padLeftDp = padLeftDp + start.value,
    padTopDp = padTopDp + top.value,
    padRightDp = padRightDp + end.value,
    padBottomDp = padBottomDp + bottom.value,
)

fun Modifier.padding(paddingValues: PaddingValues): Modifier = padding(
    start = paddingValues.start,
    top = paddingValues.top,
    end = paddingValues.end,
    bottom = paddingValues.bottom,
)

fun Modifier.fillMaxWidth(fraction: Float = 1f): Modifier = this
fun Modifier.fillMaxHeight(fraction: Float = 1f): Modifier = this
fun Modifier.fillMaxSize(fraction: Float = 1f): Modifier = this

fun Modifier.width(width: Dp): Modifier = copyWith(widthDp = width.value)
fun Modifier.height(height: Dp): Modifier = copyWith(heightDp = height.value)
fun Modifier.size(size: Dp): Modifier = copyWith(widthDp = size.value, heightDp = size.value)
fun Modifier.size(width: Dp, height: Dp): Modifier = copyWith(widthDp = width.value, heightDp = height.value)

fun Modifier.widthIn(min: Dp = 0.dp, max: Dp = Dp.Infinity): Modifier =
    copyWith(minWidthDp = min.value, maxWidthDp = max.value)

fun Modifier.heightIn(min: Dp = 0.dp, max: Dp = Dp.Infinity): Modifier =
    copyWith(minHeightDp = min.value, maxHeightDp = max.value)

fun Modifier.sizeIn(
    minWidth: Dp = 0.dp,
    minHeight: Dp = 0.dp,
    maxWidth: Dp = Dp.Infinity,
    maxHeight: Dp = Dp.Infinity,
): Modifier = copyWith(
    minWidthDp = minWidth.value,
    minHeightDp = minHeight.value,
    maxWidthDp = maxWidth.value,
    maxHeightDp = maxHeight.value,
)

fun Modifier.weight(weight: Float, fill: Boolean = true): Modifier = copyWith(weight = weight)

fun Modifier.offset(x: Dp = 0.dp, y: Dp = 0.dp): Modifier =
    copyWith(offsetXDp = x.value, offsetYDp = y.value)

fun Modifier.offset(offset: Density.() -> IntOffset): Modifier {
    val resolved = Density(1f, 1f).offset()
    return copyWith(offsetXDp = resolved.x.toFloat(), offsetYDp = resolved.y.toFloat())
}

fun Modifier.background(color: Color, shape: Shape = RectangleShape): Modifier = copyWith(
    bgRgba = color.toArgb(),
    cornerRadiusDp = if (shape.cornerRadiusDp > 0f) shape.cornerRadiusDp else cornerRadiusDp,
)

fun Modifier.border(width: Dp, color: Color, shape: Shape = RectangleShape): Modifier = copyWith(
    borderW = width.value,
    borderRgba = color.toArgb(),
    cornerRadiusDp = if (shape.cornerRadiusDp > 0f) shape.cornerRadiusDp else cornerRadiusDp,
)

fun Modifier.border(border: BorderStroke, shape: Shape = RectangleShape): Modifier =
    border(width = border.width, color = border.color, shape = shape)

fun Modifier.clip(shape: Shape): Modifier = copyWith(
    cornerRadiusDp = shape.cornerRadiusDp,
    clip = true,
)

fun Modifier.alpha(alpha: Float): Modifier = copyWith(alpha = alpha.coerceIn(0f, 1f))

fun Modifier.clickable(
    enabled: Boolean = true,
    onClickLabel: String? = null,
    interactionSource: MutableInteractionSource? = null,
    indication: Any? = null,
    onClick: () -> Unit,
): Modifier = if (enabled) copyWith(onClick = onClick) else this

fun Modifier.combinedClickable(
    enabled: Boolean = true,
    onLongClick: (() -> Unit)? = null,
    onDoubleClick: (() -> Unit)? = null,
    onClick: () -> Unit,
): Modifier = if (enabled) copyWith(onClick = onClick, onLongClick = onLongClick) else this

fun Modifier.hoverable(
    interactionSource: MutableInteractionSource,
    enabled: Boolean = true,
): Modifier = if (enabled) {
    copyWith(onHoverChanged = { hovered -> interactionSource.hoveredState.value = hovered })
} else {
    this
}

fun Modifier.verticalScroll(state: ScrollState, enabled: Boolean = true): Modifier =
    if (enabled) copyWith(scrollState = state, scrollAxis = 0) else this

fun Modifier.horizontalScroll(state: ScrollState, enabled: Boolean = true): Modifier =
    if (enabled) copyWith(scrollState = state, scrollAxis = 1) else this

fun Modifier.pointerInput(
    key1: Any?,
    block: suspend PointerInputScope.() -> Unit,
): Modifier {
    val scope = PointerInputScope()
    return copyWith(
        onDragDelta = { dx, dy -> scope.onDragDeltaCallback?.invoke(dx, dy) },
    )
}

fun Modifier.pointerInput(
    key1: Any?,
    key2: Any?,
    block: suspend PointerInputScope.() -> Unit,
): Modifier = pointerInput(key1, block)

// ============================================================================
// First-Class GPU Shader & Physics Modifier Extensions for `IguanodonModifier`
// ============================================================================

enum class IguanodonShaderMaterialId(val id: Int, val displayName: String) {
    StandardSdf(0, "Standard SDF"),
    FrostedGlass(1, "Frosted Glass"),
    ProceduralFire(2, "Procedural Fire"),
    WaterRipple(3, "Water Ripple"),
    FluentRevealHighlight(4, "Fluent Reveal"),
    ObsidianNeonPulse(5, "Obsidian Neon Pulse"),
    TexturedImage(6, "Textured Image"),
    GentleSmoke(7, "Gentle Smoke"),
    GentlePulsingLights(8, "Pulsing Lights"),
    ReflectiveGlass(9, "Reflective Glass"),
    ColorCyclingGlowbar(10, "Color-Cycling Glowbar"),
    FlowingGentleGradient(11, "Flowing Yellow-White"),
    ElectricJellyfish(12, "Electric Jellyfish"),
    SubtleGlassReflection(13, "Subtle Reflection"),
}

fun Modifier.frostedGlass(
    blurRadius: Float = 24f,
    noiseGrain: Float = 0.03f,
    refraction: Float = 0.25f,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.FrostedGlass.id.toByte(),
    shaderP0 = blurRadius,
    shaderP1 = noiseGrain,
    shaderP2 = refraction,
)

fun Modifier.fireEmitter(
    intensity: Float = 1.0f,
    turbulence: Float = 0.8f,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.ProceduralFire.id.toByte(),
    shaderP0 = intensity,
    shaderP1 = turbulence,
)

fun Modifier.waterRipple(
    damping: Float = 0.96f,
    frequency: Float = 10.0f,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.WaterRipple.id.toByte(),
    shaderP0 = damping,
    shaderP1 = frequency,
)

fun Modifier.revealHighlight(radius: Float = 120f): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.FluentRevealHighlight.id.toByte(),
    shaderP0 = radius,
)

fun Modifier.gentleSmoke(
    density: Float = 0.65f,
    driftSpeed: Float = 0.20f,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.GentleSmoke.id.toByte(),
    shaderP0 = density,
    shaderP1 = driftSpeed,
)

fun Modifier.gentleSmokeBorder(
    borderWidthDp: Float = 2.0f,
    plumeReachPx: Float = 28.0f,
    driftSpeed: Float = 0.28f,
): Modifier = copyWith(
    borderW = maxOf(borderW, borderWidthDp),
    shaderId = IguanodonShaderMaterialId.GentleSmoke.id.toByte(),
    shaderP0 = borderWidthDp,
    shaderP1 = plumeReachPx,
    shaderP2 = driftSpeed,
    shaderP3 = 2f,
)

fun Modifier.fireBorder(
    borderWidthDp: Float = 1.5f,
    coronaReachPx: Float = 11.5f,
    intensity: Float = 0.90f,
): Modifier = copyWith(
    borderW = maxOf(borderW, borderWidthDp),
    shaderId = IguanodonShaderMaterialId.ProceduralFire.id.toByte(),
    shaderP0 = borderWidthDp,
    shaderP1 = coronaReachPx,
    shaderP2 = intensity,
    shaderP3 = 1f,
)

fun Modifier.pulsingLights(
    frequencyHz: Float = 0.18f,
    bloomIntensity: Float = 0.75f,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.GentlePulsingLights.id.toByte(),
    shaderP0 = frequencyHz,
    shaderP1 = bloomIntensity,
)

fun Modifier.reflectiveGlass(
    fresnelPower: Float = 2.0f,
    sheenIntensity: Float = 0.65f,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.ReflectiveGlass.id.toByte(),
    shaderP0 = fresnelPower,
    shaderP1 = sheenIntensity,
)

/**
 * Applies a whisper-soft, delicate diagonal optical reflection sheen and faint top glint
 * (`ShaderMaterialId::SubtleGlassReflection = 13`), much more subtle than [reflectiveGlass].
 */
fun Modifier.subtleReflection(
    sheenIntensity: Float = 0.12f,
    topGlint: Float = 0.055f,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.SubtleGlassReflection.id.toByte(),
    shaderP0 = sheenIntensity,
    shaderP1 = topGlint,
)

fun Modifier.colorCyclingGlowbar(
    cycleSpeedHz: Float = 0.065f,
    bloomRadiusPx: Float = 10.0f,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.ColorCyclingGlowbar.id.toByte(),
    shaderP0 = cycleSpeedHz,
    shaderP1 = bloomRadiusPx,
)

fun Modifier.flowingGradient(
    baseColor: Color = Color(0xFFFAD638),
    peakWhiteColor: Color = Color(0xFFFFFAE0),
    speedCyclesPerSec: Float = 0.085f,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.FlowingGentleGradient.id.toByte(),
    shaderP0 = speedCyclesPerSec,
)

/**
 * Applies the **Electric Jellyfish Backdrop** shader (`ShaderMaterialId::ElectricJellyfish = 12`):
 * a deep abyssal ocean blue gradient with 5 slowly drifting, gently pulsing bioluminescent
 * jellyfish-like light blooms (without legs) viewed through a diffuse opaque pane of frosted glass.
 */
fun Modifier.electricJellyfishBackdrop(
    darkTheme: Boolean = true,
): Modifier = copyWith(
    shaderId = IguanodonShaderMaterialId.ElectricJellyfish.id.toByte(),
    shaderP0 = if (darkTheme) 1.0f else 0.0f,
)

/**
 * Full-screen animated backdrop composable that renders gently pulsing, slowly drifting
 * bioluminescent electric jellyfish light blooms (no legs) viewed through a diffuse opaque
 * pane of frosted glass.
 */
@Composable
fun ElectricJellyfishBackdrop(
    darkTheme: Boolean = LocalIguanodonThemeMode.current == IguanodonThemeMode.Dark,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    if (modifier === Modifier) {
        content()
    } else {
        Box(modifier = modifier.fillMaxSize().electricJellyfishBackdrop(darkTheme = darkTheme)) {
            content()
        }
    }
}

fun Modifier.jelloClickWobble(
    dampingRatio: Float = 0.22f,
    stiffness: Float = 680.0f,
): Modifier = this

fun Modifier.overspringScroll(
    dampingRatio: Float = 0.52f,
    stiffness: Float = 420.0f,
): Modifier = this

fun Modifier.customShader(wgslSource: String): Modifier = this

// ============================================================================
// All 4 Iguanodon Themes in Dark & Light Modes (`IguanodonThemeId` × `IguanodonThemeMode`)
// ============================================================================

@Immutable
data class ColorScheme(
    val primary: Color,
    val onPrimary: Color,
    val primaryContainer: Color,
    val onPrimaryContainer: Color,
    val secondary: Color,
    val onSecondary: Color,
    val secondaryContainer: Color,
    val onSecondaryContainer: Color,
    val tertiary: Color,
    val onTertiary: Color = Color.White,
    val tertiaryContainer: Color = secondaryContainer,
    val onTertiaryContainer: Color = onSecondaryContainer,
    val background: Color,
    val onBackground: Color,
    val surface: Color,
    val onSurface: Color,
    val surfaceVariant: Color,
    val onSurfaceVariant: Color,
    val outline: Color = onSurfaceVariant,
    val outlineVariant: Color = surfaceVariant,
    val error: Color = Color(0xFFF2B8B5),
    val onError: Color = Color(0xFF601410),
    val errorContainer: Color = Color(0xFF8C1D18),
    val onErrorContainer: Color = Color(0xFFF9DEDC),
)

fun darkColorScheme(
    primary: Color = Color(0xFFD0BCFF),
    onPrimary: Color = Color(0xFF381E72),
    primaryContainer: Color = Color(0xFF4F378B),
    onPrimaryContainer: Color = Color(0xFFEADDFF),
    secondary: Color = Color(0xFFCCC2DC),
    onSecondary: Color = Color(0xFF332D41),
    secondaryContainer: Color = Color(0xFF4A4458),
    onSecondaryContainer: Color = Color(0xFFE8DEF8),
    tertiary: Color = Color(0xFFEFB8C8),
    onTertiary: Color = Color(0xFF492532),
    tertiaryContainer: Color = Color(0xFF633B48),
    onTertiaryContainer: Color = Color(0xFFFFD8E4),
    background: Color = Color(0xFF141218),
    onBackground: Color = Color(0xFFE6E0E9),
    surface: Color = Color(0xFF1D1B20),
    onSurface: Color = Color(0xFFE6E0E9),
    surfaceVariant: Color = Color(0xFF49454F),
    onSurfaceVariant: Color = Color(0xFFCAC4D0),
    outline: Color = Color(0xFF938F99),
    outlineVariant: Color = Color(0xFF49454F),
    error: Color = Color(0xFFF2B8B5),
    onError: Color = Color(0xFF601410),
    errorContainer: Color = Color(0xFF8C1D18),
    onErrorContainer: Color = Color(0xFFF9DEDC),
): ColorScheme = ColorScheme(
    primary = primary,
    onPrimary = onPrimary,
    primaryContainer = primaryContainer,
    onPrimaryContainer = onPrimaryContainer,
    secondary = secondary,
    onSecondary = onSecondary,
    secondaryContainer = secondaryContainer,
    onSecondaryContainer = onSecondaryContainer,
    tertiary = tertiary,
    onTertiary = onTertiary,
    tertiaryContainer = tertiaryContainer,
    onTertiaryContainer = onTertiaryContainer,
    background = background,
    onBackground = onBackground,
    surface = surface,
    onSurface = onSurface,
    surfaceVariant = surfaceVariant,
    onSurfaceVariant = onSurfaceVariant,
    outline = outline,
    outlineVariant = outlineVariant,
    error = error,
    onError = onError,
    errorContainer = errorContainer,
    onErrorContainer = onErrorContainer,
)

fun lightColorScheme(
    primary: Color = Color(0xFF6750A4),
    onPrimary: Color = Color(0xFFFFFFFF),
    primaryContainer: Color = Color(0xFFEADDFF),
    onPrimaryContainer: Color = Color(0xFF21005D),
    secondary: Color = Color(0xFF625B71),
    onSecondary: Color = Color(0xFFFFFFFF),
    secondaryContainer: Color = Color(0xFFE8DEF8),
    onSecondaryContainer: Color = Color(0xFF1D192B),
    tertiary: Color = Color(0xFF7D5260),
    onTertiary: Color = Color(0xFFFFFFFF),
    tertiaryContainer: Color = Color(0xFFFFD8E4),
    onTertiaryContainer: Color = Color(0xFF31111D),
    background: Color = Color(0xFFFEF7FF),
    onBackground: Color = Color(0xFF1D1B20),
    surface: Color = Color(0xFFF7F2FA),
    onSurface: Color = Color(0xFF1D1B20),
    surfaceVariant: Color = Color(0xFFE7E0EC),
    onSurfaceVariant: Color = Color(0xFF49454F),
    outline: Color = Color(0xFF79747E),
    outlineVariant: Color = Color(0xFFCAC4D0),
    error: Color = Color(0xFFB3261E),
    onError: Color = Color(0xFFFFFFFF),
    errorContainer: Color = Color(0xFFF9DEDC),
    onErrorContainer: Color = Color(0xFF410E0B),
): ColorScheme = ColorScheme(
    primary = primary,
    onPrimary = onPrimary,
    primaryContainer = primaryContainer,
    onPrimaryContainer = onPrimaryContainer,
    secondary = secondary,
    onSecondary = onSecondary,
    secondaryContainer = secondaryContainer,
    onSecondaryContainer = onSecondaryContainer,
    tertiary = tertiary,
    onTertiary = onTertiary,
    tertiaryContainer = tertiaryContainer,
    onTertiaryContainer = onTertiaryContainer,
    background = background,
    onBackground = onBackground,
    surface = surface,
    onSurface = onSurface,
    surfaceVariant = surfaceVariant,
    onSurfaceVariant = onSurfaceVariant,
    outline = outline,
    outlineVariant = outlineVariant,
    error = error,
    onError = onError,
    errorContainer = errorContainer,
    onErrorContainer = onErrorContainer,
)

val LocalColorScheme = staticCompositionLocalOf { darkColorScheme() }
val LocalTypography = staticCompositionLocalOf { Typography() }
val LocalShapes = staticCompositionLocalOf { Shapes() }

object MaterialTheme {
    val colorScheme: ColorScheme
        @Composable get() = LocalColorScheme.current
    val typography: Typography
        @Composable get() = LocalTypography.current
    val shapes: Shapes
        @Composable get() = LocalShapes.current
}

fun resolveIguanodonColorScheme(
    themeId: IguanodonThemeId,
    darkTheme: Boolean,
): ColorScheme = when (themeId) {
    IguanodonThemeId.Material3Expressive -> if (darkTheme) {
        darkColorScheme(
            primary = Color(0xFFD0BCFF),
            onPrimary = Color(0xFF381E72),
            primaryContainer = Color(0xFF4F378B),
            onPrimaryContainer = Color(0xFFEADDFF),
            secondary = Color(0xFFCCC2DC),
            onSecondary = Color(0xFF332D41),
            secondaryContainer = Color(0xFF4A4458),
            onSecondaryContainer = Color(0xFFE8DEF8),
            tertiary = Color(0xFFEFB8C8),
            background = Color(0xFF141218),
            onBackground = Color(0xFFE6E0E9),
            surface = Color(0xFF1D1B20),
            onSurface = Color(0xFFE6E0E9),
            surfaceVariant = Color(0xFF49454F),
            onSurfaceVariant = Color(0xFFCAC4D0),
        )
    } else {
        lightColorScheme(
            primary = Color(0xFF6750A4),
            onPrimary = Color(0xFFFFFFFF),
            primaryContainer = Color(0xFFEADDFF),
            onPrimaryContainer = Color(0xFF21005D),
            secondary = Color(0xFF625B71),
            onSecondary = Color(0xFFFFFFFF),
            secondaryContainer = Color(0xFFE8DEF8),
            onSecondaryContainer = Color(0xFF1D192B),
            tertiary = Color(0xFF7D5260),
            background = Color(0xFFFEF7FF),
            onBackground = Color(0xFF1D1B20),
            surface = Color(0xFFF7F2FA),
            onSurface = Color(0xFF1D1B20),
            surfaceVariant = Color(0xFFE7E0EC),
            onSurfaceVariant = Color(0xFF49454F),
        )
    }

    IguanodonThemeId.WindowsFluent2 -> if (darkTheme) {
        darkColorScheme(
            primary = Color(0xFF60CDFF),
            onPrimary = Color(0xFF002D42),
            primaryContainer = Color(0xFF004C6D),
            onPrimaryContainer = Color(0xFFD6F0FF),
            secondary = Color(0xFF9FB4C7),
            onSecondary = Color(0xFF1A2833),
            secondaryContainer = Color(0xFF2B3B48),
            onSecondaryContainer = Color(0xFFD8E6F3),
            tertiary = Color(0xFF7AD7F0),
            background = Color(0xFF181C20),
            onBackground = Color(0xFFEBF0F5),
            surface = Color(0xFF20252B),
            onSurface = Color(0xFFEBF0F5),
            surfaceVariant = Color(0xFF323942),
            onSurfaceVariant = Color(0xFFC2CCD6),
        )
    } else {
        lightColorScheme(
            primary = Color(0xFF0067C0),
            onPrimary = Color(0xFFFFFFFF),
            primaryContainer = Color(0xFFD8ECFF),
            onPrimaryContainer = Color(0xFF002447),
            secondary = Color(0xFF495B6E),
            onSecondary = Color(0xFFFFFFFF),
            secondaryContainer = Color(0xFFDCE6F2),
            onSecondaryContainer = Color(0xFF16222F),
            tertiary = Color(0xFF0078D4),
            background = Color(0xFFF3F6F9),
            onBackground = Color(0xFF1B1F24),
            surface = Color(0xFFFFFFFF),
            onSurface = Color(0xFF1B1F24),
            surfaceVariant = Color(0xFFE4E9F0),
            onSurfaceVariant = Color(0xFF48525E),
        )
    }

    IguanodonThemeId.IosLiquidGlass -> if (darkTheme) {
        darkColorScheme(
            primary = Color(0xFF0A84FF),
            onPrimary = Color(0xFFFFFFFF),
            primaryContainer = Color(0xFF123E73),
            onPrimaryContainer = Color(0xFFD8ECFF),
            secondary = Color(0xFF64D2FF),
            onSecondary = Color(0xFF002F42),
            secondaryContainer = Color(0xFF1B3A4B),
            onSecondaryContainer = Color(0xFFD4F4FF),
            tertiary = Color(0xFF5E5CE6),
            background = Color(0xFF0E131A),
            onBackground = Color(0xFFF2F5FA),
            surface = Color(0xFF18202C),
            onSurface = Color(0xFFF2F5FA),
            surfaceVariant = Color(0xFF293446),
            onSurfaceVariant = Color(0xFFC7D2E2),
        )
    } else {
        lightColorScheme(
            primary = Color(0xFF007AFF),
            onPrimary = Color(0xFFFFFFFF),
            primaryContainer = Color(0xFFDCEEFF),
            onPrimaryContainer = Color(0xFF002960),
            secondary = Color(0xFF32ADE6),
            onSecondary = Color(0xFFFFFFFF),
            secondaryContainer = Color(0xFFD8F3FF),
            onSecondaryContainer = Color(0xFF002B3D),
            tertiary = Color(0xFF5856D6),
            background = Color(0xFFF2F5FA),
            onBackground = Color(0xFF121821),
            surface = Color(0xFFFFFFFF),
            onSurface = Color(0xFF121821),
            surfaceVariant = Color(0xFFE2E8F2),
            onSurfaceVariant = Color(0xFF434E61),
        )
    }

    IguanodonThemeId.LinuxEliteObsidian -> if (darkTheme) {
        darkColorScheme(
            primary = Color(0xFFFF3B30),
            onPrimary = Color(0xFFFFFFFF),
            primaryContainer = Color(0xFF5C1010),
            onPrimaryContainer = Color(0xFFFFDAD6),
            secondary = Color(0xFFFF6B35),
            onSecondary = Color(0xFF3A0F00),
            secondaryContainer = Color(0xFF4D1F12),
            onSecondaryContainer = Color(0xFFFFDBCF),
            tertiary = Color(0xFFFF9F1C),
            background = Color(0xFF0A0A0D),
            onBackground = Color(0xFFF4EDE8),
            surface = Color(0xFF14141A),
            onSurface = Color(0xFFF4EDE8),
            surfaceVariant = Color(0xFF2B2226),
            onSurfaceVariant = Color(0xFFDDC2C0),
        )
    } else {
        lightColorScheme(
            primary = Color(0xFFD62828),
            onPrimary = Color(0xFFFFFFFF),
            primaryContainer = Color(0xFFFFDAD6),
            onPrimaryContainer = Color(0xFF410002),
            secondary = Color(0xFFE85D04),
            onSecondary = Color(0xFFFFFFFF),
            secondaryContainer = Color(0xFFFFDBCF),
            onSecondaryContainer = Color(0xFF380D00),
            tertiary = Color(0xFFF77F00),
            background = Color(0xFFFAF6F4),
            onBackground = Color(0xFF1B1515),
            surface = Color(0xFFFFFFFF),
            onSurface = Color(0xFF1B1515),
            surfaceVariant = Color(0xFFF2E2DF),
            onSurfaceVariant = Color(0xFF534341),
        )
    }
}

@Composable
fun MaterialExpressiveTheme(
    themeId: IguanodonThemeId = IguanodonThemeId.Material3Expressive,
    darkTheme: Boolean = true,
    fontScale: Float = 1.0f,
    content: @Composable () -> Unit,
) {
    val session = LocalIguanodonSession.current
    val mode = if (darkTheme) IguanodonThemeMode.Dark else IguanodonThemeMode.Light
    var lastSyncedTheme by remember { mutableStateOf<IguanodonThemeId?>(null) }
    var lastSyncedMode by remember { mutableStateOf<IguanodonThemeMode?>(null) }
    SideEffect {
        if (lastSyncedTheme != themeId) {
            lastSyncedTheme = themeId
            session?.encoder?.encodeSetTheme(themeId)
        }
        if (lastSyncedMode != mode) {
            lastSyncedMode = mode
            session?.encoder?.encodeSetThemeMode(mode)
            session?.notifyThemeModeChanged(mode)
        }
    }
    val scheme = resolveIguanodonColorScheme(themeId = themeId, darkTheme = darkTheme)
    val baseDensity = LocalDensity.current
    val clampedScale = fontScale.coerceIn(0.75f, 2.0f)
    val scaledDensity = Density(
        density = baseDensity.density,
        fontScale = baseDensity.fontScale * clampedScale,
    )
    CompositionLocalProvider(
        LocalIguanodonThemeId provides themeId,
        LocalIguanodonThemeMode provides mode,
        LocalIguanodonFontScale provides clampedScale,
        LocalDensity provides scaledDensity,
        LocalColorScheme provides scheme,
        LocalContentColor provides scheme.onSurface,
    ) {
        content()
    }
}

@Composable
fun IguanodonTheme(
    themeId: IguanodonThemeId = IguanodonThemeId.Material3Expressive,
    mode: IguanodonThemeMode = IguanodonThemeMode.Dark,
    fontScale: Float = 1.0f,
    content: @Composable () -> Unit,
) = MaterialExpressiveTheme(
    themeId = themeId,
    darkTheme = (mode == IguanodonThemeMode.Dark),
    fontScale = fontScale,
    content = content,
)

// ============================================================================
// Pure `compose.runtime` `ComposeNode<IguanodonNode, IguanodonApplier>` Primitives
// ============================================================================

interface RowScope {
    fun Modifier.weight(weight: Float, fill: Boolean = true): Modifier =
        this.copyWith(weight = weight)
}

internal object RowScopeInstance : RowScope

interface ColumnScope {
    fun Modifier.weight(weight: Float, fill: Boolean = true): Modifier =
        this.copyWith(weight = weight)
}

internal object ColumnScopeInstance : ColumnScope

interface BoxScope {
    fun Modifier.align(alignment: Alignment): Modifier = this
}

internal object BoxScopeInstance : BoxScope

class BoxWithConstraintsScope(
    val maxWidth: Dp,
    val maxHeight: Dp,
    val minWidth: Dp = 0.dp,
    val minHeight: Dp = 0.dp,
) : BoxScope by BoxScopeInstance

@Composable
fun Row(
    modifier: Modifier = Modifier,
    horizontalArrangement: Arrangement.Horizontal = Arrangement.Start,
    verticalAlignment: Alignment.Vertical = Alignment.Top,
    content: @Composable RowScope.() -> Unit,
) {
    val session = LocalIguanodonSession.current
    val nodeId = remember { session?.allocateNodeId() ?: 0L }
    ComposeNode<IguanodonNode, IguanodonApplier>(
        factory = { IguanodonNode(nodeId = nodeId, nodeKind = NodeKinds.ROW) },
        update = {
            set(modifier) { mod ->
                session?.syncModifierToNode(
                    node = this,
                    modifier = mod,
                    flexDir = 1,
                    align = verticalAlignment.code,
                    justify = horizontalArrangement.code,
                    gapDp = horizontalArrangement.spacingDp,
                )
            }
        },
        content = { RowScopeInstance.content() },
    )
}

@Composable
fun Column(
    modifier: Modifier = Modifier,
    verticalArrangement: Arrangement.Vertical = Arrangement.Top,
    horizontalAlignment: Alignment.Horizontal = Alignment.Start,
    content: @Composable ColumnScope.() -> Unit,
) {
    val session = LocalIguanodonSession.current
    val nodeId = remember { session?.allocateNodeId() ?: 0L }
    ComposeNode<IguanodonNode, IguanodonApplier>(
        factory = { IguanodonNode(nodeId = nodeId, nodeKind = NodeKinds.COLUMN) },
        update = {
            set(modifier) { mod ->
                session?.syncModifierToNode(
                    node = this,
                    modifier = mod,
                    flexDir = 0,
                    align = horizontalAlignment.code,
                    justify = verticalArrangement.code,
                    gapDp = verticalArrangement.spacingDp,
                )
            }
        },
        content = { ColumnScopeInstance.content() },
    )
}

@Composable
fun Box(
    modifier: Modifier = Modifier,
    contentAlignment: Alignment = Alignment.TopStart,
    propagateMinConstraints: Boolean = false,
    content: @Composable BoxScope.() -> Unit = {},
) {
    val session = LocalIguanodonSession.current
    val nodeId = remember { session?.allocateNodeId() ?: 0L }
    ComposeNode<IguanodonNode, IguanodonApplier>(
        factory = { IguanodonNode(nodeId = nodeId, nodeKind = NodeKinds.BOX) },
        update = {
            set(modifier) { mod ->
                session?.syncModifierToNode(
                    node = this,
                    modifier = mod,
                    flexDir = 2,
                    align = contentAlignment.code,
                )
            }
        },
        content = { BoxScopeInstance.content() },
    )
}

@Composable
fun BoxWithConstraints(
    modifier: Modifier = Modifier,
    contentAlignment: Alignment = Alignment.TopStart,
    content: @Composable BoxWithConstraintsScope.() -> Unit,
) {
    val session = LocalIguanodonSession.current
    val maxW = (session?.viewportWidth ?: 1180).dp
    val maxH = (session?.viewportHeight ?: 760).dp
    val scope = remember(maxW, maxH) { BoxWithConstraintsScope(maxWidth = maxW, maxHeight = maxH) }
    Box(modifier = modifier, contentAlignment = contentAlignment) {
        scope.content()
    }
}

@Composable
fun PerfOverlay(modifier: Modifier = Modifier) {
    val session = LocalIguanodonSession.current
    val frameMs = session?.latestFrameMs ?: 0.85f
    val targetFps = session?.targetFps ?: 120
    val backendName = session?.rendererBackend?.displayName ?: "Iguanodon"
    val nativeStatus = if (IguanodonNativeBridge.isNativeLoaded) "Vulkan SDF ($backendName)" else "Pure Compose Runtime"
    Surface(
        modifier = modifier
            .clip(RoundedCornerShape(12.dp))
            .border(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.35f), RoundedCornerShape(12.dp)),
        color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.85f),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "🦖 $nativeStatus • ${String.format("%.2f", frameMs)}ms • ${targetFps}Hz",
                fontFamily = FontFamily.Monospace,
                fontSize = 11.sp,
                fontWeight = FontWeight.Normal,
                color = MaterialTheme.colorScheme.onPrimaryContainer,
            )
        }
    }
}

// ============================================================================
// Core Iguanodon Benchmark & Retained Scene Primitives
// ============================================================================

@Composable
fun Label(
    text: String,
    nodeId: Int = 0,
    modifier: Modifier = Modifier,
) {
    val session = LocalIguanodonSession.current
    val color = MaterialTheme.colorScheme.onSurface
    val resolvedId = remember(nodeId) {
        if (nodeId != 0) nodeId.toLong() else (session?.allocateNodeId() ?: BenchNodeIds.HEADER_LABEL.toLong())
    }
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
                    colorRgba = color.toArgb(),
                    fontSizeSp = 15f,
                    fontWeight = FontWeight.Normal.weight,
                    maxLines = 1,
                    text = str,
                )
            }
        },
    )
}

@Composable
fun Button(
    label: String,
    nodeId: Int = BenchNodeIds.PRIMARY_BTN,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    val session = LocalIguanodonSession.current
    val scheme = MaterialTheme.colorScheme
    val btnMod = modifier
        .heightIn(min = MIN_TOUCH_TARGET_DP.dp)
        .background(scheme.primary, RoundedCornerShape(12.dp))
        .clickable {
            onClick()
        }
    ComposeNode<IguanodonNode, IguanodonApplier>(
        factory = { IguanodonNode(nodeId = nodeId.toLong(), nodeKind = NodeKinds.BUTTON) },
        update = {
            set(btnMod) { mod ->
                session?.syncModifierToNode(this, mod, flexDir = 1, align = 1, justify = 1)
            }
            set(label) { str ->
                this.textValue = str
                session?.encoder?.encodeSetText(
                    nodeId = this.nodeId,
                    colorRgba = scheme.onPrimary.toArgb(),
                    fontSizeSp = 14f,
                    fontWeight = FontWeight.Normal.weight,
                    maxLines = 1,
                    text = str,
                )
            }
        },
    )
}

@Composable
fun ScrollableText(
    document: String,
    scrollDeltaY: Float = 0f,
    modifier: Modifier = Modifier,
) {
    val session = LocalIguanodonSession.current
    SideEffect {
        if (scrollDeltaY != 0f) {
            session?.encoder?.encodeScrollNode(BenchNodeIds.SCROLLABLE_TEXT, scrollDeltaY)
        }
    }
    Text(
        text = document,
        modifier = modifier.padding(8.dp),
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
fun <T> LazyColumn(
    items: List<T>,
    showScrollbar: Boolean = true,
    scrollDeltaY: Float = 0f,
    modifier: Modifier = Modifier,
    itemContent: @Composable (index: Int, item: T) -> Unit,
) {
    val session = LocalIguanodonSession.current
    SideEffect {
        if (scrollDeltaY != 0f) {
            session?.encoder?.encodeScrollNode(BenchNodeIds.LAZY_COLUMN, scrollDeltaY)
        }
    }
    Column(modifier = modifier) {
        val previewCount = items.size.coerceAtMost(25)
        for (i in 0 until previewCount) {
            itemContent(i, items[i])
        }
    }
}

@Composable
fun ProgressBar(
    progress: Float? = null,
    modifier: Modifier = Modifier,
) {
    val session = LocalIguanodonSession.current
    val nodeId = remember { session?.allocateNodeId() ?: BenchNodeIds.DETERMINATE_PROGRESS.toLong() }
    ComposeNode<IguanodonNode, IguanodonApplier>(
        factory = { IguanodonNode(nodeId = nodeId, nodeKind = NodeKinds.PROGRESS) },
        update = {
            set(modifier.fillMaxWidth()) { mod ->
                session?.syncModifierToNode(this, mod)
            }
        },
    )
}

@Composable
fun Draggable(
    payloadId: String,
    nodeId: Int = BenchNodeIds.DRAGGABLE_CARD_A,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Box(modifier = modifier) {
        content()
    }
}

@Composable
fun DropTarget(
    onDrop: (payloadId: String) -> Unit,
    nodeId: Int = BenchNodeIds.DROP_TARGET_BUCKET,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Box(
        modifier = modifier.clickable { onDrop("Vulkan-Shader-Asset-01") },
    ) {
        content()
    }
}

@Composable
fun TextField(
    value: String,
    placeholder: String = "",
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val session = LocalIguanodonSession.current
    val nodeId = remember { session?.allocateNodeId() ?: BenchNodeIds.TEXT_FIELD_INPUT.toLong() }
    ComposeNode<IguanodonNode, IguanodonApplier>(
        factory = { IguanodonNode(nodeId = nodeId, nodeKind = NodeKinds.TEXT_FIELD) },
        update = {
            set(value) { v ->
                this.textValue = v
                session?.encoder?.encodeSetText(
                    nodeId = this.nodeId,
                    colorRgba = 0xFFE6E0E9.toInt(),
                    fontSizeSp = 14f,
                    fontWeight = FontWeight.Normal.weight,
                    maxLines = 1,
                    text = v.ifEmpty { placeholder },
                )
            }
            set(onValueChange) { cb ->
                this.onTextChange = cb
            }
            set(modifier.fillMaxWidth().heightIn(min = MIN_TOUCH_TARGET_DP.dp)) { mod ->
                session?.syncModifierToNode(this, mod)
            }
        },
    )
}

@Composable
fun ContextMenuRegion(
    items: List<String>,
    onSelectItem: (index: Int) -> Unit,
    content: @Composable () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        content()
    }
}

@Composable
fun Image(
    textureId: Int,
    contentDescription: String,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f), RoundedCornerShape(12.dp))
            .padding(12.dp),
    ) {
        Text(text = contentDescription, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
