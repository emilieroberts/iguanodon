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

package iguanodon.compose.bridge

import iguanodon.compose.ByteBufferCommandEncoder
import iguanodon.compose.IguanodonEventMask
import iguanodon.compose.IguanodonFrameRateLimit
import iguanodon.compose.IguanodonModifier
import iguanodon.compose.IguanodonNode
import iguanodon.compose.IguanodonPowerMode
import iguanodon.compose.IguanodonRendererBackend
import iguanodon.compose.IguanodonThemeMode
import iguanodon.compose.ReverseEventTypes
import java.io.File
import java.io.FileOutputStream

/**
 * Shared JNI / C-ABI bridge to `libiguanodon_ffi.so` (`crates/iguanodon-ffi/src/lib.rs`).
 * Identical native entrypoints (`Java_iguanodon_compose_bridge_IguanodonNativeBridge_*`)
 * are used on both Android (`arm64-v8a` / `x86_64`) and Desktop JVM (`Linux` / `Windows` / `macOS`).
 */
object IguanodonNativeBridge {
    var isNativeLoaded: Boolean = false
        private set

    init {
        try {
            System.loadLibrary("iguanodon_ffi")
            isNativeLoaded = true
        } catch (_: UnsatisfiedLinkError) {
            try {
                val stream = IguanodonNativeBridge::class.java.getResourceAsStream(
                    "/META-INF/native/linux-x86_64/libiguanodon_ffi.so",
                )
                if (stream != null) {
                    val tmp = File.createTempFile("libiguanodon_ffi_", ".so")
                    tmp.setReadable(false, false)
                    tmp.setReadable(true, true)
                    tmp.setWritable(false, false)
                    tmp.setWritable(true, true)
                    tmp.setExecutable(false, false)
                    tmp.setExecutable(true, true)
                    tmp.deleteOnExit()
                    FileOutputStream(tmp).use { out -> stream.copyTo(out) }
                    System.load(tmp.absolutePath)
                    isNativeLoaded = true
                }
            } catch (_: Throwable) {
                // Fallback when running without native shared library on classpath
            }
        }
    }

    @JvmStatic
    external fun nativeCreate(width: Float, height: Float): Long

    @JvmStatic
    external fun nativeDestroy(handle: Long)

    @JvmStatic
    external fun nativeResize(handle: Long, width: Float, height: Float)

    @JvmStatic
    external fun nativeSetSafeInsets(
        handle: Long,
        left: Float,
        top: Float,
        right: Float,
        bottom: Float,
    )

    @JvmStatic
    external fun nativeFlushCommands(handle: Long, batch: ByteArray): Int

    @JvmStatic
    external fun nativeFlushDirectBuffer(handle: Long, directBufPtr: Long, byteLen: Int): Int

    @JvmStatic
    external fun nativePollEvents(handle: Long, outBuffer: ByteArray): Int

    @JvmStatic
    external fun nativePollEventsDirect(handle: Long, directBufPtr: Long, capacity: Int): Int

    @JvmStatic
    external fun nativePointerMove(handle: Long, x: Float, y: Float)

    @JvmStatic
    external fun nativePointerDown(handle: Long, x: Float, y: Float, isRightOrLongPress: Byte)

    @JvmStatic
    external fun nativePointerUp(handle: Long, x: Float, y: Float)

    @JvmStatic
    external fun nativeScroll(handle: Long, x: Float, y: Float, deltaY: Float)

    @JvmStatic
    external fun nativeKeyChar(handle: Long, codepoint: Int)

    @JvmStatic
    external fun nativeBackspace(handle: Long)

    @JvmStatic
    external fun nativeSetAndroidSurface(
        handle: Long,
        surface: Any?,
        width: Int,
        height: Int,
    ): Boolean

    @JvmStatic
    external fun nativeRenderAndPresent(
        handle: Long,
        dtSec: Float,
        width: Int,
        height: Int,
    ): Float

    @JvmStatic
    external fun nativeRenderFrameByteArray(
        handle: Long,
        dtSec: Float,
        width: Int,
        height: Int,
        outRgba: ByteArray,
    ): Float

    @JvmStatic
    external fun nativeRenderFrameRgba(
        handle: Long,
        dtSec: Float,
        width: Int,
        height: Int,
        directRgbaPtr: Long,
    ): Float
}

/**
 * Platform-agnostic session controller shared between Android (`IguanodonVulkanSurfaceView`)
 * and Desktop (`IguanodonDesktopWindow`).
 *
 * Collects binary opcodes emitted by `@Composable` functions and `IguanodonApplier` into [encoder],
 * flushes them in a single zero-copy batch per dirty frame, and drains reverse input/window events
 * from `libiguanodon_ffi.so` via [pollEvents].
 */
class IguanodonSession(
    initialWidth: Float = 1180f,
    initialHeight: Float = 760f,
) : AutoCloseable {
    var nativeHandle: Long = if (IguanodonNativeBridge.isNativeLoaded) {
        IguanodonNativeBridge.nativeCreate(initialWidth, initialHeight)
    } else {
        0L
    }
        private set

    val encoder = ByteBufferCommandEncoder()
    private val nodeRegistry = HashMap<Long, IguanodonNode>()
    private val eventPollBuffer = ByteArray(ReverseEventTypes.RECORD_BYTE_SIZE * 64)
    private var nextDynamicNodeId: Long = 1000L

    var viewportWidth: Int = initialWidth.toInt()
        private set
    var viewportHeight: Int = initialHeight.toInt()
        private set
    var safeInsetLeftDp: Float = 0f
        private set
    var safeInsetTopDp: Float = 0f
        private set
    var safeInsetRightDp: Float = 0f
        private set
    var safeInsetBottomDp: Float = 0f
        private set
    var latestFrameMs: Float = 0f
        private set
    var currentThemeMode: IguanodonThemeMode = IguanodonThemeMode.Dark
        private set
    var powerMode: IguanodonPowerMode = IguanodonPowerMode.Performance
        private set
    var frameRateLimit: IguanodonFrameRateLimit = IguanodonFrameRateLimit.Fps120
        private set
    var rendererBackend: IguanodonRendererBackend = IguanodonRendererBackend.Iguanodon
        private set
    var onThemeModeChanged: ((IguanodonThemeMode) -> Unit)? = null
    var onPowerModeChanged: ((IguanodonPowerMode) -> Unit)? = null
    var onFrameRateLimitChanged: ((IguanodonFrameRateLimit) -> Unit)? = null
    var onRendererBackendChanged: ((IguanodonRendererBackend) -> Unit)? = null

    val isPortrait: Boolean
        get() = viewportWidth < viewportHeight

    val effectiveFrameRateLimit: IguanodonFrameRateLimit
        get() = frameRateLimit.effectiveForPowerMode(powerMode)

    val targetFps: Int
        get() = effectiveFrameRateLimit.targetFps

    val targetFrameIntervalMs: Int
        get() = effectiveFrameRateLimit.frameIntervalMs.toInt()

    val targetFrameIntervalNanos: Long
        get() = effectiveFrameRateLimit.frameIntervalNanos

    fun allocateNodeId(): Long = nextDynamicNodeId++

    fun registerNode(node: IguanodonNode) {
        nodeRegistry[node.nodeId] = node
    }

    fun unregisterSubtree(node: IguanodonNode) {
        nodeRegistry.remove(node.nodeId)
        for (c in node.children) {
            unregisterSubtree(c)
        }
    }

    fun syncModifierToNode(
        node: IguanodonNode,
        modifier: IguanodonModifier,
        flexDir: Byte = 0,
        align: Byte = 0,
        justify: Byte = 0,
        gapDp: Float = 0f,
    ) {
        node.modifierChain = modifier
        node.onClick = modifier.onClick ?: node.onClick
        node.onLongClick = modifier.onLongClick ?: node.onLongClick
        node.onHoverChanged = modifier.onHoverChanged ?: node.onHoverChanged
        node.onDragDelta = modifier.onDragDelta ?: node.onDragDelta

        encoder.encodeSetLayoutStyle(
            nodeId = node.nodeId,
            flexDir = flexDir,
            align = align,
            justify = justify,
            padL = modifier.padLeftDp,
            padT = modifier.padTopDp,
            padR = modifier.padRightDp,
            padB = modifier.padBottomDp,
            gap = gapDp,
            width = modifier.widthDp,
            height = modifier.heightDp,
            minW = modifier.minWidthDp,
            maxW = modifier.maxWidthDp,
            weight = modifier.weight,
        )

        if (modifier.bgRgba != 0 || modifier.borderW > 0f || modifier.cornerRadiusDp > 0f || modifier.clip) {
            encoder.encodeSetVisualStyle(
                nodeId = node.nodeId,
                bgRgba = modifier.bgRgba,
                borderRgba = modifier.borderRgba,
                borderW = modifier.borderW,
                cornerRadius = modifier.cornerRadiusDp,
                clip = if (modifier.clip) 1 else 0,
            )
        }

        if (modifier.shaderId != 0.toByte()) {
            encoder.encodeSetShaderEffect(
                nodeId = node.nodeId,
                shaderId = modifier.shaderId,
                p0 = modifier.shaderP0,
                p1 = modifier.shaderP1,
                p2 = modifier.shaderP2,
                p3 = modifier.shaderP3,
            )
        }

        if (modifier.scrollState != null) {
            encoder.encodeSetScrollState(
                nodeId = node.nodeId,
                axis = modifier.scrollAxis,
                offsetPx = modifier.scrollState.value.toFloat(),
            )
        }

        var mask = 0
        if (node.onClick != null) mask = mask or IguanodonEventMask.CLICK
        if (node.onLongClick != null) mask = mask or IguanodonEventMask.LONG_CLICK
        if (node.onHoverChanged != null) mask = mask or IguanodonEventMask.HOVER
        if (node.onDragDelta != null) mask = mask or IguanodonEventMask.DRAG
        if (node.onTextChange != null) mask = mask or IguanodonEventMask.TEXT_INPUT or IguanodonEventMask.KEY
        if (modifier.scrollState != null || node.onScrollDelta != null) mask = mask or IguanodonEventMask.SCROLL
        if (mask != 0) {
            encoder.encodeSetEventMask(node.nodeId, mask)
        }
    }

    fun notifyThemeModeChanged(mode: IguanodonThemeMode) {
        if (currentThemeMode != mode) {
            currentThemeMode = mode
        }
        onThemeModeChanged?.invoke(mode)
    }

    fun setPowerMode(mode: IguanodonPowerMode) {
        powerMode = mode
        encoder.encodeSetPowerMode(mode)
        flushPendingCommands()
        onPowerModeChanged?.invoke(mode)
    }

    fun togglePowerMode(): IguanodonPowerMode {
        val next = powerMode.toggle()
        setPowerMode(next)
        return next
    }

    fun setFrameRateLimit(limit: IguanodonFrameRateLimit) {
        frameRateLimit = limit
        encoder.encodeSetFrameRateLimit(limit)
        flushPendingCommands()
        onFrameRateLimitChanged?.invoke(limit)
    }

    fun setRendererBackend(backend: IguanodonRendererBackend) {
        rendererBackend = backend
        encoder.encodeSetRendererBackend(backend)
        flushPendingCommands()
        onRendererBackendChanged?.invoke(backend)
    }

    fun toggleRendererBackend(): IguanodonRendererBackend {
        val next = rendererBackend.toggle()
        setRendererBackend(next)
        return next
    }

    fun resize(width: Int, height: Int) {
        val clampedW = width.coerceIn(240, 3840)
        val clampedH = height.coerceIn(240, 3840)
        if (clampedW != viewportWidth || clampedH != viewportHeight) {
            viewportWidth = clampedW
            viewportHeight = clampedH
            if (nativeHandle != 0L) {
                IguanodonNativeBridge.nativeResize(
                    nativeHandle,
                    clampedW.toFloat(),
                    clampedH.toFloat(),
                )
            }
        }
    }

    fun setSafeInsets(left: Float, top: Float, right: Float, bottom: Float) {
        safeInsetLeftDp = left
        safeInsetTopDp = top
        safeInsetRightDp = right
        safeInsetBottomDp = bottom
        if (nativeHandle != 0L) {
            IguanodonNativeBridge.nativeSetSafeInsets(nativeHandle, left, top, right, bottom)
        }
    }

    fun flushByteArray(batch: ByteArray): Int {
        if (nativeHandle == 0L || batch.isEmpty()) return 0
        return IguanodonNativeBridge.nativeFlushCommands(nativeHandle, batch)
    }

    fun flushPendingCommands(): Int {
        if (encoder.isEmpty()) return 0
        val batch = encoder.toByteArrayAndClear()
        return flushByteArray(batch)
    }

    /**
     * Drains reverse input and viewport events queued by `libiguanodon_ffi.so` and dispatches
     * them to the registered [IguanodonNode] callbacks on the UI thread.
     */
    fun pollEvents(): Int {
        if (nativeHandle == 0L) return 0
        val bytesRead = IguanodonNativeBridge.nativePollEvents(nativeHandle, eventPollBuffer)
        if (bytesRead <= 0) return 0
        var count = 0
        var offset = 0
        while (offset + ReverseEventTypes.RECORD_BYTE_SIZE <= bytesRead) {
            val evType = eventPollBuffer[offset].toInt() and 0xFF
            val auxU32 = readIntLe(eventPollBuffer, offset + 4)
            val nodeId = readLongLe(eventPollBuffer, offset + 8)
            val f0 = Float.fromBits(readIntLe(eventPollBuffer, offset + 16))
            val f1 = Float.fromBits(readIntLe(eventPollBuffer, offset + 20))
            val f2 = Float.fromBits(readIntLe(eventPollBuffer, offset + 24))
            val f3 = Float.fromBits(readIntLe(eventPollBuffer, offset + 28))

            when (evType) {
                ReverseEventTypes.CLICK -> {
                    nodeRegistry[nodeId]?.onClick?.invoke()
                }
                ReverseEventTypes.LONG_OR_SECONDARY_CLICK -> {
                    val node = nodeRegistry[nodeId]
                    if (node?.onLongClick != null) {
                        node.onLongClick?.invoke()
                    } else {
                        node?.onClick?.invoke()
                    }
                }
                ReverseEventTypes.HOVER_ENTER -> {
                    nodeRegistry[nodeId]?.onHoverChanged?.invoke(true)
                }
                ReverseEventTypes.HOVER_EXIT -> {
                    nodeRegistry[nodeId]?.onHoverChanged?.invoke(false)
                }
                ReverseEventTypes.DRAG_DELTA -> {
                    nodeRegistry[nodeId]?.onDragDelta?.invoke(f0, f1)
                }
                ReverseEventTypes.SCROLL_DELTA -> {
                    val node = nodeRegistry[nodeId]
                    node?.onScrollDelta?.invoke(f0, f1)
                    node?.modifierChain?.scrollState?.dispatchRawDelta(f1)
                }
                ReverseEventTypes.KEY_CHAR -> {
                    val node = nodeRegistry[nodeId]
                    if (node?.onTextChange != null && auxU32 > 0) {
                        val next = node.textValue + auxU32.toChar()
                        node.textValue = next
                        node.onTextChange?.invoke(next)
                    }
                }
                ReverseEventTypes.KEY_BACKSPACE -> {
                    val node = nodeRegistry[nodeId]
                    if (node?.onTextChange != null && node.textValue.isNotEmpty()) {
                        val next = node.textValue.dropLast(1)
                        node.textValue = next
                        node.onTextChange?.invoke(next)
                    }
                }
                ReverseEventTypes.WINDOW_RESIZE -> {
                    // Viewport size is owned by the host surface (`session.resize(w, h)`)
                    // so a queued reverse resize event never desynchronizes `viewportWidth`/`viewportHeight`
                    // from the allocated `frameBitmap` and `rgbaBytes` mid-frame.
                }
                ReverseEventTypes.SAFE_INSETS -> {
                    safeInsetLeftDp = f0
                    safeInsetTopDp = f1
                    safeInsetRightDp = f2
                    safeInsetBottomDp = f3
                }
                ReverseEventTypes.THEME_MODE_CHANGED -> {
                    val nextMode = if (auxU32 == 1) IguanodonThemeMode.Light else IguanodonThemeMode.Dark
                    notifyThemeModeChanged(nextMode)
                }
                ReverseEventTypes.POWER_MODE_CHANGED -> {
                    val nextPwr = if (auxU32 == 1) IguanodonPowerMode.PowerSaving else IguanodonPowerMode.Performance
                    if (powerMode != nextPwr) {
                        powerMode = nextPwr
                    }
                    onPowerModeChanged?.invoke(nextPwr)
                }
                ReverseEventTypes.RENDERER_BACKEND_CHANGED -> {
                    val nextBackend = if (auxU32 == 1) IguanodonRendererBackend.ComposeUi else IguanodonRendererBackend.Iguanodon
                    if (rendererBackend != nextBackend) {
                        rendererBackend = nextBackend
                    }
                    onRendererBackendChanged?.invoke(nextBackend)
                }
                ReverseEventTypes.FRAME_RATE_LIMIT_CHANGED -> {
                    val nextLimit = IguanodonFrameRateLimit.fromCode(auxU32)
                    if (frameRateLimit != nextLimit) {
                        frameRateLimit = nextLimit
                    }
                    onFrameRateLimitChanged?.invoke(nextLimit)
                }
            }
            offset += ReverseEventTypes.RECORD_BYTE_SIZE
            count++
        }
        return count
    }

    fun tickEngine(dtSeconds: Float = 1f / 120f): Float {
        flushPendingCommands()
        pollEvents()
        return latestFrameMs
    }

    fun onPointerMove(x: Float, y: Float) {
        if (nativeHandle != 0L) {
            IguanodonNativeBridge.nativePointerMove(nativeHandle, x, y)
            pollEvents()
        }
    }

    fun onPointerDown(x: Float, y: Float, isRightClickOrLongPress: Boolean = false) {
        if (nativeHandle != 0L) {
            IguanodonNativeBridge.nativePointerDown(
                nativeHandle,
                x,
                y,
                if (isRightClickOrLongPress) 1 else 0,
            )
            pollEvents()
        }
    }

    fun onPointerUp(x: Float, y: Float) {
        if (nativeHandle != 0L) {
            IguanodonNativeBridge.nativePointerUp(nativeHandle, x, y)
            pollEvents()
        }
    }

    fun onScroll(x: Float, y: Float, deltaY: Float) {
        if (nativeHandle != 0L) {
            IguanodonNativeBridge.nativeScroll(nativeHandle, x, y, deltaY)
            pollEvents()
        }
    }

    fun onKeyChar(ch: Char) {
        if (nativeHandle != 0L) {
            IguanodonNativeBridge.nativeKeyChar(nativeHandle, ch.code)
            pollEvents()
        }
    }

    fun onBackspace() {
        if (nativeHandle != 0L) {
            IguanodonNativeBridge.nativeBackspace(nativeHandle)
            pollEvents()
        }
    }

    var hasVulkanSwapchain: Boolean = false
        private set

    fun attachAndroidSurface(surface: Any?, width: Int, height: Int): Boolean {
        if (nativeHandle == 0L) {
            hasVulkanSwapchain = false
            return false
        }
        val clampedW = width.coerceIn(0, 3840)
        val clampedH = height.coerceIn(0, 3840)
        if (surface != null && clampedW >= 240 && clampedH >= 240) {
            viewportWidth = clampedW
            viewportHeight = clampedH
        }
        hasVulkanSwapchain = IguanodonNativeBridge.nativeSetAndroidSurface(
            nativeHandle,
            surface,
            clampedW,
            clampedH,
        )
        return hasVulkanSwapchain
    }

    fun renderAndPresent(dtSeconds: Float): Float {
        flushPendingCommands()
        pollEvents()
        if (nativeHandle == 0L) return 0f
        latestFrameMs = IguanodonNativeBridge.nativeRenderAndPresent(
            nativeHandle,
            dtSeconds,
            viewportWidth,
            viewportHeight,
        )
        return latestFrameMs
    }

    fun renderFrame(dtSeconds: Float, outRgba: ByteArray): Float {
        flushPendingCommands()
        pollEvents()
        if (nativeHandle == 0L) return 0f
        latestFrameMs = IguanodonNativeBridge.nativeRenderFrameByteArray(
            nativeHandle,
            dtSeconds,
            viewportWidth,
            viewportHeight,
            outRgba,
        )
        return latestFrameMs
    }

    override fun close() {
        if (nativeHandle != 0L) {
            IguanodonNativeBridge.nativeDestroy(nativeHandle)
            nativeHandle = 0L
        }
    }

    private fun readIntLe(buf: ByteArray, offset: Int): Int {
        return (buf[offset].toInt() and 0xFF) or
            ((buf[offset + 1].toInt() and 0xFF) shl 8) or
            ((buf[offset + 2].toInt() and 0xFF) shl 16) or
            ((buf[offset + 3].toInt() and 0xFF) shl 24)
    }

    private fun readLongLe(buf: ByteArray, offset: Int): Long {
        val lo = readIntLe(buf, offset).toLong() and 0xFFFFFFFFL
        val hi = readIntLe(buf, offset + 4).toLong() and 0xFFFFFFFFL
        return lo or (hi shl 32)
    }
}
