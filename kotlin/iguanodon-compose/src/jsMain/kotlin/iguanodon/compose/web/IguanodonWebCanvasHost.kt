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

package iguanodon.compose.web

import androidx.compose.runtime.Composable
import iguanodon.compose.IguanodonCompositionHost
import iguanodon.compose.bridge.IguanodonNativeBridge
import iguanodon.compose.bridge.IguanodonSession
import kotlinx.browser.document
import kotlinx.browser.window
import kotlinx.coroutines.Dispatchers
import org.khronos.webgl.Uint8ClampedArray
import org.w3c.dom.CanvasRenderingContext2D
import org.w3c.dom.HTMLCanvasElement
import org.w3c.dom.ImageData
import org.w3c.dom.events.KeyboardEvent
import org.w3c.dom.events.MouseEvent
import org.w3c.dom.events.WheelEvent

/**
 * Pure `compose.runtime` HTML5 `<canvas>` + WebAssembly (`iguanodon_ffi.wasm`) host for Iguanodon.
 *
 * Tuned for ultra-low CPU usage in browsers:
 * - Caps ambient background shader animations to [ambientFps] (`10 FPS` by default).
 * - Pauses rendering completely (`0% CPU`) whenever the browser tab is hidden (`document.hidden`).
 * - Passive cursor hover does not boost FPS above [ambientFps]; only active drags, clicks, wheel
 *   scrolls, typing, or Compose state updates briefly step up to [interactiveFps] (`25 FPS`).
 */
class IguanodonWebCanvasHost(
    canvasElementId: String = "iguanodon-canvas",
    private val ambientFps: Int = 10,
    private val interactiveFps: Int = 25,
    private val content: @Composable () -> Unit,
) {
    val canvas: HTMLCanvasElement = (document.getElementById(canvasElementId) as? HTMLCanvasElement)
        ?: error("Missing <canvas id=\"$canvasElementId\">")
    private val ctx: CanvasRenderingContext2D = canvas.getContext("2d") as CanvasRenderingContext2D

    val session: IguanodonSession
    val compositionHost: IguanodonCompositionHost

    private val ambientIntervalMs: Double = 1000.0 / ambientFps.coerceIn(1, 60).toDouble()
    private val interactiveIntervalMs: Double = 1000.0 / interactiveFps.coerceIn(10, 60).toDouble()

    private var lastFrameMs: Double = 0.0
    private var interactiveUntilMs: Double = window.performance.now() + 300.0
    private var isPointerDown: Boolean = false
    private var lastTouchY: Float = 0f
    private var needsImmediateFrame: Boolean = true
    private var disposed: Boolean = false

    init {
        val (initW, initH) = computeLogicalViewport(window.innerWidth, window.innerHeight)
        canvas.width = initW
        canvas.height = initH

        session = IguanodonSession(initialWidth = initW.toFloat(), initialHeight = initH.toFloat())
        compositionHost = IguanodonCompositionHost(session = session, parentContext = Dispatchers.Unconfined)
        compositionHost.setContent(content)

        installDomListeners()
        scheduleNextFrame()
    }

    private fun computeLogicalViewport(winW: Int, winH: Int): Pair<Int, Int> {
        val safeW = winW.coerceAtLeast(320).toDouble()
        val safeH = winH.coerceAtLeast(240).toDouble()
        // Keep 1:1 CSS pixel aspect & font sizing across all window sizes so resizing the browser window
        // dynamically reflows and scrolls content instead of shrinking/inflating typography!
        // Only cap total pixels uniformly on >2.2M-pixel 4K monitors for low CPU usage.
        var scale = 1.0
        val maxPixels = 2_200_000.0
        val projectedPixels = safeW * safeH
        if (projectedPixels > maxPixels) {
            scale = kotlin.math.sqrt(maxPixels / projectedPixels)
        }
        val w = kotlin.math.round(safeW * scale).toInt().coerceIn(320, 2560)
        val h = kotlin.math.round(safeH * scale).toInt().coerceIn(240, 1600)
        return w to h
    }

    private fun mapClientCoords(clientX: Double, clientY: Double): Pair<Float, Float> {
        val rect = canvas.getBoundingClientRect()
        val rw = rect.width.coerceAtLeast(1.0)
        val rh = rect.height.coerceAtLeast(1.0)
        val x = ((clientX - rect.left) / rw * session.viewportWidth).toFloat()
        val y = ((clientY - rect.top) / rh * session.viewportHeight).toFloat()
        return x to y
    }

    private fun boostInteractive(durationMs: Double = 250.0) {
        interactiveUntilMs = window.performance.now() + durationMs
        needsImmediateFrame = true
    }

    private fun installDomListeners() {
        window.addEventListener("resize", {
            val (nw, nh) = computeLogicalViewport(window.innerWidth, window.innerHeight)
            if (nw != session.viewportWidth || nh != session.viewportHeight) {
                canvas.width = nw
                canvas.height = nh
                session.resize(nw, nh)
                boostInteractive(300.0)
            }
        })

        canvas.addEventListener("mousemove", { ev ->
            val e = ev as MouseEvent
            val (x, y) = mapClientCoords(e.clientX.toDouble(), e.clientY.toDouble())
            session.onPointerMove(x, y)
            // Only step up to interactive FPS when actively dragging (e.g., ROI box or scrubber);
            // passive hover updates ride the lightweight 10 FPS ambient tick.
            if (isPointerDown) {
                boostInteractive(150.0)
            }
        })

        canvas.addEventListener("mousedown", { ev ->
            val e = ev as MouseEvent
            isPointerDown = true
            val (x, y) = mapClientCoords(e.clientX.toDouble(), e.clientY.toDouble())
            val isRight = e.button.toInt() == 2
            session.onPointerMove(x, y)
            session.onPointerDown(x, y, isRight)
            boostInteractive(300.0)
        })

        window.addEventListener("mouseup", { ev ->
            val e = ev as MouseEvent
            if (isPointerDown) {
                isPointerDown = false
                if (e.button.toInt() == 0) {
                    val (x, y) = mapClientCoords(e.clientX.toDouble(), e.clientY.toDouble())
                    session.onPointerUp(x, y)
                }
                boostInteractive(250.0)
            }
        })

        canvas.addEventListener("contextmenu", { ev ->
            ev.preventDefault()
        })

        canvas.addEventListener(
            "wheel",
            { ev ->
                val e = ev as WheelEvent
                ev.preventDefault()
                val (x, y) = mapClientCoords(e.clientX.toDouble(), e.clientY.toDouble())
                session.onScroll(x, y, e.deltaY.toFloat() * 0.65f)
                boostInteractive(200.0)
            },
            js("{ passive: false }"),
        )

        canvas.addEventListener(
            "touchstart",
            { ev ->
                ev.preventDefault()
                isPointerDown = true
                val touches = ev.asDynamic().touches
                if ((touches.length as Int) > 0) {
                    val t = touches[0]
                    val (x, y) = mapClientCoords((t.clientX as Number).toDouble(), (t.clientY as Number).toDouble())
                    lastTouchY = y
                    session.onPointerMove(x, y)
                    session.onPointerDown(x, y, false)
                    boostInteractive(300.0)
                }
            },
            js("{ passive: false }"),
        )

        canvas.addEventListener(
            "touchmove",
            { ev ->
                ev.preventDefault()
                val touches = ev.asDynamic().touches
                if ((touches.length as Int) > 0) {
                    val t = touches[0]
                    val (x, y) = mapClientCoords((t.clientX as Number).toDouble(), (t.clientY as Number).toDouble())
                    val dy = lastTouchY - y
                    lastTouchY = y
                    session.onPointerMove(x, y)
                    if (kotlin.math.abs(dy) > 0.5f) {
                        session.onScroll(x, y, dy)
                    }
                    boostInteractive(150.0)
                }
            },
            js("{ passive: false }"),
        )

        canvas.addEventListener(
            "touchend",
            { ev ->
                ev.preventDefault()
                isPointerDown = false
                val touches = ev.asDynamic().changedTouches
                if ((touches.length as Int) > 0) {
                    val t = touches[0]
                    val (x, y) = mapClientCoords((t.clientX as Number).toDouble(), (t.clientY as Number).toDouble())
                    session.onPointerUp(x, y)
                    boostInteractive(250.0)
                }
            },
            js("{ passive: false }"),
        )

        window.addEventListener("keydown", { ev ->
            val e = ev as KeyboardEvent
            if (!e.ctrlKey && !e.metaKey && !e.altKey) {
                when (e.key) {
                    "Backspace" -> {
                        e.preventDefault()
                        session.onBackspace()
                        boostInteractive(250.0)
                    }
                    "PageDown" -> {
                        e.preventDefault()
                        session.onScroll(session.viewportWidth * 0.5f, session.viewportHeight * 0.5f, 260f)
                        boostInteractive(250.0)
                    }
                    "PageUp" -> {
                        e.preventDefault()
                        session.onScroll(session.viewportWidth * 0.5f, session.viewportHeight * 0.5f, -260f)
                        boostInteractive(250.0)
                    }
                    "ArrowDown" -> {
                        e.preventDefault()
                        session.onScroll(session.viewportWidth * 0.5f, session.viewportHeight * 0.5f, 56f)
                        boostInteractive(200.0)
                    }
                    "ArrowUp" -> {
                        e.preventDefault()
                        session.onScroll(session.viewportWidth * 0.5f, session.viewportHeight * 0.5f, -56f)
                        boostInteractive(200.0)
                    }
                    else -> if (e.key.length == 1) {
                        val code = e.key[0].code
                        if (code in 32..126) {
                            session.onKeyChar(e.key[0])
                            boostInteractive(250.0)
                        }
                    }
                }
            }
        })
    }

    private fun scheduleNextFrame() {
        if (disposed) return
        window.requestAnimationFrame { nowMs ->
            if (!disposed) {
                onAnimationFrame(nowMs)
                scheduleNextFrame()
            }
        }
    }

    private fun onAnimationFrame(nowMs: Double) {
        // 0% CPU when browser tab is in the background or window is minimized
        if (document.asDynamic().hidden == true) {
            return
        }

        val elapsedMs = if (lastFrameMs == 0.0) 999.0 else (nowMs - lastFrameMs)
        val minInterval = if (isPointerDown || nowMs < interactiveUntilMs) {
            interactiveIntervalMs
        } else {
            ambientIntervalMs
        }

        if (!needsImmediateFrame && elapsedMs < minInterval) {
            return
        }
        needsImmediateFrame = false

        val nanos = (nowMs * 1_000_000.0).toLong()
        compositionHost.doFrame(nanos)

        val dtSec = (elapsedMs / 1000.0).toFloat().coerceIn(0.001f, 0.25f)
        lastFrameMs = nowMs

        val w = session.viewportWidth
        val h = session.viewportHeight
        if (canvas.width != w || canvas.height != h) {
            canvas.width = w
            canvas.height = h
        }

        session.renderAndPresent(dtSec)
        val fbPtr = IguanodonNativeBridge.framebufferPtr(session.nativeHandle)
        val memBuf = IguanodonNativeBridge.wasmMemoryBuffer()
        if (fbPtr != 0 && memBuf != null) {
            val view = Uint8ClampedArray(memBuf, fbPtr, w * h * 4)
            val imageData = ImageData(view, w, h)
            ctx.putImageData(imageData, 0.0, 0.0)
        }
    }

    fun dispose() {
        disposed = true
        compositionHost.dispose()
        session.close()
    }
}
