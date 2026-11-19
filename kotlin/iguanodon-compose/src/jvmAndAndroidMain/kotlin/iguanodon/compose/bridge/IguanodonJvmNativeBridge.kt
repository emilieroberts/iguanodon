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

import java.io.File
import java.io.FileOutputStream

/**
 * Multi-OS JNI bridge to `iguanodon_ffi` (`libiguanodon_ffi.so` on Android/Linux,
 * `iguanodon_ffi.dll` on Windows, and `libiguanodon_ffi.dylib` on macOS).
 */
actual object IguanodonNativeBridge {
    private var loaded: Boolean = false
    actual val isNativeLoaded: Boolean
        get() = loaded

    init {
        try {
            System.loadLibrary("iguanodon_ffi")
            loaded = true
        } catch (_: UnsatisfiedLinkError) {
            loaded = tryLoadBundledDesktopLibrary()
        }
    }

    private fun tryLoadBundledDesktopLibrary(): Boolean {
        val osName = (System.getProperty("os.name") ?: "").lowercase()
        val osArch = (System.getProperty("os.arch") ?: "").lowercase()
        val isArm64 = osArch.contains("aarch64") || osArch.contains("arm64")

        val (resourcePath, prefix, suffix) = when {
            osName.contains("win") -> Triple(
                "/META-INF/native/windows-x86_64/iguanodon_ffi.dll",
                "iguanodon_ffi_",
                ".dll",
            )
            osName.contains("mac") || osName.contains("darwin") -> {
                val archDir = if (isArm64) "macos-aarch64" else "macos-x86_64"
                Triple(
                    "/META-INF/native/$archDir/libiguanodon_ffi.dylib",
                    "libiguanodon_ffi_",
                    ".dylib",
                )
            }
            else -> Triple(
                "/META-INF/native/linux-x86_64/libiguanodon_ffi.so",
                "libiguanodon_ffi_",
                ".so",
            )
        }

        val localFileName = resourcePath.substringAfterLast('/')
        val jarDir = runCatching {
            IguanodonNativeBridge::class.java.protectionDomain?.codeSource?.location?.toURI()?.let { File(it).parentFile }
        }.getOrNull()
        val localCandidates = buildList {
            add(File("lib", localFileName))
            add(File(localFileName))
            if (jarDir != null) {
                add(File(jarDir, localFileName))
                add(File(jarDir, "lib/$localFileName"))
                add(File(jarDir, "native/$localFileName"))
            }
        }
        for (candidate in localCandidates) {
            if (candidate.isFile) {
                try {
                    System.load(candidate.absolutePath)
                    return true
                } catch (_: Throwable) {
                }
            }
        }

        return try {
            val stream = IguanodonNativeBridge::class.java.getResourceAsStream(resourcePath)
                ?: return false
            val tmp = File.createTempFile(prefix, suffix)
            tmp.setReadable(true, true)
            tmp.setWritable(true, true)
            tmp.setExecutable(true, true)
            tmp.deleteOnExit()
            FileOutputStream(tmp).use { out -> stream.copyTo(out) }
            System.load(tmp.absolutePath)
            true
        } catch (_: Throwable) {
            false
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

    actual fun createEngine(width: Float, height: Float): Long = nativeCreate(width, height)
    actual fun destroyEngine(handle: Long) = nativeDestroy(handle)
    actual fun resizeEngine(handle: Long, width: Float, height: Float) = nativeResize(handle, width, height)
    actual fun setSafeInsets(handle: Long, left: Float, top: Float, right: Float, bottom: Float) =
        nativeSetSafeInsets(handle, left, top, right, bottom)
    actual fun flushCommands(handle: Long, batch: ByteArray): Int = nativeFlushCommands(handle, batch)
    actual fun pollEvents(handle: Long, outBuffer: ByteArray): Int = nativePollEvents(handle, outBuffer)
    actual fun pointerMove(handle: Long, x: Float, y: Float) = nativePointerMove(handle, x, y)
    actual fun pointerDown(handle: Long, x: Float, y: Float, isRightOrLongPress: Byte) =
        nativePointerDown(handle, x, y, isRightOrLongPress)
    actual fun pointerUp(handle: Long, x: Float, y: Float) = nativePointerUp(handle, x, y)
    actual fun scroll(handle: Long, x: Float, y: Float, deltaY: Float) = nativeScroll(handle, x, y, deltaY)
    actual fun keyChar(handle: Long, codepoint: Int) = nativeKeyChar(handle, codepoint)
    actual fun backspace(handle: Long) = nativeBackspace(handle)
    actual fun setAndroidSurface(handle: Long, surface: Any?, width: Int, height: Int): Boolean =
        nativeSetAndroidSurface(handle, surface, width, height)
    actual fun renderAndPresent(handle: Long, dtSec: Float, width: Int, height: Int): Float =
        nativeRenderAndPresent(handle, dtSec, width, height)
    actual fun renderFrameByteArray(
        handle: Long,
        dtSec: Float,
        width: Int,
        height: Int,
        outRgba: ByteArray,
    ): Float = nativeRenderFrameByteArray(handle, dtSec, width, height, outRgba)
}
