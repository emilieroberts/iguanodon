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

import androidx.compose.runtime.AbstractApplier
import androidx.compose.runtime.BroadcastFrameClock
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Composition
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Recomposer
import androidx.compose.runtime.snapshots.Snapshot
import iguanodon.compose.bridge.IguanodonSession
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlin.coroutines.CoroutineContext

/**
 * Binary opcodes matching `crates/iguanodon-ffi/src/lib.rs` (`FfiOpcode`).
 * All `@Composable` tree and state mutations are batched into a single contiguous byte buffer
 * and flushed across FFI once per dirty frame (`iguanodon_flush_commands`).
 */
object FfiOpcodes {
    const val SET_THEME: Byte = 0x01
    const val SET_LABEL_TEXT: Byte = 0x02
    const val CLICK_NODE: Byte = 0x03
    const val SET_TEXT_FIELD_VALUE: Byte = 0x04
    const val SCROLL_NODE: Byte = 0x05
    const val SET_PROGRESS: Byte = 0x06
    const val DRAG_AND_DROP: Byte = 0x07
    const val RIGHT_CLICK_NODE: Byte = 0x08
    const val INVOKE_CONTEXT_MENU: Byte = 0x09
    const val TOGGLE_FIRE_SHADER: Byte = 0x0A
    const val SET_THEME_MODE: Byte = 0x0B
    const val SET_POWER_MODE: Byte = 0x0C
    const val SET_RENDERER_BACKEND: Byte = 0x0D
    const val SET_FRAME_RATE_LIMIT: Byte = 0x0E

    // Pure `compose.runtime` Applier Scene-Tree Opcodes (`0x10..0x1A`)
    const val INSERT_CHILD: Byte = 0x10
    const val MOVE_CHILDREN: Byte = 0x11
    const val SET_LAYOUT_STYLE: Byte = 0x12
    const val SET_VISUAL_STYLE: Byte = 0x13
    const val SET_SHADER_EFFECT: Byte = 0x14
    const val SET_TEXT: Byte = 0x15
    const val SET_IMAGE_BYTES: Byte = 0x16
    const val SET_SCROLL_STATE: Byte = 0x17
    const val SET_EVENT_MASK: Byte = 0x18
    const val REMOVE_CHILDREN: Byte = 0x19
    const val CLEAR_TREE: Byte = 0x1A
}

/**
 * Reverse UI event codes emitted by `iguanodon_engine_poll_events` (32 bytes per record).
 */
object ReverseEventTypes {
    const val CLICK: Int = 1
    const val LONG_OR_SECONDARY_CLICK: Int = 2
    const val HOVER_ENTER: Int = 3
    const val HOVER_EXIT: Int = 4
    const val DRAG_DELTA: Int = 5
    const val SCROLL_DELTA: Int = 6
    const val KEY_CHAR: Int = 7
    const val KEY_BACKSPACE: Int = 8
    const val WINDOW_RESIZE: Int = 9
    const val SAFE_INSETS: Int = 10
    const val THEME_MODE_CHANGED: Int = 11
    const val POWER_MODE_CHANGED: Int = 12
    const val RENDERER_BACKEND_CHANGED: Int = 13
    const val FRAME_RATE_LIMIT_CHANGED: Int = 14
    const val RECORD_BYTE_SIZE: Int = 32
}

/**
 * Event subscription bitmask for `FfiOpcodes.SET_EVENT_MASK` (`0x18`).
 */
object IguanodonEventMask {
    const val CLICK: Int = 1 shl 0
    const val LONG_CLICK: Int = 1 shl 1
    const val HOVER: Int = 1 shl 2
    const val DRAG: Int = 1 shl 3
    const val KEY: Int = 1 shl 4
    const val TEXT_INPUT: Int = 1 shl 5
    const val SCROLL: Int = 1 shl 6
}

/**
 * Node kind tags for `FfiOpcodes.INSERT_CHILD` (`0x10`).
 */
object NodeKinds {
    const val ROOT: Byte = 0
    const val ROW: Byte = 1
    const val COLUMN: Byte = 2
    const val BOX: Byte = 3
    const val TEXT: Byte = 4
    const val BUTTON: Byte = 5
    const val SURFACE: Byte = 6
    const val TEXT_FIELD: Byte = 7
    const val IMAGE: Byte = 8
    const val PROGRESS: Byte = 9
    const val LAZY_LIST: Byte = 10
    const val DIVIDER: Byte = 11
    const val SPACER: Byte = 12
    const val ICON: Byte = 13
    const val CANVAS: Byte = 14
}

/**
 * Node IDs matching `crates/iguanodon-widgets/src/lib.rs` (`bench_nodes`).
 * Shared between Android, Desktop, and iOS so the `@Composable` tree addresses
 * identical retained scene nodes in Rust.
 */
object BenchNodeIds {
    const val ROOT: Int = 1
    const val HEADER_LABEL: Int = 10
    const val THEME_SWITCHER_BTN: Int = 4
    const val MODE_SWITCHER_BTN: Int = 9
    const val PRIMARY_BTN: Int = 12
    const val SECONDARY_FIRE_BTN: Int = 13
    const val POWER_MODE_BTN: Int = 14
    const val RENDERER_BACKEND_BTN: Int = 15
    const val SCROLLABLE_TEXT: Int = 20
    const val LAZY_COLUMN: Int = 30
    const val DETERMINATE_PROGRESS: Int = 40
    const val INDETERMINATE_PROGRESS: Int = 41
    const val DRAGGABLE_CARD_A: Int = 50
    const val DRAGGABLE_CARD_B: Int = 51
    const val DROP_TARGET_BUCKET: Int = 52
    const val TEXT_FIELD_INPUT: Int = 60
    const val CONTEXT_MENU_REGION: Int = 70
    const val IMAGE_VIEW: Int = 80
    const val ANIMATED_SHADER_CARD: Int = 90
    const val PERF_OVERLAY_HUD: Int = 3
    const val ANIM_SPEED_SLIDER: Int = 95
    const val FPS_LIMIT_RADIO_GROUP: Int = 96
}

enum class IguanodonThemeId(val code: Byte, val displayName: String) {
    Material3Expressive(0, "Material 3 Expressive (Compose 1.9)"),
    WindowsFluent2(1, "Windows 11 Fluent 2 (Mica/Acrylic)"),
    IosLiquidGlass(2, "iOS Cupertino Liquid Glass"),
    LinuxEliteObsidian(3, "Linux Elite — Iguanodon Obsidian"),
}

enum class IguanodonThemeMode(val code: Byte, val displayName: String) {
    Dark(0, "Dark"),
    Light(1, "Light"),
    ;

    fun toggle(): IguanodonThemeMode = if (this == Dark) Light else Dark
}

/**
 * Runtime power mode (`Performance` at 120 FPS vs `PowerSaving` at 30 FPS with shader & halo optimizations).
 */
enum class IguanodonPowerMode(
    val code: Byte,
    val displayName: String,
    val targetFps: Int,
    val frameIntervalMs: Int,
) {
    Performance(0, "Performance (120 FPS)", 120, 8),
    PowerSaving(1, "Power-Saving (30 FPS)", 30, 33),
    ;

    fun toggle(): IguanodonPowerMode = if (this == Performance) PowerSaving else Performance
}

/**
 * User-selectable framerate limiter preset (`15`, `30`, `45`, `60`, `75`, `90`, `120` FPS, or `Unlimited`).
 * In `IguanodonPowerMode.PowerSaving` (Eco mode), the effective framerate is capped at `30 FPS`.
 */
enum class IguanodonFrameRateLimit(
    val code: Byte,
    val displayName: String,
    val targetFps: Int,
) {
    Fps15(0, "15 FPS", 15),
    Fps30(1, "30 FPS", 30),
    Fps45(2, "45 FPS", 45),
    Fps60(3, "60 FPS", 60),
    Fps75(4, "75 FPS", 75),
    Fps90(5, "90 FPS", 90),
    Fps120(6, "120 FPS", 120),
    Unlimited(7, "Unlimited", 0),
    ;

    fun effectiveForPowerMode(powerMode: IguanodonPowerMode): IguanodonFrameRateLimit =
        if (powerMode == IguanodonPowerMode.PowerSaving) {
            if (this == Fps15) Fps15 else Fps30
        } else {
            this
        }

    val frameIntervalNanos: Long
        get() = if (targetFps <= 0) 0L else 1_000_000_000L / targetFps.toLong()

    val frameIntervalMs: Long
        get() = if (targetFps <= 0) 0L else (1000L / targetFps.toLong()).coerceAtLeast(1L)

    companion object {
        fun fromCode(code: Int): IguanodonFrameRateLimit =
            entries.firstOrNull { it.code.toInt() == code } ?: Unlimited
    }
}

/**
 * Runtime renderer backend selection (`Iguanodon` default vs `ComposeUi` live comparison mode).
 */
enum class IguanodonRendererBackend(val code: Byte, val displayName: String) {
    Iguanodon(0, "Iguanodon"),
    ComposeUi(1, "Compose-UI"),
    ;

    fun toggle(): IguanodonRendererBackend = if (this == Iguanodon) ComposeUi else Iguanodon
}

typealias IguanodonCommandEncoder = ByteBufferCommandEncoder

class ByteBufferCommandEncoder {
    private val buffer = ArrayList<Byte>(1024)

    fun encodeSetTheme(theme: IguanodonThemeId) {
        buffer.add(FfiOpcodes.SET_THEME)
        buffer.add(theme.code)
    }

    fun encodeSetThemeMode(mode: IguanodonThemeMode) {
        buffer.add(FfiOpcodes.SET_THEME_MODE)
        buffer.add(mode.code)
    }

    fun encodeSetPowerMode(mode: IguanodonPowerMode) {
        buffer.add(FfiOpcodes.SET_POWER_MODE)
        buffer.add(mode.code)
    }

    fun encodeSetRendererBackend(backend: IguanodonRendererBackend) {
        buffer.add(FfiOpcodes.SET_RENDERER_BACKEND)
        buffer.add(backend.code)
    }

    fun encodeSetFrameRateLimit(limit: IguanodonFrameRateLimit) {
        buffer.add(FfiOpcodes.SET_FRAME_RATE_LIMIT)
        buffer.add(limit.code)
    }

    fun encodeSetLabelText(nodeId: Int, text: String) {
        buffer.add(FfiOpcodes.SET_LABEL_TEXT)
        writeIntLe(nodeId)
        writeUtf8(text)
    }

    fun encodeClickNode(nodeId: Int) {
        buffer.add(FfiOpcodes.CLICK_NODE)
        writeIntLe(nodeId)
    }

    fun encodeSetTextFieldValue(text: String) {
        buffer.add(FfiOpcodes.SET_TEXT_FIELD_VALUE)
        writeUtf8(text)
    }

    fun encodeScrollNode(nodeId: Int, deltaY: Float) {
        buffer.add(FfiOpcodes.SCROLL_NODE)
        writeIntLe(nodeId)
        writeFloatLe(deltaY)
    }

    fun encodeSetProgress(progress: Float) {
        buffer.add(FfiOpcodes.SET_PROGRESS)
        writeFloatLe(progress)
    }

    fun encodeDragAndDrop(sourceNodeId: Int, payload: String, targetNodeId: Int) {
        buffer.add(FfiOpcodes.DRAG_AND_DROP)
        writeIntLe(sourceNodeId)
        writeIntLe(targetNodeId)
        writeUtf8(payload)
    }

    fun encodeRightClickNode(nodeId: Int) {
        buffer.add(FfiOpcodes.RIGHT_CLICK_NODE)
        writeIntLe(nodeId)
    }

    fun encodeInvokeContextMenuItem(itemIndex: Int) {
        buffer.add(FfiOpcodes.INVOKE_CONTEXT_MENU)
        writeIntLe(itemIndex)
    }

    fun encodeToggleFireShader(enabled: Boolean = true) {
        buffer.add(FfiOpcodes.TOGGLE_FIRE_SHADER)
        buffer.add(if (enabled) 1 else 0)
    }

    // ========================================================================
    // Pure `compose.runtime` Applier Scene-Tree Opcodes (`0x10..0x1A`)
    // ========================================================================

    fun encodeInsertChild(parentId: Long, index: Int, childId: Long, kind: Byte) {
        buffer.add(FfiOpcodes.INSERT_CHILD)
        writeLongLe(parentId)
        writeIntLe(index)
        writeLongLe(childId)
        buffer.add(kind)
    }

    fun encodeMoveChildren(parentId: Long, from: Int, to: Int, count: Int) {
        buffer.add(FfiOpcodes.MOVE_CHILDREN)
        writeLongLe(parentId)
        writeIntLe(from)
        writeIntLe(to)
        writeIntLe(count)
    }

    fun encodeRemoveChildren(parentId: Long, index: Int, count: Int) {
        buffer.add(FfiOpcodes.REMOVE_CHILDREN)
        writeLongLe(parentId)
        writeIntLe(index)
        writeIntLe(count)
    }

    fun encodeRemoveNode(parentId: Long, index: Int, count: Int) =
        encodeRemoveChildren(parentId, index, count)

    fun encodeClearTree() {
        buffer.add(FfiOpcodes.CLEAR_TREE)
    }

    fun encodeSetLayoutStyle(
        nodeId: Long,
        flexDir: Byte,
        align: Byte,
        justify: Byte,
        padL: Float,
        padT: Float,
        padR: Float,
        padB: Float,
        gap: Float,
        width: Float,
        height: Float,
        minW: Float,
        maxW: Float,
        weight: Float,
    ) {
        buffer.add(FfiOpcodes.SET_LAYOUT_STYLE)
        writeLongLe(nodeId)
        buffer.add(flexDir)
        buffer.add(align)
        buffer.add(justify)
        writeFloatLe(padL)
        writeFloatLe(padT)
        writeFloatLe(padR)
        writeFloatLe(padB)
        writeFloatLe(gap)
        writeFloatLe(width)
        writeFloatLe(height)
        writeFloatLe(minW)
        writeFloatLe(maxW)
        writeFloatLe(weight)
    }

    fun encodeSetVisualStyle(
        nodeId: Long,
        bgRgba: Int,
        borderRgba: Int,
        borderW: Float,
        cornerRadius: Float,
        clip: Byte,
    ) {
        buffer.add(FfiOpcodes.SET_VISUAL_STYLE)
        writeLongLe(nodeId)
        writeIntLe(bgRgba)
        writeIntLe(borderRgba)
        writeFloatLe(borderW)
        writeFloatLe(cornerRadius)
        buffer.add(clip)
    }

    fun encodeSetShaderEffect(
        nodeId: Long,
        shaderId: Byte,
        p0: Float = 0f,
        p1: Float = 0f,
        p2: Float = 0f,
        p3: Float = 0f,
    ) {
        buffer.add(FfiOpcodes.SET_SHADER_EFFECT)
        writeLongLe(nodeId)
        buffer.add(shaderId)
        writeFloatLe(p0)
        writeFloatLe(p1)
        writeFloatLe(p2)
        writeFloatLe(p3)
    }

    fun encodeSetText(
        nodeId: Long,
        colorRgba: Int,
        fontSizeSp: Float,
        fontWeight: Int,
        maxLines: Int,
        text: String,
    ) {
        buffer.add(FfiOpcodes.SET_TEXT)
        writeLongLe(nodeId)
        writeIntLe(colorRgba)
        writeFloatLe(fontSizeSp)
        writeShortLe(fontWeight)
        writeShortLe(maxLines)
        writeUtf8(text)
    }

    fun encodeSetImageBytes(
        nodeId: Long,
        width: Int,
        height: Int,
        rgbaBytes: ByteArray,
    ) {
        buffer.add(FfiOpcodes.SET_IMAGE_BYTES)
        writeLongLe(nodeId)
        writeIntLe(width)
        writeIntLe(height)
        writeIntLe(rgbaBytes.size)
        for (b in rgbaBytes) buffer.add(b)
    }

    fun encodeSetScrollState(
        nodeId: Long,
        axis: Byte,
        offsetPx: Float,
    ) {
        buffer.add(FfiOpcodes.SET_SCROLL_STATE)
        writeLongLe(nodeId)
        buffer.add(axis)
        writeFloatLe(offsetPx)
    }

    fun encodeSetEventMask(nodeId: Long, mask: Int) {
        buffer.add(FfiOpcodes.SET_EVENT_MASK)
        writeLongLe(nodeId)
        writeIntLe(mask)
    }

    fun isEmpty(): Boolean = buffer.isEmpty()

    fun toByteArrayAndClear(): ByteArray {
        val out = ByteArray(buffer.size)
        for (i in buffer.indices) out[i] = buffer[i]
        buffer.clear()
        return out
    }

    private fun writeShortLe(v: Int) {
        buffer.add((v and 0xFF).toByte())
        buffer.add(((v ushr 8) and 0xFF).toByte())
    }

    private fun writeIntLe(v: Int) {
        buffer.add((v and 0xFF).toByte())
        buffer.add(((v ushr 8) and 0xFF).toByte())
        buffer.add(((v ushr 16) and 0xFF).toByte())
        buffer.add(((v ushr 24) and 0xFF).toByte())
    }

    private fun writeLongLe(v: Long) {
        writeIntLe((v and 0xFFFFFFFFL).toInt())
        writeIntLe((v ushr 32).toInt())
    }

    private fun writeFloatLe(v: Float) {
        writeIntLe(v.toRawBits())
    }

    private fun writeUtf8(s: String) {
        val bytes = s.encodeToByteArray()
        writeIntLe(bytes.size)
        for (b in bytes) buffer.add(b)
    }
}

/**
 * Retained scene-graph node managed by [IguanodonApplier] (`AbstractApplier<IguanodonNode>`).
 */
class IguanodonNode(
    val nodeId: Long,
    var nodeKind: Byte = NodeKinds.BOX,
    var modifierChain: IguanodonModifier = IguanodonModifier,
) {
    val children: MutableList<IguanodonNode> = ArrayList()
    var onClick: (() -> Unit)? = null
    var onLongClick: (() -> Unit)? = null
    var onHoverChanged: ((Boolean) -> Unit)? = null
    var onDragDelta: ((Float, Float) -> Unit)? = null
    var onScrollDelta: ((Float, Float) -> Unit)? = null
    var onTextChange: ((String) -> Unit)? = null
    var textValue: String = ""
}

/**
 * Custom `androidx.compose.runtime.AbstractApplier<IguanodonNode>` that drives Iguanodon's
 * Rust/Vulkan retained scene tree (`libiguanodon_ffi.so`) directly without `compose-ui` or Skia.
 */
class IguanodonApplier(
    root: IguanodonNode,
    val encoder: ByteBufferCommandEncoder,
    private val session: IguanodonSession? = null,
    private val onFlushFrame: (ByteArray) -> Unit = {},
) : AbstractApplier<IguanodonNode>(root) {

    constructor(
        root: IguanodonNode,
        session: IguanodonSession,
    ) : this(
        root = root,
        encoder = session.encoder,
        session = session,
        onFlushFrame = { session.flushByteArray(it) },
    )

    constructor(
        rootNodeId: Int,
        encoder: ByteBufferCommandEncoder,
        onFlushFrame: (ByteArray) -> Unit,
    ) : this(
        root = IguanodonNode(nodeId = rootNodeId.toLong(), nodeKind = NodeKinds.ROOT),
        encoder = encoder,
        session = null,
        onFlushFrame = onFlushFrame,
    )

    override fun insertTopDown(index: Int, instance: IguanodonNode) {
        val safeIdx = index.coerceIn(0, current.children.size)
        current.children.add(safeIdx, instance)
        session?.registerNode(instance)
        encoder.encodeInsertChild(
            parentId = current.nodeId,
            index = safeIdx,
            childId = instance.nodeId,
            kind = instance.nodeKind,
        )
    }

    override fun insertBottomUp(index: Int, instance: IguanodonNode) {
        // Tree insertion is emitted top-down so parent nodes exist in the Rust scene graph first.
    }

    override fun remove(index: Int, count: Int) {
        if (count <= 0 || index < 0 || index >= current.children.size) return
        val end = (index + count).coerceAtMost(current.children.size)
        for (i in (end - 1) downTo index) {
            val removed = current.children.removeAt(i)
            session?.unregisterSubtree(removed)
        }
        encoder.encodeRemoveChildren(
            parentId = current.nodeId,
            index = index,
            count = end - index,
        )
    }

    override fun move(from: Int, to: Int, count: Int) {
        if (count <= 0 || from == to || from < 0 || from >= current.children.size) return
        val end = (from + count).coerceAtMost(current.children.size)
        val moved = ArrayList<IguanodonNode>(end - from)
        for (i in from until end) {
            moved.add(current.children[from])
            current.children.removeAt(from)
        }
        val dst = if (from > to) to else (to - moved.size)
        val safeDst = dst.coerceIn(0, current.children.size)
        current.children.addAll(safeDst, moved)
        encoder.encodeMoveChildren(
            parentId = current.nodeId,
            from = from,
            to = safeDst,
            count = moved.size,
        )
    }

    override fun onClear() {
        for (c in root.children) {
            session?.unregisterSubtree(c)
        }
        root.children.clear()
        encoder.encodeClearTree()
    }

    override fun onEndChanges() {
        if (session != null) {
            session.flushPendingCommands()
        } else {
            val bytes = encoder.toByteArrayAndClear()
            if (bytes.isNotEmpty()) {
                onFlushFrame(bytes)
            }
        }
    }
}

/**
 * Pure `androidx.compose.runtime` composition driver (`Recomposer` + `BroadcastFrameClock` +
 * `Composition(IguanodonApplier)`) used by both `IguanodonDesktopWindow` and `IguanodonVulkanSurfaceView`.
 */
class IguanodonCompositionHost(
    val session: IguanodonSession,
    parentContext: CoroutineContext = kotlinx.coroutines.Dispatchers.Default,
) {
    val frameClock = BroadcastFrameClock()
    private val job = Job()
    private val scope = CoroutineScope(parentContext + frameClock + job)
    val recomposer = Recomposer(scope.coroutineContext)
    val rootNode = IguanodonNode(nodeId = BenchNodeIds.ROOT.toLong(), nodeKind = NodeKinds.ROOT)
    val applier = IguanodonApplier(root = rootNode, session = session)
    private val composition = Composition(applier, recomposer)

    init {
        session.registerNode(rootNode)
        scope.launch {
            recomposer.runRecomposeAndApplyChanges()
        }
    }

    fun setContent(content: @Composable () -> Unit) {
        composition.setContent {
            CompositionLocalProvider(LocalIguanodonSession provides session) {
                content()
            }
        }
        Snapshot.sendApplyNotifications()
        frameClock.sendFrame(System.nanoTime())
        session.flushPendingCommands()
    }

    fun doFrame(frameTimeNanos: Long) {
        Snapshot.sendApplyNotifications()
        frameClock.sendFrame(frameTimeNanos)
        session.flushPendingCommands()
    }

    fun dispose() {
        composition.dispose()
        recomposer.cancel()
        scope.cancel()
    }
}
