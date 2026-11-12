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

package iguanodon.compose.desktop

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import iguanodon.compose.IguanodonCompositionHost
import iguanodon.compose.LocalIguanodonSession
import iguanodon.compose.bridge.IguanodonSession
import kotlinx.coroutines.Dispatchers
import java.awt.Dimension
import java.awt.Graphics
import java.awt.event.KeyAdapter
import java.awt.event.KeyEvent
import java.awt.event.MouseAdapter
import java.awt.event.MouseEvent
import java.awt.event.MouseMotionAdapter
import java.awt.event.WindowAdapter
import java.awt.event.WindowEvent
import java.awt.image.BufferedImage
import javax.swing.JFrame
import javax.swing.JPanel
import javax.swing.SwingUtilities
import javax.swing.Timer

/**
 * Pure `compose.runtime` Desktop JVM window host for Iguanodon's Rust/Vulkan engine (`iguanodon-vk`).
 *
 * Drives `Recomposer` + `BroadcastFrameClock` + `IguanodonApplier` directly against `libiguanodon_ffi.so`
 * without `compose-ui`, `ComposeWindow`, or Skiko.
 */
class IguanodonDesktopWindow(
    title: String = "Iguanodon Vulkan UI Toolkit — Standard Benchmark App (Kotlin Multiplatform)",
    private val content: @Composable () -> Unit,
) : JPanel() {

    val session = IguanodonSession(initialWidth = 1180f, initialHeight = 760f)
    val compositionHost = IguanodonCompositionHost(session = session, parentContext = Dispatchers.Unconfined)
    private var rgbaBytes = ByteArray(1180 * 760 * 4)
    private var bufferedImage = BufferedImage(1180, 760, BufferedImage.TYPE_INT_ARGB)
    private var argbInts = IntArray(1180 * 760)
    private var lastFrameNanos = System.nanoTime()
    private var lastInputNanos = System.nanoTime()

    init {
        preferredSize = Dimension(1180, 760)
        isFocusable = true

        compositionHost.setContent(content)

        addMouseListener(object : MouseAdapter() {
            override fun mousePressed(e: MouseEvent) {
                lastInputNanos = System.nanoTime()
                requestFocusInWindow()
                val isRight = SwingUtilities.isRightMouseButton(e)
                session.onPointerDown(e.x.toFloat(), e.y.toFloat(), isRight)
            }

            override fun mouseReleased(e: MouseEvent) {
                lastInputNanos = System.nanoTime()
                if (SwingUtilities.isLeftMouseButton(e)) {
                    session.onPointerUp(e.x.toFloat(), e.y.toFloat())
                }
            }
        })

        addMouseMotionListener(object : MouseMotionAdapter() {
            override fun mouseMoved(e: MouseEvent) {
                lastInputNanos = System.nanoTime()
                session.onPointerMove(e.x.toFloat(), e.y.toFloat())
            }

            override fun mouseDragged(e: MouseEvent) {
                lastInputNanos = System.nanoTime()
                session.onPointerMove(e.x.toFloat(), e.y.toFloat())
            }
        })

        addMouseWheelListener { e ->
            lastInputNanos = System.nanoTime()
            session.onScroll(e.x.toFloat(), e.y.toFloat(), (e.preciseWheelRotation * 48.0).toFloat())
        }

        addKeyListener(object : KeyAdapter() {
            override fun keyPressed(e: KeyEvent) {
                lastInputNanos = System.nanoTime()
                if (e.keyCode == KeyEvent.VK_BACK_SPACE) {
                    session.onBackspace()
                } else if (e.keyChar.code in 32..126) {
                    session.onKeyChar(e.keyChar)
                }
            }
        })

        var frameTimer: Timer? = null
        frameTimer = Timer(8) {
            val now = System.nanoTime()
            val targetDelay = session.targetFrameIntervalMs
            if (frameTimer?.delay != targetDelay) {
                frameTimer?.delay = targetDelay
            }
            if (width > 0 && height > 0 && (width != session.viewportWidth || height != session.viewportHeight)) {
                lastInputNanos = now
                session.resize(width, height)
                val w = session.viewportWidth
                val h = session.viewportHeight
                rgbaBytes = ByteArray(w * h * 4)
                argbInts = IntArray(w * h)
                bufferedImage = BufferedImage(w, h, BufferedImage.TYPE_INT_ARGB)
            }
            val dt = ((now - lastFrameNanos) / 1_000_000_000.0f).coerceIn(1f / 1000f, 0.15f)
            lastFrameNanos = now
            compositionHost.doFrame(now)
            session.renderFrame(dt, rgbaBytes)
            repaint()
        }
        frameTimer.start()

        val frame = JFrame(title)
        frame.defaultCloseOperation = JFrame.EXIT_ON_CLOSE
        frame.addWindowListener(object : WindowAdapter() {
            override fun windowClosing(e: WindowEvent?) {
                frameTimer.stop()
                compositionHost.dispose()
                session.close()
            }
        })
        frame.contentPane.add(this)
        frame.pack()
        frame.setLocationRelativeTo(null)
        frame.isVisible = true
    }

    @Composable
    fun BindSharedComposableContent() {
        CompositionLocalProvider(LocalIguanodonSession provides session) {
            content()
        }
    }

    override fun paintComponent(g: Graphics) {
        super.paintComponent(g)
        val w = session.viewportWidth
        val h = session.viewportHeight
        var srcIdx = 0
        for (i in 0 until (w * h)) {
            val r = rgbaBytes[srcIdx].toInt() and 0xFF
            val gr = rgbaBytes[srcIdx + 1].toInt() and 0xFF
            val b = rgbaBytes[srcIdx + 2].toInt() and 0xFF
            argbInts[i] = (0xFF shl 24) or (r shl 16) or (gr shl 8) or b
            srcIdx += 4
        }
        bufferedImage.setRGB(0, 0, w, h, argbInts, 0, w)
        g.drawImage(bufferedImage, 0, 0, width, height, null)
    }
}
