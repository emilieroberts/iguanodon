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

import kotlinx.browser.window
import org.khronos.webgl.Int8Array

/**
 * WebAssembly (`iguanodon_ffi.wasm`) bridge for Kotlin/JS (`js(IR)`).
 *
 * Uses `iguanodon_wasm_alloc` + `TypedArray.set` for single-memcpy command buffer uploads and
 * `iguanodon_framebuffer_ptr` for zero-copy HTML5 `<canvas>` `ImageData` presentation.
 */
actual object IguanodonNativeBridge {
    private val wasm: dynamic
        get() = window.asDynamic().__iguanodonWasm

    private var cmdPtr: Int = 0
    private var cmdCapacity: Int = 0
    private var pollPtr: Int = 0
    private var pollCapacity: Int = 0

    actual val isNativeLoaded: Boolean
        get() = wasm != null && wasm != undefined

    private fun ensureCmdScratch(needed: Int): Int {
        val ex = wasm ?: return 0
        if (needed <= cmdCapacity && cmdPtr != 0) return cmdPtr
        if (cmdPtr != 0 && cmdCapacity > 0) {
            ex.iguanodon_wasm_free(cmdPtr, cmdCapacity)
        }
        val cap = maxOf(65536, needed + (needed ushr 2))
        cmdPtr = (ex.iguanodon_wasm_alloc(cap) as Number).toInt()
        cmdCapacity = if (cmdPtr != 0) cap else 0
        return cmdPtr
    }

    private fun ensurePollScratch(needed: Int): Int {
        val ex = wasm ?: return 0
        if (needed <= pollCapacity && pollPtr != 0) return pollPtr
        if (pollPtr != 0 && pollCapacity > 0) {
            ex.iguanodon_wasm_free(pollPtr, pollCapacity)
        }
        val cap = maxOf(4096, needed)
        pollPtr = (ex.iguanodon_wasm_alloc(cap) as Number).toInt()
        pollCapacity = if (pollPtr != 0) cap else 0
        return pollPtr
    }

    actual fun createEngine(width: Float, height: Float): Long {
        val ex = wasm ?: return 0L
        val ptr = (ex.iguanodon_engine_create(width, height) as Number).toInt()
        return ptr.toLong() and 0xFFFFFFFFL
    }

    actual fun destroyEngine(handle: Long) {
        val ex = wasm ?: return
        ex.iguanodon_engine_destroy(handle.toInt())
    }

    actual fun resizeEngine(handle: Long, width: Float, height: Float) {
        val ex = wasm ?: return
        ex.iguanodon_engine_resize(handle.toInt(), width, height)
    }

    actual fun setSafeInsets(handle: Long, left: Float, top: Float, right: Float, bottom: Float) {
        val ex = wasm ?: return
        ex.iguanodon_set_safe_insets(handle.toInt(), left, top, right, bottom)
    }

    actual fun flushCommands(handle: Long, batch: ByteArray): Int {
        val ex = wasm ?: return 0
        if (batch.isEmpty()) return 0
        val ptr = ensureCmdScratch(batch.size)
        if (ptr == 0) return 0
        val srcTyped = batch.unsafeCast<Int8Array>()
        val dstView = Int8Array(ex.memory.buffer, ptr, batch.size)
        dstView.set(srcTyped)
        return (ex.iguanodon_flush_commands(handle.toInt(), ptr, batch.size) as Number).toInt()
    }

    actual fun pollEvents(handle: Long, outBuffer: ByteArray): Int {
        val ex = wasm ?: return 0
        if (outBuffer.isEmpty()) return 0
        val ptr = ensurePollScratch(outBuffer.size)
        if (ptr == 0) return 0
        val bytesWritten = (ex.iguanodon_engine_poll_events(handle.toInt(), ptr, outBuffer.size) as Number).toInt()
        if (bytesWritten > 0) {
            val count = minOf(bytesWritten, outBuffer.size)
            val wasmSlice = Int8Array(ex.memory.buffer, ptr, count)
            outBuffer.unsafeCast<Int8Array>().set(wasmSlice)
        }
        return bytesWritten
    }

    actual fun pointerMove(handle: Long, x: Float, y: Float) {
        wasm?.iguanodon_pointer_move(handle.toInt(), x, y)
    }

    actual fun pointerDown(handle: Long, x: Float, y: Float, isRightOrLongPress: Byte) {
        wasm?.iguanodon_pointer_down(handle.toInt(), x, y, isRightOrLongPress != 0.toByte())
    }

    actual fun pointerUp(handle: Long, x: Float, y: Float) {
        wasm?.iguanodon_pointer_up(handle.toInt(), x, y)
    }

    actual fun scroll(handle: Long, x: Float, y: Float, deltaY: Float) {
        wasm?.iguanodon_scroll_wheel(handle.toInt(), x, y, deltaY)
    }

    actual fun keyChar(handle: Long, codepoint: Int) {
        wasm?.iguanodon_key_char(handle.toInt(), codepoint)
    }

    actual fun backspace(handle: Long) {
        wasm?.iguanodon_key_backspace(handle.toInt())
    }

    actual fun setAndroidSurface(handle: Long, surface: Any?, width: Int, height: Int): Boolean = false

    actual fun renderAndPresent(handle: Long, dtSec: Float, width: Int, height: Int): Float {
        val ex = wasm ?: return 0f
        return (ex.iguanodon_render_frame_rgba(handle.toInt(), dtSec, width, height, 0) as Number).toFloat()
    }

    actual fun renderFrameByteArray(
        handle: Long,
        dtSec: Float,
        width: Int,
        height: Int,
        outRgba: ByteArray,
    ): Float {
        val ex = wasm ?: return 0f
        val ms = (ex.iguanodon_render_frame_rgba(handle.toInt(), dtSec, width, height, 0) as Number).toFloat()
        val fbPtr = (ex.iguanodon_framebuffer_ptr(handle.toInt()) as Number).toInt()
        val byteLen = width * height * 4
        if (fbPtr != 0 && outRgba.size >= byteLen) {
            val wasmSlice = Int8Array(ex.memory.buffer, fbPtr, byteLen)
            outRgba.unsafeCast<Int8Array>().set(wasmSlice)
        }
        return ms
    }

    fun framebufferPtr(handle: Long): Int {
        val ex = wasm ?: return 0
        return (ex.iguanodon_framebuffer_ptr(handle.toInt()) as Number).toInt()
    }

    fun wasmMemoryBuffer(): dynamic {
        return wasm?.memory?.buffer
    }
}
