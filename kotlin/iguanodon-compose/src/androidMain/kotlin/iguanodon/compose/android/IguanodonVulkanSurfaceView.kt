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

package iguanodon.compose.android

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.graphics.Bitmap
import android.graphics.Rect
import android.os.Build
import android.view.Choreographer
import android.view.GestureDetector
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.widget.FrameLayout
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import iguanodon.compose.IguanodonCompositionHost
import iguanodon.compose.IguanodonPowerMode
import iguanodon.compose.IguanodonThemeMode
import iguanodon.compose.LocalIguanodonSession
import iguanodon.compose.bridge.IguanodonSession
import kotlinx.coroutines.Dispatchers
import java.nio.ByteBuffer

/**
 * Pure `compose.runtime` Android host container (`FrameLayout` + Vulkan `SurfaceView`)
 * for Iguanodon's Rust/Vulkan engine (`iguanodon-vk`).
 *
 * Drives `Recomposer` + `BroadcastFrameClock` + `IguanodonApplier` directly via Android's
 * `Choreographer` without requiring `androidx.compose.ui.platform.ComposeView` or `compose-ui`.
 *
 * Preserves all upstream Android porting improvements:
 * 1. **Edge-to-Edge WindowInsets Forwarding**: Calls `v.onApplyWindowInsets(insets)` at the end of
 *    `setOnApplyWindowInsetsListener` and forwards scaled `WindowInsets.statusBars`, `navigationBars`,
 *    and `displayCutout` into `IguanodonSession.setSafeInsets(...)`.
 * 2. **Phone Landscape Viewport Height Support**: Clamps `dpW` and `dpH` with `.coerceAtLeast(240)`
 *    in `onSizeChanged` so `360dp`–`420dp` landscape phone screens are never artificially inflated to `480dp`.
 * 3. **Automatic System Bar Icon Polarity Sync**: Listens to `session.onThemeModeChanged` and
 *    automatically updates status bar and navigation bar icon polarity (`Light` vs `Dark`) whenever
 *    `IguanodonThemeMode` switches between `Dark` and `Light`.
 */
class IguanodonVulkanSurfaceView(
    context: Context,
    private val ambientFps: Int = 10,
    private val interactiveFps: Int = 30,
    private val content: @Composable () -> Unit,
) : FrameLayout(context), SurfaceHolder.Callback, Choreographer.FrameCallback {

    val session = IguanodonSession(initialWidth = 1420f, initialHeight = 920f)
    val compositionHost = IguanodonCompositionHost(session = session, parentContext = Dispatchers.Unconfined)
    private val backingSurfaceView = SurfaceView(context)
    private var rgbaBytes = ByteArray(1420 * 920 * 4)
    private var frameBitmap: Bitmap = Bitmap.createBitmap(1420, 920, Bitmap.Config.ARGB_8888)
    private var running = false
    private var windowVisible = true
    private var isTouchDown = false
    private val monotonicOrigin = kotlin.time.TimeSource.Monotonic.markNow()
    private var lastFrameNanos = 0L
    private var interactiveUntilNanos = 0L
    private var needsImmediateFrame = true
    private var appliedSurfaceW = 0
    private var appliedSurfaceH = 0
    private var longPressTriggeredOnCurrentTouch = false
    private var rawInsetsPx = IntArray(4) // [left, top, right, bottom] in physical screen pixels

    private val ambientIntervalNanos: Long = 1_000_000_000L / ambientFps.coerceIn(1, 60).toLong()
    private val interactiveIntervalNanos: Long = 1_000_000_000L / interactiveFps.coerceIn(10, 60).toLong()

    private fun monoNanos(): Long = monotonicOrigin.elapsedNow().inWholeNanoseconds

    private fun boostInteractive(durationMs: Long = 300L) {
        interactiveUntilNanos = monoNanos() + durationMs * 1_000_000L
        needsImmediateFrame = true
    }

    private fun mapTouchX(rawX: Float): Float =
        rawX * session.viewportWidth.toFloat() / width.coerceAtLeast(1).toFloat()

    private fun mapTouchY(rawY: Float): Float =
        rawY * session.viewportHeight.toFloat() / height.coerceAtLeast(1).toFloat()

    private fun applyScaledSafeInsets() {
        if (width <= 0 || height <= 0) return
        val scaleX = session.viewportWidth.toFloat() / width.toFloat()
        val scaleY = session.viewportHeight.toFloat() / height.toFloat()
        session.setSafeInsets(
            rawInsetsPx[0] * scaleX,
            rawInsetsPx[1] * scaleY,
            rawInsetsPx[2] * scaleX,
            rawInsetsPx[3] * scaleY,
        )
    }

    private fun findHostActivity(): Activity? {
        var ctx: Context? = context
        while (ctx is ContextWrapper) {
            if (ctx is Activity) return ctx
            ctx = ctx.baseContext
        }
        return null
    }

    /**
     * Automatically syncs Android status bar and navigation bar icon appearance (`Light` vs `Dark`)
     * whenever [IguanodonThemeMode] changes, eliminating the need for per-app `expect/actual` controllers.
     */
    fun syncSystemBarIconPolarity(darkTheme: Boolean) {
        val activity = findHostActivity() ?: return
        val window = activity.window ?: return
        val lightBars = !darkTheme
        if (Build.VERSION.SDK_INT >= 30) {
            val mask = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS or
                WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
            window.insetsController?.setSystemBarsAppearance(
                if (lightBars) mask else 0,
                mask,
            )
        } else {
            @Suppress("DEPRECATION")
            val decor = window.decorView
            @Suppress("DEPRECATION")
            var flags = decor.systemUiVisibility
            @Suppress("DEPRECATION")
            val mask = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR or View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
            flags = if (lightBars) (flags or mask) else (flags and mask.inv())
            @Suppress("DEPRECATION")
            decor.systemUiVisibility = flags
        }
    }

    private val gestureDetector = GestureDetector(
        context,
        object : GestureDetector.SimpleOnGestureListener() {
            override fun onLongPress(e: MotionEvent) {
                boostInteractive(300L)
                longPressTriggeredOnCurrentTouch = true
                session.onPointerDown(mapTouchX(e.x), mapTouchY(e.y), isRightClickOrLongPress = true)
            }

            override fun onScroll(
                e1: MotionEvent?,
                e2: MotionEvent,
                distanceX: Float,
                distanceY: Float,
            ): Boolean {
                boostInteractive(250L)
                val scaleY = session.viewportHeight.toFloat() / height.coerceAtLeast(1).toFloat()
                session.onScroll(mapTouchX(e2.x), mapTouchY(e2.y), distanceY * scaleY * 0.45f)
                return true
            }
        },
    )

    init {
        // Default Android to battery-friendly PowerSaving mode (lightweight backdrop + adaptive 10/30 FPS)
        session.setPowerMode(IguanodonPowerMode.PowerSaving)

        backingSurfaceView.holder.addCallback(this)
        addView(
            backingSurfaceView,
            LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT),
        )

        compositionHost.setContent(content)

        isFocusable = true
        isFocusableInTouchMode = true

        session.onThemeModeChanged = { mode ->
            post {
                syncSystemBarIconPolarity(darkTheme = (mode == IguanodonThemeMode.Dark))
            }
        }

        setOnApplyWindowInsetsListener { v, insets ->
            if (Build.VERSION.SDK_INT >= 30) {
                val mask = WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout()
                val sys = insets.getInsets(mask)
                rawInsetsPx[0] = sys.left
                rawInsetsPx[1] = sys.top
                rawInsetsPx[2] = sys.right
                rawInsetsPx[3] = sys.bottom
            } else {
                @Suppress("DEPRECATION")
                rawInsetsPx[0] = insets.systemWindowInsetLeft
                @Suppress("DEPRECATION")
                rawInsetsPx[1] = insets.systemWindowInsetTop
                @Suppress("DEPRECATION")
                rawInsetsPx[2] = insets.systemWindowInsetRight
                @Suppress("DEPRECATION")
                rawInsetsPx[3] = insets.systemWindowInsetBottom
            }
            applyScaledSafeInsets()
            v.onApplyWindowInsets(insets)
        }
    }

    @Composable
    fun BindSharedComposableContent() {
        CompositionLocalProvider(LocalIguanodonSession provides session) {
            content()
        }
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        syncSystemBarIconPolarity(darkTheme = (session.currentThemeMode == IguanodonThemeMode.Dark))
    }

    private fun computeLogicalViewport(rawWidthPx: Int, rawHeightPx: Int): Pair<Int, Int> {
        val safeW = rawWidthPx.coerceAtLeast(320).toDouble()
        val safeH = rawHeightPx.coerceAtLeast(240).toDouble()
        val density = resources.displayMetrics.density.coerceIn(1.0f, 4.0f).toDouble()
        val swDp = resources.configuration.smallestScreenWidthDp
        // Use real Android dp density (with a subtle 0.92x UI density factor so 16.5px glyphs read at a crisp ~15sp)
        // while strictly preserving 1:1 physical pixel aspect ratio across phones, foldables, tablets, & Chromebooks!
        val uiDivisor = if (swDp >= 600) (density * 0.94).coerceAtLeast(1.0) else (density * 0.88).coerceAtLeast(1.0)
        var scale = 1.0 / uiDivisor
        if (swDp < 600 && safeW < safeH && safeW * scale < 460.0) {
            scale = 460.0 / safeW
        }
        val maxPixels = 1_850_000.0
        val projected = (safeW * scale) * (safeH * scale)
        if (projected > maxPixels) {
            scale *= kotlin.math.sqrt(maxPixels / projected)
        }
        val w = (kotlin.math.round(safeW * scale).toInt() and -2).coerceIn(360, 2400)
        val h = (kotlin.math.round(safeH * scale).toInt() and -2).coerceIn(280, 2400)
        return w to h
    }

    private fun updateViewportAndBuffers(rawWidthPx: Int, rawHeightPx: Int): Boolean {
        if (rawWidthPx <= 0 || rawHeightPx <= 0) return false
        val (logicalW, logicalH) = computeLogicalViewport(rawWidthPx, rawHeightPx)
        val changed = session.viewportWidth != logicalW || session.viewportHeight != logicalH
        if (changed) {
            session.resize(logicalW, logicalH)
            boostInteractive(300L)
        }
        if (appliedSurfaceW != logicalW || appliedSurfaceH != logicalH) {
            appliedSurfaceW = logicalW
            appliedSurfaceH = logicalH
            backingSurfaceView.holder.setFixedSize(logicalW, logicalH)
        }
        applyScaledSafeInsets()
        return changed
    }

    private fun ensureBuffersMatchViewport() {
        val w = session.viewportWidth.coerceAtLeast(1)
        val h = session.viewportHeight.coerceAtLeast(1)
        val neededBytes = w * h * 4
        if (rgbaBytes.size != neededBytes) {
            rgbaBytes = ByteArray(neededBytes)
        }
        if (frameBitmap.width != w || frameBitmap.height != h) {
            frameBitmap = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        }
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        updateViewportAndBuffers(w, h)
    }

    override fun onWindowVisibilityChanged(visibility: Int) {
        super.onWindowVisibilityChanged(visibility)
        windowVisible = (visibility == View.VISIBLE)
        if (windowVisible && running) {
            boostInteractive(250L)
            Choreographer.getInstance().removeFrameCallback(this)
            Choreographer.getInstance().postFrameCallback(this)
        }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        running = true
        lastFrameNanos = monoNanos()
        val viewW = this.width.takeIf { it > 0 } ?: holder.surfaceFrame.width()
        val viewH = this.height.takeIf { it > 0 } ?: holder.surfaceFrame.height()
        if (viewW > 0 && viewH > 0) {
            updateViewportAndBuffers(viewW, viewH)
        }
        session.attachAndroidSurface(
            holder.surface,
            session.viewportWidth,
            session.viewportHeight,
        )
        requestApplyInsets()
        syncSystemBarIconPolarity(darkTheme = (session.currentThemeMode == IguanodonThemeMode.Dark))
        boostInteractive(350L)
        Choreographer.getInstance().removeFrameCallback(this)
        Choreographer.getInstance().postFrameCallback(this)
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        // Always derive logical viewport from the parent View's physical screen pixels, not the scaled Surface buffer!
        val viewW = this.width.takeIf { it > 0 } ?: width
        val viewH = this.height.takeIf { it > 0 } ?: height
        updateViewportAndBuffers(viewW, viewH)
        session.attachAndroidSurface(
            holder.surface,
            session.viewportWidth,
            session.viewportHeight,
        )
        boostInteractive(250L)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        running = false
        Choreographer.getInstance().removeFrameCallback(this)
        session.attachAndroidSurface(null, 0, 0)
    }

    override fun doFrame(frameTimeNanos: Long) {
        if (!running || !windowVisible) return
        val nowMono = monoNanos()
        val elapsedNanos = frameTimeNanos - lastFrameNanos
        val adaptiveInterval = if (isTouchDown || nowMono < interactiveUntilNanos) {
            interactiveIntervalNanos
        } else {
            ambientIntervalNanos
        }
        val minIntervalNanos = maxOf(adaptiveInterval, session.targetFrameIntervalNanos)
        // Subtract a 1.5ms vsync jitter tolerance so 60Hz/90Hz/120Hz Choreographer ticks don't skip an extra cycle
        if (!needsImmediateFrame && minIntervalNanos > 0L && elapsedNanos in 1L until (minIntervalNanos - 1_500_000L)) {
            Choreographer.getInstance().postFrameCallback(this)
            return
        }
        needsImmediateFrame = false
        val dt = if (lastFrameNanos == 0L) 0.016f else (elapsedNanos / 1_000_000_000.0f).coerceIn(1f / 1000f, 0.15f)
        lastFrameNanos = frameTimeNanos

        if (width > 0 && height > 0) {
            updateViewportAndBuffers(width, height)
        }

        compositionHost.doFrame(frameTimeNanos)

        if (session.hasVulkanSwapchain) {
            session.renderAndPresent(dt)
        } else {
            ensureBuffersMatchViewport()
            session.renderFrame(dt, rgbaBytes)

            val holder = backingSurfaceView.holder
            val canvas = holder.lockCanvas()
            if (canvas != null) {
                try {
                    frameBitmap.copyPixelsFromBuffer(ByteBuffer.wrap(rgbaBytes))
                    val dstRect = Rect(0, 0, canvas.width, canvas.height)
                    canvas.drawBitmap(frameBitmap, null, dstRect, null)
                } finally {
                    holder.unlockCanvasAndPost(canvas)
                }
            }
        }

        Choreographer.getInstance().postFrameCallback(this)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        gestureDetector.onTouchEvent(event)
        val x = mapTouchX(event.x)
        val y = mapTouchY(event.y)
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                isTouchDown = true
                boostInteractive(350L)
                requestFocus()
                longPressTriggeredOnCurrentTouch = false
                session.onPointerMove(x, y)
                session.onPointerDown(x, y, isRightClickOrLongPress = false)
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                boostInteractive(200L)
                session.onPointerMove(x, y)
                return true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                isTouchDown = false
                boostInteractive(300L)
                if (!longPressTriggeredOnCurrentTouch) {
                    session.onPointerUp(x, y)
                }
                longPressTriggeredOnCurrentTouch = false
                return true
            }
        }
        return super.onTouchEvent(event)
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        if (keyCode == KeyEvent.KEYCODE_DEL) {
            boostInteractive(250L)
            session.onBackspace()
            return true
        }
        val unicode = event.unicodeChar
        if (unicode in 32..126) {
            boostInteractive(250L)
            session.onKeyChar(unicode.toChar())
            return true
        }
        return super.onKeyDown(keyCode, event)
    }

    fun releaseEngine() {
        running = false
        Choreographer.getInstance().removeFrameCallback(this)
        compositionHost.dispose()
        session.close()
    }
}
