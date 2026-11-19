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
 * Drives `Recomposer` + `BroadcastFrameClock` + `IguanodonApplier` directly in the browser via
 * `window.requestAnimationFrame` and blits `iguanodon_framebuffer_ptr` into `<canvas>` with zero copy.
 */
class IguanodonWebCanvasHost(
    canvasElementId: String = "iguanodon-canvas",
    private val content: @Composable () -> Unit,
) {
    val canvas: HTMLCanvasElement = (document.getElementById(canvasElementId) as? HTMLCanvasElement)
        ?: error("Missing <canvas id=\"$canvasElementId\">")
    private val ctx: CanvasRenderingContext2D = canvas.getContext("2d") as CanvasRenderingContext2D

    val session: IguanodonSession
    val compositionHost: IguanodonCompositionHost

    private var lastFrameMs: Double = 0.0
    private var lastActivityMs: Double = window.performance.now()
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
        val safeW = winW.coerceAtLeast(320)
        val safeH = winH.coerceAtLeast(320)
        val aspect = safeW.toFloat() / safeH.toFloat()
        return if (safeW < safeH) {
            val w = 640
            val h = (w / aspect).toInt().coerceIn(1080, 1560)
            w to h
        } else {
            val h = 880
            val w = (h * aspect).toInt().coerceIn(1120, 1680)
            w to h
        }
    }

    private fun mapClientCoords(clientX: Double, clientY: Double): Pair<Float, Float> {
        val rect = canvas.getBoundingClientRect()
        val rw = rect.width.coerceAtLeast(1.0)
        val rh = rect.height.coerceAtLeast(1.0)
        val x = ((clientX - rect.left) / rw * session.viewportWidth).toFloat()
        val y = ((clientY - rect.top) / rh * session.viewportHeight).toFloat()
        return x to y
    }

    private fun markActive() {
        lastActivityMs = window.performance.now()
    }

    private fun installDomListeners() {
        window.addEventListener("resize", {
            markActive()
            val (nw, nh) = computeLogicalViewport(window.innerWidth, window.innerHeight)
            if (nw != session.viewportWidth || nh != session.viewportHeight) {
                canvas.width = nw
                canvas.height = nh
                session.resize(nw, nh)
            }
        })

        canvas.addEventListener("mousemove", { ev ->
            val e = ev as MouseEvent
            markActive()
            val (x, y) = mapClientCoords(e.clientX.toDouble(), e.clientY.toDouble())
            session.onPointerMove(x, y)
        })

        canvas.addEventListener("mousedown", { ev ->
            val e = ev as MouseEvent
            markActive()
            val (x, y) = mapClientCoords(e.clientX.toDouble(), e.clientY.toDouble())
            val isRight = e.button.toInt() == 2
            session.onPointerMove(x, y)
            session.onPointerDown(x, y, isRight)
        })

        canvas.addEventListener("mouseup", { ev ->
            val e = ev as MouseEvent
            markActive()
            if (e.button.toInt() == 0) {
                val (x, y) = mapClientCoords(e.clientX.toDouble(), e.clientY.toDouble())
                session.onPointerUp(x, y)
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
                markActive()
                val (x, y) = mapClientCoords(e.clientX.toDouble(), e.clientY.toDouble())
                session.onScroll(x, y, e.deltaY.toFloat() * 0.65f)
            },
            js("{ passive: false }"),
        )

        // Touch support for iPad / mobile browsers
        canvas.addEventListener(
            "touchstart",
            { ev ->
                ev.preventDefault()
                markActive()
                val touches = ev.asDynamic().touches
                if ((touches.length as Int) > 0) {
                    val t = touches[0]
                    val (x, y) = mapClientCoords((t.clientX as Number).toDouble(), (t.clientY as Number).toDouble())
                    session.onPointerMove(x, y)
                    session.onPointerDown(x, y, false)
                }
            },
            js("{ passive: false }"),
        )

        canvas.addEventListener(
            "touchmove",
            { ev ->
                ev.preventDefault()
                markActive()
                val touches = ev.asDynamic().touches
                if ((touches.length as Int) > 0) {
                    val t = touches[0]
                    val (x, y) = mapClientCoords((t.clientX as Number).toDouble(), (t.clientY as Number).toDouble())
                    session.onPointerMove(x, y)
                }
            },
            js("{ passive: false }"),
        )

        canvas.addEventListener(
            "touchend",
            { ev ->
                ev.preventDefault()
                markActive()
                val touches = ev.asDynamic().changedTouches
                if ((touches.length as Int) > 0) {
                    val t = touches[0]
                    val (x, y) = mapClientCoords((t.clientX as Number).toDouble(), (t.clientY as Number).toDouble())
                    session.onPointerUp(x, y)
                }
            },
            js("{ passive: false }"),
        )

        window.addEventListener("keydown", { ev ->
            val e = ev as KeyboardEvent
            // Ignore shortcuts like Ctrl+R / Cmd+R
            if (!e.ctrlKey && !e.metaKey && !e.altKey) {
                if (e.key == "Backspace") {
                    e.preventDefault()
                    markActive()
                    session.onBackspace()
                } else if (e.key.length == 1) {
                    val code = e.key[0].code
                    if (code in 32..126) {
                        markActive()
                        session.onKeyChar(e.key[0])
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
        val elapsedSinceLast = if (lastFrameMs == 0.0) 16.6 else (nowMs - lastFrameMs)
        val idleMs = nowMs - lastActivityMs
        // Run at full refresh rate during interaction or Compose updates; coast smoothly at ~15fps when idle
        if (idleMs > 3000.0 && elapsedSinceLast < 64.0 && session.encoder.isEmpty()) {
            return
        }
        val dtSec = (elapsedSinceLast / 1000.0).toFloat().coerceIn(0.001f, 0.1f)
        lastFrameMs = nowMs

        val nanos = (nowMs * 1_000_000.0).toLong()
        compositionHost.doFrame(nanos)

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
