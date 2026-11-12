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

//! Batched Binary Command Protocol & C-ABI / JNI Bridge (`iguanodon-ffi`).
//! Allows Kotlin Multiplatform (`@Composable` `IguanodonApplier`) to serialize all tree
//! and state mutations into a single flat binary buffer per dirty frame.

use iguanodon_core::NodeId;
use iguanodon_theme::{ThemeId, ThemeMode};
use iguanodon_widgets::{
    bench_nodes, ApplierImagePayload, ApplierLayoutStyle, ApplierShaderEffect, ApplierTextPayload,
    ApplierVisualStyle, IguanodonTestHarness, PowerMode, RendererBackend, ReverseEventType,
    ReverseUiEvent,
};
use std::slice;

/// Binary Opcodes emitted by Kotlin's `IguanodonApplier` / `ByteBufferCommandEncoder`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FfiOpcode {
    SetTheme = 0x01,
    SetLabelText = 0x02,
    ClickNode = 0x03,
    SetTextFieldValue = 0x04,
    ScrollNode = 0x05,
    SetProgress = 0x06,
    DragAndDrop = 0x07,
    RightClickNode = 0x08,
    InvokeContextMenu = 0x09,
    ToggleFireShader = 0x0A,
    SetThemeMode = 0x0B,
    SetPowerMode = 0x0C,
    SetRendererBackend = 0x0D,

    // Pure `compose.runtime` Applier Scene-Tree Opcodes (`0x10..0x1A`)
    InsertChild = 0x10,
    MoveChildren = 0x11,
    SetLayoutStyle = 0x12,
    SetVisualStyle = 0x13,
    SetShaderEffect = 0x14,
    SetText = 0x15,
    SetImageBytes = 0x16,
    SetScrollState = 0x17,
    SetEventMask = 0x18,
    RemoveChildren = 0x19,
    ClearTree = 0x1A,
}

/// High-level command representation decoded from the flat binary FFI buffer.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedCommand {
    SetTheme(ThemeId),
    SetThemeMode(ThemeMode),
    SetPowerMode(PowerMode),
    SetRendererBackend(RendererBackend),
    SetLabelText {
        node_id: u32,
        text: String,
    },
    ClickNode {
        node_id: u32,
    },
    SetTextFieldValue {
        text: String,
    },
    ScrollNode {
        node_id: u32,
        delta_y: f32,
    },
    SetProgress {
        value: f32,
    },
    DragAndDrop {
        src_node: u32,
        dst_node: u32,
        payload: String,
    },
    RightClickNode {
        node_id: u32,
    },
    InvokeContextMenu {
        index: u32,
    },
    ToggleFireShader {
        enabled: bool,
    },
    InsertChild {
        parent_id: u64,
        index: u32,
        child_id: u64,
        kind: u8,
    },
    MoveChildren {
        parent_id: u64,
        from: u32,
        to: u32,
        count: u32,
    },
    SetLayoutStyle {
        node_id: u64,
        style: ApplierLayoutStyle,
    },
    SetVisualStyle {
        node_id: u64,
        style: ApplierVisualStyle,
    },
    SetShaderEffect {
        node_id: u64,
        effect: ApplierShaderEffect,
    },
    SetText {
        node_id: u64,
        payload: ApplierTextPayload,
    },
    SetImageBytes {
        node_id: u64,
        payload: ApplierImagePayload,
    },
    SetScrollState {
        node_id: u64,
        axis: u8,
        offset_px: f32,
    },
    SetEventMask {
        node_id: u64,
        mask: u32,
    },
    RemoveChildren {
        parent_id: u64,
        index: u32,
        count: u32,
    },
    ClearTree,
}

/// Zero-copy binary buffer encoder (mirrors Kotlin Multiplatform `ByteBufferCommandEncoder`).
#[derive(Debug, Default, Clone)]
pub struct BinaryCommandEncoder {
    pub bytes: Vec<u8>,
}

impl BinaryCommandEncoder {
    pub fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(512),
        }
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    pub fn encode_set_theme(&mut self, theme: ThemeId) {
        self.bytes.push(FfiOpcode::SetTheme as u8);
        self.bytes.push(theme as u8);
    }

    pub fn encode_set_theme_mode(&mut self, mode: ThemeMode) {
        self.bytes.push(FfiOpcode::SetThemeMode as u8);
        self.bytes.push(mode as u8);
    }

    pub fn encode_set_power_mode(&mut self, mode: PowerMode) {
        self.bytes.push(FfiOpcode::SetPowerMode as u8);
        self.bytes.push(mode as u8);
    }

    pub fn encode_set_renderer_backend(&mut self, backend: RendererBackend) {
        self.bytes.push(FfiOpcode::SetRendererBackend as u8);
        self.bytes.push(backend as u8);
    }

    pub fn encode_set_label_text(&mut self, node_id: u32, text: &str) {
        self.bytes.push(FfiOpcode::SetLabelText as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
        self.write_str(text);
    }

    pub fn encode_click_node(&mut self, node_id: u32) {
        self.bytes.push(FfiOpcode::ClickNode as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
    }

    pub fn encode_set_text_field(&mut self, text: &str) {
        self.bytes.push(FfiOpcode::SetTextFieldValue as u8);
        self.write_str(text);
    }

    pub fn encode_scroll_node(&mut self, node_id: u32, delta_y: f32) {
        self.bytes.push(FfiOpcode::ScrollNode as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
        self.bytes.extend_from_slice(&delta_y.to_le_bytes());
    }

    pub fn encode_set_progress(&mut self, value: f32) {
        self.bytes.push(FfiOpcode::SetProgress as u8);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn encode_drag_and_drop(&mut self, src_node: u32, dst_node: u32, payload: &str) {
        self.bytes.push(FfiOpcode::DragAndDrop as u8);
        self.bytes.extend_from_slice(&src_node.to_le_bytes());
        self.bytes.extend_from_slice(&dst_node.to_le_bytes());
        self.write_str(payload);
    }

    pub fn encode_right_click_node(&mut self, node_id: u32) {
        self.bytes.push(FfiOpcode::RightClickNode as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
    }

    pub fn encode_invoke_context_menu(&mut self, index: u32) {
        self.bytes.push(FfiOpcode::InvokeContextMenu as u8);
        self.bytes.extend_from_slice(&index.to_le_bytes());
    }

    pub fn encode_toggle_fire_shader(&mut self, enabled: bool) {
        self.bytes.push(FfiOpcode::ToggleFireShader as u8);
        self.bytes.push(u8::from(enabled));
    }

    pub fn encode_insert_child(&mut self, parent_id: u64, index: u32, child_id: u64, kind: u8) {
        self.bytes.push(FfiOpcode::InsertChild as u8);
        self.bytes.extend_from_slice(&parent_id.to_le_bytes());
        self.bytes.extend_from_slice(&index.to_le_bytes());
        self.bytes.extend_from_slice(&child_id.to_le_bytes());
        self.bytes.push(kind);
    }

    pub fn encode_move_children(&mut self, parent_id: u64, from: u32, to: u32, count: u32) {
        self.bytes.push(FfiOpcode::MoveChildren as u8);
        self.bytes.extend_from_slice(&parent_id.to_le_bytes());
        self.bytes.extend_from_slice(&from.to_le_bytes());
        self.bytes.extend_from_slice(&to.to_le_bytes());
        self.bytes.extend_from_slice(&count.to_le_bytes());
    }

    pub fn encode_set_layout_style(&mut self, node_id: u64, style: &ApplierLayoutStyle) {
        self.bytes.push(FfiOpcode::SetLayoutStyle as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
        self.bytes.push(style.flex_dir);
        self.bytes.push(style.align);
        self.bytes.push(style.justify);
        self.bytes.extend_from_slice(&style.pad_l.to_le_bytes());
        self.bytes.extend_from_slice(&style.pad_t.to_le_bytes());
        self.bytes.extend_from_slice(&style.pad_r.to_le_bytes());
        self.bytes.extend_from_slice(&style.pad_b.to_le_bytes());
        self.bytes.extend_from_slice(&style.gap.to_le_bytes());
        self.bytes.extend_from_slice(&style.width.to_le_bytes());
        self.bytes.extend_from_slice(&style.height.to_le_bytes());
        self.bytes.extend_from_slice(&style.min_w.to_le_bytes());
        self.bytes.extend_from_slice(&style.max_w.to_le_bytes());
        self.bytes.extend_from_slice(&style.weight.to_le_bytes());
    }

    pub fn encode_set_visual_style(&mut self, node_id: u64, style: &ApplierVisualStyle) {
        self.bytes.push(FfiOpcode::SetVisualStyle as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
        self.bytes.extend_from_slice(&style.bg_rgba.to_le_bytes());
        self.bytes.extend_from_slice(&style.border_rgba.to_le_bytes());
        self.bytes.extend_from_slice(&style.border_w.to_le_bytes());
        self.bytes.extend_from_slice(&style.corner_radius.to_le_bytes());
        self.bytes.push(style.clip);
    }

    pub fn encode_set_shader_effect(&mut self, node_id: u64, effect: &ApplierShaderEffect) {
        self.bytes.push(FfiOpcode::SetShaderEffect as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
        self.bytes.push(effect.shader_id);
        self.bytes.extend_from_slice(&effect.p0.to_le_bytes());
        self.bytes.extend_from_slice(&effect.p1.to_le_bytes());
        self.bytes.extend_from_slice(&effect.p2.to_le_bytes());
        self.bytes.extend_from_slice(&effect.p3.to_le_bytes());
    }

    pub fn encode_set_text(&mut self, node_id: u64, payload: &ApplierTextPayload) {
        self.bytes.push(FfiOpcode::SetText as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
        self.bytes.extend_from_slice(&payload.color_rgba.to_le_bytes());
        self.bytes.extend_from_slice(&payload.font_size_sp.to_le_bytes());
        self.bytes.extend_from_slice(&payload.font_weight.to_le_bytes());
        self.bytes.extend_from_slice(&payload.max_lines.to_le_bytes());
        self.write_str(&payload.text);
    }

    pub fn encode_set_image_bytes(
        &mut self,
        node_id: u64,
        width: u32,
        height: u32,
        rgba_bytes: &[u8],
    ) {
        self.bytes.push(FfiOpcode::SetImageBytes as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
        self.bytes.extend_from_slice(&width.to_le_bytes());
        self.bytes.extend_from_slice(&height.to_le_bytes());
        self.bytes
            .extend_from_slice(&(rgba_bytes.len() as u32).to_le_bytes());
        self.bytes.extend_from_slice(rgba_bytes);
    }

    pub fn encode_set_scroll_state(&mut self, node_id: u64, axis: u8, offset_px: f32) {
        self.bytes.push(FfiOpcode::SetScrollState as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
        self.bytes.push(axis);
        self.bytes.extend_from_slice(&offset_px.to_le_bytes());
    }

    pub fn encode_set_event_mask(&mut self, node_id: u64, mask: u32) {
        self.bytes.push(FfiOpcode::SetEventMask as u8);
        self.bytes.extend_from_slice(&node_id.to_le_bytes());
        self.bytes.extend_from_slice(&mask.to_le_bytes());
    }

    pub fn encode_remove_children(&mut self, parent_id: u64, index: u32, count: u32) {
        self.bytes.push(FfiOpcode::RemoveChildren as u8);
        self.bytes.extend_from_slice(&parent_id.to_le_bytes());
        self.bytes.extend_from_slice(&index.to_le_bytes());
        self.bytes.extend_from_slice(&count.to_le_bytes());
    }

    pub fn encode_clear_tree(&mut self) {
        self.bytes.push(FfiOpcode::ClearTree as u8);
    }

    fn write_str(&mut self, s: &str) {
        let b = s.as_bytes();
        self.bytes.extend_from_slice(&(b.len() as u32).to_le_bytes());
        self.bytes.extend_from_slice(b);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    UnexpectedEof,
    UnknownOpcode(u8),
    InvalidUtf8,
}

/// Decodes a batched binary command buffer produced by Kotlin Multiplatform or Rust.
pub fn decode_command_buffer(buf: &[u8]) -> Result<Vec<DecodedCommand>, DecodeError> {
    let mut cursor = 0_usize;
    let mut cmds = Vec::new();

    let read_u8 = |c: &mut usize| -> Result<u8, DecodeError> {
        if *c >= buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let val = buf[*c];
        *c += 1;
        Ok(val)
    };

    let read_u16 = |c: &mut usize| -> Result<u16, DecodeError> {
        let end = c.checked_add(2).ok_or(DecodeError::UnexpectedEof)?;
        if end > buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let val = u16::from_le_bytes([buf[*c], buf[*c + 1]]);
        *c = end;
        Ok(val)
    };

    let read_u32 = |c: &mut usize| -> Result<u32, DecodeError> {
        let end = c.checked_add(4).ok_or(DecodeError::UnexpectedEof)?;
        if end > buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let val = u32::from_le_bytes([buf[*c], buf[*c + 1], buf[*c + 2], buf[*c + 3]]);
        *c = end;
        Ok(val)
    };

    let read_u64 = |c: &mut usize| -> Result<u64, DecodeError> {
        let end = c.checked_add(8).ok_or(DecodeError::UnexpectedEof)?;
        if end > buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let val = u64::from_le_bytes([
            buf[*c],
            buf[*c + 1],
            buf[*c + 2],
            buf[*c + 3],
            buf[*c + 4],
            buf[*c + 5],
            buf[*c + 6],
            buf[*c + 7],
        ]);
        *c = end;
        Ok(val)
    };

    let read_f32 = |c: &mut usize| -> Result<f32, DecodeError> {
        let end = c.checked_add(4).ok_or(DecodeError::UnexpectedEof)?;
        if end > buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let val = f32::from_le_bytes([buf[*c], buf[*c + 1], buf[*c + 2], buf[*c + 3]]);
        *c = end;
        Ok(val)
    };

    let read_str = |c: &mut usize| -> Result<String, DecodeError> {
        let len = read_u32(c)? as usize;
        let end = c.checked_add(len).ok_or(DecodeError::UnexpectedEof)?;
        if end > buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let s = std::str::from_utf8(&buf[*c..end])
            .map_err(|_| DecodeError::InvalidUtf8)?
            .to_string();
        *c = end;
        Ok(s)
    };

    while cursor < buf.len() {
        let op = buf[cursor];
        cursor += 1;
        match op {
            x if x == FfiOpcode::SetTheme as u8 => {
                let th = ThemeId::from_u8(read_u8(&mut cursor)?);
                cmds.push(DecodedCommand::SetTheme(th));
            }
            x if x == FfiOpcode::SetLabelText as u8 => {
                let node_id = read_u32(&mut cursor)?;
                let text = read_str(&mut cursor)?;
                cmds.push(DecodedCommand::SetLabelText { node_id, text });
            }
            x if x == FfiOpcode::ClickNode as u8 => {
                let node_id = read_u32(&mut cursor)?;
                cmds.push(DecodedCommand::ClickNode { node_id });
            }
            x if x == FfiOpcode::SetTextFieldValue as u8 => {
                let text = read_str(&mut cursor)?;
                cmds.push(DecodedCommand::SetTextFieldValue { text });
            }
            x if x == FfiOpcode::ScrollNode as u8 => {
                let node_id = read_u32(&mut cursor)?;
                let delta_y = read_f32(&mut cursor)?;
                cmds.push(DecodedCommand::ScrollNode { node_id, delta_y });
            }
            x if x == FfiOpcode::SetProgress as u8 => {
                let value = read_f32(&mut cursor)?;
                cmds.push(DecodedCommand::SetProgress { value });
            }
            x if x == FfiOpcode::DragAndDrop as u8 => {
                let src_node = read_u32(&mut cursor)?;
                let dst_node = read_u32(&mut cursor)?;
                let payload = read_str(&mut cursor)?;
                cmds.push(DecodedCommand::DragAndDrop {
                    src_node,
                    dst_node,
                    payload,
                });
            }
            x if x == FfiOpcode::RightClickNode as u8 => {
                let node_id = read_u32(&mut cursor)?;
                cmds.push(DecodedCommand::RightClickNode { node_id });
            }
            x if x == FfiOpcode::InvokeContextMenu as u8 => {
                let index = read_u32(&mut cursor)?;
                cmds.push(DecodedCommand::InvokeContextMenu { index });
            }
            x if x == FfiOpcode::ToggleFireShader as u8 => {
                let enabled = read_u8(&mut cursor)? != 0;
                cmds.push(DecodedCommand::ToggleFireShader { enabled });
            }
            x if x == FfiOpcode::SetThemeMode as u8 => {
                let mode = ThemeMode::from_u8(read_u8(&mut cursor)?);
                cmds.push(DecodedCommand::SetThemeMode(mode));
            }
            x if x == FfiOpcode::SetPowerMode as u8 => {
                let mode = PowerMode::from_u8(read_u8(&mut cursor)?);
                cmds.push(DecodedCommand::SetPowerMode(mode));
            }
            x if x == FfiOpcode::SetRendererBackend as u8 => {
                let backend = RendererBackend::from_u8(read_u8(&mut cursor)?);
                cmds.push(DecodedCommand::SetRendererBackend(backend));
            }
            x if x == FfiOpcode::InsertChild as u8 => {
                let parent_id = read_u64(&mut cursor)?;
                let index = read_u32(&mut cursor)?;
                let child_id = read_u64(&mut cursor)?;
                let kind = read_u8(&mut cursor)?;
                cmds.push(DecodedCommand::InsertChild {
                    parent_id,
                    index,
                    child_id,
                    kind,
                });
            }
            x if x == FfiOpcode::MoveChildren as u8 => {
                let parent_id = read_u64(&mut cursor)?;
                let from = read_u32(&mut cursor)?;
                let to = read_u32(&mut cursor)?;
                let count = read_u32(&mut cursor)?;
                cmds.push(DecodedCommand::MoveChildren {
                    parent_id,
                    from,
                    to,
                    count,
                });
            }
            x if x == FfiOpcode::SetLayoutStyle as u8 => {
                let node_id = read_u64(&mut cursor)?;
                let flex_dir = read_u8(&mut cursor)?;
                let align = read_u8(&mut cursor)?;
                let justify = read_u8(&mut cursor)?;
                let pad_l = read_f32(&mut cursor)?;
                let pad_t = read_f32(&mut cursor)?;
                let pad_r = read_f32(&mut cursor)?;
                let pad_b = read_f32(&mut cursor)?;
                let gap = read_f32(&mut cursor)?;
                let width = read_f32(&mut cursor)?;
                let height = read_f32(&mut cursor)?;
                let min_w = read_f32(&mut cursor)?;
                let max_w = read_f32(&mut cursor)?;
                let weight = read_f32(&mut cursor)?;
                cmds.push(DecodedCommand::SetLayoutStyle {
                    node_id,
                    style: ApplierLayoutStyle {
                        flex_dir,
                        align,
                        justify,
                        pad_l,
                        pad_t,
                        pad_r,
                        pad_b,
                        gap,
                        width,
                        height,
                        min_w,
                        max_w,
                        weight,
                    },
                });
            }
            x if x == FfiOpcode::SetVisualStyle as u8 => {
                let node_id = read_u64(&mut cursor)?;
                let bg_rgba = read_u32(&mut cursor)?;
                let border_rgba = read_u32(&mut cursor)?;
                let border_w = read_f32(&mut cursor)?;
                let corner_radius = read_f32(&mut cursor)?;
                let clip = read_u8(&mut cursor)?;
                cmds.push(DecodedCommand::SetVisualStyle {
                    node_id,
                    style: ApplierVisualStyle {
                        bg_rgba,
                        border_rgba,
                        border_w,
                        corner_radius,
                        clip,
                    },
                });
            }
            x if x == FfiOpcode::SetShaderEffect as u8 => {
                let node_id = read_u64(&mut cursor)?;
                let shader_id = read_u8(&mut cursor)?;
                let p0 = read_f32(&mut cursor)?;
                let p1 = read_f32(&mut cursor)?;
                let p2 = read_f32(&mut cursor)?;
                let p3 = read_f32(&mut cursor)?;
                cmds.push(DecodedCommand::SetShaderEffect {
                    node_id,
                    effect: ApplierShaderEffect {
                        shader_id,
                        p0,
                        p1,
                        p2,
                        p3,
                    },
                });
            }
            x if x == FfiOpcode::SetText as u8 => {
                let node_id = read_u64(&mut cursor)?;
                let color_rgba = read_u32(&mut cursor)?;
                let font_size_sp = read_f32(&mut cursor)?;
                let font_weight = read_u16(&mut cursor)?;
                let max_lines = read_u16(&mut cursor)?;
                let text = read_str(&mut cursor)?;
                cmds.push(DecodedCommand::SetText {
                    node_id,
                    payload: ApplierTextPayload {
                        color_rgba,
                        font_size_sp,
                        font_weight,
                        max_lines,
                        text,
                    },
                });
            }
            x if x == FfiOpcode::SetImageBytes as u8 => {
                let node_id = read_u64(&mut cursor)?;
                let width = read_u32(&mut cursor)?;
                let height = read_u32(&mut cursor)?;
                let byte_len = read_u32(&mut cursor)? as usize;
                let end = cursor.checked_add(byte_len).ok_or(DecodeError::UnexpectedEof)?;
                if end > buf.len() {
                    return Err(DecodeError::UnexpectedEof);
                }
                let rgba_bytes = buf[cursor..end].to_vec();
                cursor = end;
                cmds.push(DecodedCommand::SetImageBytes {
                    node_id,
                    payload: ApplierImagePayload {
                        width,
                        height,
                        rgba_bytes,
                    },
                });
            }
            x if x == FfiOpcode::SetScrollState as u8 => {
                let node_id = read_u64(&mut cursor)?;
                let axis = read_u8(&mut cursor)?;
                let offset_px = read_f32(&mut cursor)?;
                cmds.push(DecodedCommand::SetScrollState {
                    node_id,
                    axis,
                    offset_px,
                });
            }
            x if x == FfiOpcode::SetEventMask as u8 => {
                let node_id = read_u64(&mut cursor)?;
                let mask = read_u32(&mut cursor)?;
                cmds.push(DecodedCommand::SetEventMask { node_id, mask });
            }
            x if x == FfiOpcode::RemoveChildren as u8 => {
                let parent_id = read_u64(&mut cursor)?;
                let index = read_u32(&mut cursor)?;
                let count = read_u32(&mut cursor)?;
                cmds.push(DecodedCommand::RemoveChildren {
                    parent_id,
                    index,
                    count,
                });
            }
            x if x == FfiOpcode::ClearTree as u8 => {
                cmds.push(DecodedCommand::ClearTree);
            }
            other => return Err(DecodeError::UnknownOpcode(other)),
        }
    }

    Ok(cmds)
}

/// Applies a batch of decoded FFI commands directly to an `IguanodonTestHarness` / engine instance.
pub fn apply_commands_to_harness(
    harness: &mut IguanodonTestHarness,
    buf: &[u8],
) -> Result<usize, DecodeError> {
    let cmds = decode_command_buffer(buf)?;
    let count = cmds.len();
    let mut applier_tree_dirty = false;

    for cmd in cmds {
        match cmd {
            DecodedCommand::SetTheme(t) => harness.app.set_theme(t),
            DecodedCommand::SetThemeMode(m) => harness.app.set_theme_mode(m),
            DecodedCommand::SetPowerMode(p) => harness.app.set_power_mode(p),
            DecodedCommand::SetRendererBackend(b) => harness.app.set_renderer_backend(b),
            DecodedCommand::SetLabelText { node_id, text } => {
                if node_id == bench_nodes::HEADLINE_LABEL.index {
                    harness.app.headline_text = text;
                } else if node_id == bench_nodes::STATUS_LABEL.index {
                    harness.app.status_label_text = text;
                }
            }
            DecodedCommand::ClickNode { node_id } => {
                // Suppress echoing a ReverseEventType::Click back to Kotlin when the click came from an FFI command batch
                let prev_len = harness.pending_reverse_events.len();
                harness.click_node(NodeId::from_raw(node_id));
                let mut idx = 0;
                harness.pending_reverse_events.retain(|ev| {
                    let keep = idx < prev_len || ev.event_type != ReverseEventType::Click;
                    idx += 1;
                    keep
                });
            }
            DecodedCommand::SetTextFieldValue { text } => {
                harness.app.text_field.text = text;
                harness.app.text_field.cursor_byte = harness.app.text_field.text.len();
            }
            DecodedCommand::ScrollNode { node_id, delta_y } => {
                harness.scroll_node(NodeId::from_raw(node_id), delta_y);
            }
            DecodedCommand::SetProgress { value } => {
                harness.app.progress_spring.set_target(value.clamp(0.0, 1.0));
            }
            DecodedCommand::DragAndDrop {
                src_node,
                dst_node,
                payload,
            } => {
                harness.drag_and_drop(
                    NodeId::from_raw(src_node),
                    &payload,
                    NodeId::from_raw(dst_node),
                );
            }
            DecodedCommand::RightClickNode { node_id } => {
                harness.right_click_node(NodeId::from_raw(node_id));
            }
            DecodedCommand::InvokeContextMenu { index } => {
                harness.click_context_menu_item(index as usize);
            }
            DecodedCommand::ToggleFireShader { enabled } => {
                harness.app.fire_shader_enabled = enabled;
            }
            DecodedCommand::InsertChild {
                parent_id,
                index,
                child_id,
                kind,
            } => {
                harness
                    .applier_scene
                    .insert_child(parent_id, index as usize, child_id, kind);
                applier_tree_dirty = true;
            }
            DecodedCommand::MoveChildren {
                parent_id,
                from,
                to,
                count,
            } => {
                harness
                    .applier_scene
                    .move_children(parent_id, from as usize, to as usize, count as usize);
                applier_tree_dirty = true;
            }
            DecodedCommand::SetLayoutStyle { node_id, style } => {
                harness.applier_scene.ensure_node(node_id, 0).layout = style;
                applier_tree_dirty = true;
            }
            DecodedCommand::SetVisualStyle { node_id, style } => {
                harness.applier_scene.ensure_node(node_id, 0).visual = style;
            }
            DecodedCommand::SetShaderEffect { node_id, effect } => {
                harness.applier_scene.ensure_node(node_id, 0).shader = effect;
            }
            DecodedCommand::SetText { node_id, payload } => {
                if node_id == bench_nodes::HEADLINE_LABEL.index as u64 {
                    harness.app.headline_text = payload.text.clone();
                }
                harness.applier_scene.ensure_node(node_id, 0).text = Some(payload);
            }
            DecodedCommand::SetImageBytes { node_id, payload } => {
                if payload.width > 0 && payload.height > 0 {
                    harness
                        .vk
                        .register_texture(iguanodon_widgets::GpuImageTexture {
                            id: node_id as u32,
                            width: payload.width,
                            height: payload.height,
                            pixels_rgba8: payload.rgba_bytes.clone(),
                        });
                }
                harness.applier_scene.ensure_node(node_id, 0).image = Some(payload);
            }
            DecodedCommand::SetScrollState {
                node_id,
                axis,
                offset_px,
            } => {
                let node = harness.applier_scene.ensure_node(node_id, 0);
                node.scroll_axis = axis;
                node.scroll_offset_px = offset_px;
            }
            DecodedCommand::SetEventMask { node_id, mask } => {
                harness.applier_scene.ensure_node(node_id, 0).event_mask = mask;
                applier_tree_dirty = true;
            }
            DecodedCommand::RemoveChildren {
                parent_id,
                index,
                count,
            } => {
                harness
                    .applier_scene
                    .remove_children(parent_id, index as usize, count as usize);
                applier_tree_dirty = true;
            }
            DecodedCommand::ClearTree => {
                harness.applier_scene.clear();
                applier_tree_dirty = true;
            }
        }
    }

    if applier_tree_dirty {
        let vw = harness.app.viewport.width;
        let vh = harness.app.viewport.height;
        harness.applier_scene.recompute_layout(vw, vh);
    }

    Ok(count)
}

// ============================================================================
// C-ABI Entrypoints for Kotlin Multiplatform (`cinterop` & Native Hosts)
// ============================================================================

#[no_mangle]
pub extern "C" fn iguanodon_engine_create(width: f32, height: f32) -> *mut IguanodonTestHarness {
    Box::into_raw(Box::new(IguanodonTestHarness::new(width, height)))
}

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_engine_resize(
    engine_ptr: *mut IguanodonTestHarness,
    width: f32,
    height: f32,
) {
    if !engine_ptr.is_null() {
        (*engine_ptr).resize(width, height);
    }
}

/// Configures edge-to-edge safe-area insets (`left`, `top`, `right`, `bottom` in logical dp)
/// so the 1080p Water Drop Wallpaper renders edge-to-edge behind system bars and camera cutouts
/// while keeping interactive widgets inside the safe area.
///
/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_set_safe_insets(
    engine_ptr: *mut IguanodonTestHarness,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
) {
    if !engine_ptr.is_null() {
        let harness = &mut *engine_ptr;
        harness.app.set_safe_insets(left, top, right, bottom);
        harness.push_reverse_event(ReverseUiEvent {
            event_type: ReverseEventType::SafeInsets,
            flags: 0,
            aux_u32: 0,
            node_id: 0,
            f0: left,
            f1: top,
            f2: right,
            f3: bottom,
        });
    }
}

/// Drains queued reverse UI events (`CLICK`, `LONG_CLICK`, `HOVER_ENTER`, `HOVER_EXIT`,
/// `DRAG_DELTA`, `SCROLL_DELTA`, `KEY_CHAR`, `KEY_BACKSPACE`, `WINDOW_RESIZE`, `SAFE_INSETS`)
/// into `out_ptr` (each record is 32 bytes). Returns the number of bytes written.
///
/// # Safety
/// `engine_ptr` must be valid, and `out_ptr` must be valid for `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_engine_poll_events(
    engine_ptr: *mut IguanodonTestHarness,
    out_ptr: *mut u8,
    capacity: usize,
) -> usize {
    if engine_ptr.is_null() || out_ptr.is_null() || capacity < ReverseUiEvent::BYTE_SIZE {
        return 0;
    }
    let harness = &mut *engine_ptr;
    let out_slice = slice::from_raw_parts_mut(out_ptr, capacity);
    harness.drain_reverse_events(out_slice)
}

pub const MAX_FFI_BATCH_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_FRAMEBUFFER_DIM: u32 = 8192;

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`, and
/// `buffer_ptr` must be valid for `byte_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_flush_commands(
    engine_ptr: *mut IguanodonTestHarness,
    buffer_ptr: *const u8,
    byte_len: usize,
) -> i32 {
    if engine_ptr.is_null()
        || (buffer_ptr.is_null() && byte_len > 0)
        || byte_len > MAX_FFI_BATCH_BYTES
    {
        return -1;
    }
    let harness = &mut *engine_ptr;
    let slice = if byte_len == 0 {
        &[]
    } else {
        slice::from_raw_parts(buffer_ptr, byte_len)
    };
    match apply_commands_to_harness(harness, slice) {
        Ok(count) => count as i32,
        Err(_) => -2,
    }
}

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_pointer_move(
    engine_ptr: *mut IguanodonTestHarness,
    x: f32,
    y: f32,
) {
    if !engine_ptr.is_null() {
        (*engine_ptr).on_pointer_move(iguanodon_core::Point::new(x, y));
    }
}

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_pointer_down(
    engine_ptr: *mut IguanodonTestHarness,
    x: f32,
    y: f32,
    is_right_or_long_press: bool,
) {
    if !engine_ptr.is_null() {
        (*engine_ptr).on_pointer_down(
            iguanodon_core::Point::new(x, y),
            is_right_or_long_press,
        );
    }
}

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_pointer_up(
    engine_ptr: *mut IguanodonTestHarness,
    x: f32,
    y: f32,
) {
    if !engine_ptr.is_null() {
        (*engine_ptr).on_pointer_up(iguanodon_core::Point::new(x, y));
    }
}

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_scroll_wheel(
    engine_ptr: *mut IguanodonTestHarness,
    x: f32,
    y: f32,
    delta_y: f32,
) {
    if !engine_ptr.is_null() {
        (*engine_ptr).on_scroll_wheel(iguanodon_core::Point::new(x, y), delta_y);
    }
}

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_key_char(
    engine_ptr: *mut IguanodonTestHarness,
    unicode_codepoint: u32,
) {
    if !engine_ptr.is_null() {
        let harness = &mut *engine_ptr;
        if let Some(ch) = char::from_u32(unicode_codepoint) {
            if harness.app.text_field.focused {
                harness.app.text_field.insert_str(&ch.to_string());
            }
            let target_id = harness.applier_scene.focused_node_id.unwrap_or(0);
            harness.push_reverse_event(ReverseUiEvent {
                event_type: ReverseEventType::KeyChar,
                flags: 0,
                aux_u32: unicode_codepoint,
                node_id: target_id,
                f0: 0.0,
                f1: 0.0,
                f2: 0.0,
                f3: 0.0,
            });
        }
    }
}

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_key_backspace(engine_ptr: *mut IguanodonTestHarness) {
    if !engine_ptr.is_null() {
        let harness = &mut *engine_ptr;
        if harness.app.text_field.focused {
            harness.app.text_field.backspace();
        }
        let target_id = harness.applier_scene.focused_node_id.unwrap_or(0);
        harness.push_reverse_event(ReverseUiEvent {
            event_type: ReverseEventType::KeyBackspace,
            flags: 0,
            aux_u32: 0,
            node_id: target_id,
            f0: 0.0,
            f1: 0.0,
            f2: 0.0,
            f3: 0.0,
        });
    }
}

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_render_frame(
    engine_ptr: *mut IguanodonTestHarness,
    dt_sec: f32,
) -> f32 {
    if engine_ptr.is_null() {
        return 0.0;
    }
    let harness = &mut *engine_ptr;
    let w = (harness.app.viewport.width as u32).max(64);
    let h = (harness.app.viewport.height as u32).max(40);
    let sample = harness.render_frame_rgba(dt_sec, w, h);
    sample.total_frame_ms
}

/// Attaches an Android `ANativeWindow*` to the engine's hardware Vulkan `VkSwapchainKHR`
/// (or detaches the surface when `anative_window` is null).
///
/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_attach_android_surface(
    engine_ptr: *mut IguanodonTestHarness,
    anative_window: *mut std::ffi::c_void,
    width: u32,
    height: u32,
) -> bool {
    if engine_ptr.is_null() {
        return false;
    }
    let harness = &mut *engine_ptr;
    if anative_window.is_null() || width == 0 || height == 0 {
        harness.vk.detach_surface();
        return false;
    }
    harness.resize(width as f32, height as f32);
    harness
        .vk
        .attach_android_surface(anative_window, width, height)
}

/// Attaches a Linux X11 `Display*` and `Window` to the engine's hardware Vulkan `VkSwapchainKHR`
/// (or detaches the surface when `dpy` is null or `window == 0`).
///
/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_attach_xlib_surface(
    engine_ptr: *mut IguanodonTestHarness,
    dpy: *mut std::ffi::c_void,
    window: u64,
    width: u32,
    height: u32,
) -> bool {
    if engine_ptr.is_null() {
        return false;
    }
    let harness = &mut *engine_ptr;
    if dpy.is_null() || window == 0 || width == 0 || height == 0 {
        harness.vk.detach_surface();
        return false;
    }
    harness.resize(width as f32, height as f32);
    harness.vk.attach_xlib_surface(dpy, window, width, height)
}

/// Renders a frame directly to the attached hardware Vulkan swapchain (`VkSwapchainKHR`)
/// and presents it via `vkQueuePresentKHR`.
///
/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_render_and_present(
    engine_ptr: *mut IguanodonTestHarness,
    dt_sec: f32,
    width: u32,
    height: u32,
) -> f32 {
    if engine_ptr.is_null() || width == 0 || height == 0 {
        return 0.0;
    }
    let harness = &mut *engine_ptr;
    let w = width.min(MAX_FRAMEBUFFER_DIM);
    let h = height.min(MAX_FRAMEBUFFER_DIM);
    let sample = harness.render_and_present(dt_sec, w, h);
    sample.total_frame_ms
}

/// Renders a frame at `(fb_width, fb_height)` and copies the RGBA8888 output into `out_rgba_ptr`
/// (used by offscreen readback and Desktop JVM surface blitting).
///
/// # Safety
/// `engine_ptr` must be valid, and `out_rgba_ptr` must be valid for `fb_width * fb_height * 4` bytes.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_render_frame_rgba(
    engine_ptr: *mut IguanodonTestHarness,
    dt_sec: f32,
    fb_width: u32,
    fb_height: u32,
    out_rgba_ptr: *mut u8,
) -> f32 {
    if engine_ptr.is_null() || fb_width == 0 || fb_height == 0 {
        return 0.0;
    }
    let fb_width = fb_width.min(MAX_FRAMEBUFFER_DIM);
    let fb_height = fb_height.min(MAX_FRAMEBUFFER_DIM);
    let harness = &mut *engine_ptr;
    let sample = harness.render_frame_rgba(dt_sec, fb_width, fb_height);
    if !out_rgba_ptr.is_null() {
        if let Some(byte_len) = (fb_width as usize)
            .checked_mul(fb_height as usize)
            .and_then(|px| px.checked_mul(4))
        {
            if harness.framebuffer_rgba.len() >= byte_len {
                std::ptr::copy_nonoverlapping(
                    harness.framebuffer_rgba.as_ptr(),
                    out_rgba_ptr,
                    byte_len,
                );
            }
        }
    }
    sample.total_frame_ms
}

/// # Safety
/// `engine_ptr` must be a valid pointer returned by `iguanodon_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn iguanodon_engine_destroy(engine_ptr: *mut IguanodonTestHarness) {
    if !engine_ptr.is_null() {
        drop(Box::from_raw(engine_ptr));
    }
}

// ============================================================================
// Android & Desktop JVM JNI Entrypoints (`iguanodon.compose.bridge.IguanodonNativeBridge`)
// ============================================================================

#[no_mangle]
pub extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeCreate(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    width: f32,
    height: f32,
) -> i64 {
    iguanodon_engine_create(width, height) as i64
}

/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeResize(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    width: f32,
    height: f32,
) {
    iguanodon_engine_resize(handle as *mut IguanodonTestHarness, width, height);
}

/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeSetSafeInsets(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
) {
    iguanodon_set_safe_insets(
        handle as *mut IguanodonTestHarness,
        left,
        top,
        right,
        bottom,
    );
}

/// # Safety
/// `handle` must be a valid engine handle and `direct_buf_ptr` a valid pointer for `byte_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeFlushDirectBuffer(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    direct_buf_ptr: i64,
    byte_len: i32,
) -> i32 {
    iguanodon_flush_commands(
        handle as *mut IguanodonTestHarness,
        direct_buf_ptr as *const u8,
        byte_len.max(0) as usize,
    )
}

/// Flushes a `byte[]` (`jbyteArray`) command batch from Android ART or Desktop JVM into the Rust engine.
///
/// # Safety
/// `env` must be a valid `JNIEnv*` and `jbyte_array` a valid `jbyteArray`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeFlushCommands(
    env: *mut *const *const std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    jbyte_array: *mut std::ffi::c_void,
) -> i32 {
    if env.is_null() || (*env).is_null() || jbyte_array.is_null() || handle == 0 {
        return -1;
    }
    let vtable = *env;
    let get_array_len: unsafe extern "C" fn(
        *mut *const *const std::ffi::c_void,
        *mut std::ffi::c_void,
    ) -> i32 = std::mem::transmute(*vtable.add(171));
    let get_byte_array_region: unsafe extern "C" fn(
        *mut *const *const std::ffi::c_void,
        *mut std::ffi::c_void,
        i32,
        i32,
        *mut u8,
    ) = std::mem::transmute(*vtable.add(200));

    let len = get_array_len(env, jbyte_array).max(0);
    if len == 0 {
        return 0;
    }
    if (len as usize) > MAX_FFI_BATCH_BYTES {
        return -1;
    }
    let mut temp = vec![0_u8; len as usize];
    get_byte_array_region(env, jbyte_array, 0, len, temp.as_mut_ptr());
    iguanodon_flush_commands(
        handle as *mut IguanodonTestHarness,
        temp.as_ptr(),
        temp.len(),
    )
}

/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativePointerMove(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    x: f32,
    y: f32,
) {
    iguanodon_pointer_move(handle as *mut IguanodonTestHarness, x, y);
}

/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativePointerDown(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    x: f32,
    y: f32,
    is_right_or_long_press: u8,
) {
    iguanodon_pointer_down(
        handle as *mut IguanodonTestHarness,
        x,
        y,
        is_right_or_long_press != 0,
    );
}

/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativePointerUp(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    x: f32,
    y: f32,
) {
    iguanodon_pointer_up(handle as *mut IguanodonTestHarness, x, y);
}

/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeScroll(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    x: f32,
    y: f32,
    delta_y: f32,
) {
    iguanodon_scroll_wheel(handle as *mut IguanodonTestHarness, x, y, delta_y);
}

/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeKeyChar(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    codepoint: i32,
) {
    iguanodon_key_char(handle as *mut IguanodonTestHarness, codepoint as u32);
}

/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeBackspace(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) {
    iguanodon_key_backspace(handle as *mut IguanodonTestHarness);
}

/// # Safety
/// `handle` must be a valid engine handle and `direct_rgba_ptr` valid for `width * height * 4` bytes.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeRenderFrameRgba(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    dt_sec: f32,
    width: i32,
    height: i32,
    direct_rgba_ptr: i64,
) -> f32 {
    iguanodon_render_frame_rgba(
        handle as *mut IguanodonTestHarness,
        dt_sec,
        width.max(1) as u32,
        height.max(1) as u32,
        direct_rgba_ptr as *mut u8,
    )
}

/// Attaches an `android.view.Surface` (`jobject`) directly to the hardware Vulkan `VkSwapchainKHR`
/// via `ANativeWindow_fromSurface` (`libandroid.so`), or detaches the swapchain when `jsurface` is null.
///
/// # Safety
/// `env` must be a valid `JNIEnv*` and `handle` a valid engine handle.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeSetAndroidSurface(
    env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    jsurface: *mut std::ffi::c_void,
    width: i32,
    height: i32,
) -> u8 {
    use std::sync::atomic::{AtomicPtr, Ordering};
    static ACTIVE_ANW: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

    if handle == 0 {
        return 0;
    }
    let harness = &mut *(handle as *mut IguanodonTestHarness);
    harness.vk.detach_surface();

    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        extern "C" {
            fn dlopen(filename: *const i8, flags: i32) -> *mut std::ffi::c_void;
            fn dlsym(
                handle: *mut std::ffi::c_void,
                symbol: *const i8,
            ) -> *mut std::ffi::c_void;
        }
        let lib = dlopen(b"libandroid.so\0".as_ptr() as *const i8, 2);
        if !lib.is_null() {
            let prev_anw = ACTIVE_ANW.swap(std::ptr::null_mut(), Ordering::SeqCst);
            if !prev_anw.is_null() {
                let release_sym = dlsym(lib, b"ANativeWindow_release\0".as_ptr() as *const i8);
                if !release_sym.is_null() {
                    let release_fn: unsafe extern "C" fn(*mut std::ffi::c_void) =
                        std::mem::transmute(release_sym);
                    release_fn(prev_anw);
                }
            }
            if jsurface.is_null() || width <= 0 || height <= 0 {
                return 0;
            }
            let from_surface_sym = dlsym(
                lib,
                b"ANativeWindow_fromSurface\0".as_ptr() as *const i8,
            );
            if !from_surface_sym.is_null() {
                let from_surface: unsafe extern "C" fn(
                    *mut std::ffi::c_void,
                    *mut std::ffi::c_void,
                ) -> *mut std::ffi::c_void = std::mem::transmute(from_surface_sym);
                let anw = from_surface(env, jsurface);
                if !anw.is_null() {
                    ACTIVE_ANW.store(anw, Ordering::SeqCst);
                    let ok = iguanodon_attach_android_surface(
                        harness,
                        anw,
                        width as u32,
                        height as u32,
                    );
                    return if ok { 1 } else { 0 };
                }
            }
        }
    }
    0
}

/// Renders a frame directly to the attached hardware Vulkan swapchain (`VkSwapchainKHR`)
/// and presents it via `vkQueuePresentKHR`.
///
/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeRenderAndPresent(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    dt_sec: f32,
    width: i32,
    height: i32,
) -> f32 {
    if handle == 0 || width <= 0 || height <= 0 {
        return 0.0;
    }
    iguanodon_render_and_present(
        handle as *mut IguanodonTestHarness,
        dt_sec,
        width as u32,
        height as u32,
    )
}

/// Renders a frame and copies the resulting RGBA8888 pixels directly into an Android/JVM `byte[]` (`jbyteArray`).
///
/// # Safety
/// `env` must be a valid `JNIEnv*` and `out_rgba_array` a valid `jbyteArray` of at least `width * height * 4` bytes.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeRenderFrameByteArray(
    env: *mut *const *const std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    dt_sec: f32,
    width: i32,
    height: i32,
    out_rgba_array: *mut std::ffi::c_void,
) -> f32 {
    if handle == 0 || width <= 0 || height <= 0 {
        return 0.0;
    }
    let harness = &mut *(handle as *mut IguanodonTestHarness);
    let w = (width as u32).min(MAX_FRAMEBUFFER_DIM);
    let h = (height as u32).min(MAX_FRAMEBUFFER_DIM);
    let sample = harness.render_frame_rgba(dt_sec, w, h);
    if !env.is_null() && !(*env).is_null() && !out_rgba_array.is_null() {
        let vtable = *env;
        let get_array_len: unsafe extern "C" fn(
            *mut *const *const std::ffi::c_void,
            *mut std::ffi::c_void,
        ) -> i32 = std::mem::transmute(*vtable.add(171));
        let set_byte_array_region: unsafe extern "C" fn(
            *mut *const *const std::ffi::c_void,
            *mut std::ffi::c_void,
            i32,
            i32,
            *const u8,
        ) = std::mem::transmute(*vtable.add(208));

        let arr_len = get_array_len(env, out_rgba_array).max(0) as usize;
        let copy_len = arr_len.min(harness.framebuffer_rgba.len());
        if copy_len > 0 {
            set_byte_array_region(
                env,
                out_rgba_array,
                0,
                copy_len as i32,
                harness.framebuffer_rgba.as_ptr(),
            );
        }
    }
    sample.total_frame_ms
}

/// Polls queued reverse UI events from the Rust engine into a direct memory buffer.
///
/// # Safety
/// `handle` must be a valid engine handle and `direct_buf_ptr` valid for `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativePollEventsDirect(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    direct_buf_ptr: i64,
    capacity: i32,
) -> i32 {
    iguanodon_engine_poll_events(
        handle as *mut IguanodonTestHarness,
        direct_buf_ptr as *mut u8,
        capacity.max(0) as usize,
    ) as i32
}

/// Polls queued reverse UI events from the Rust engine directly into an Android/JVM `byte[]` (`jbyteArray`).
///
/// # Safety
/// `env` must be a valid `JNIEnv*` and `out_byte_array` a valid `jbyteArray`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativePollEvents(
    env: *mut *const *const std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    out_byte_array: *mut std::ffi::c_void,
) -> i32 {
    if handle == 0 || env.is_null() || (*env).is_null() || out_byte_array.is_null() {
        return 0;
    }
    let harness = &mut *(handle as *mut IguanodonTestHarness);
    if harness.pending_reverse_events.is_empty() {
        return 0;
    }
    let vtable = *env;
    let get_array_len: unsafe extern "C" fn(
        *mut *const *const std::ffi::c_void,
        *mut std::ffi::c_void,
    ) -> i32 = std::mem::transmute(*vtable.add(171));
    let set_byte_array_region: unsafe extern "C" fn(
        *mut *const *const std::ffi::c_void,
        *mut std::ffi::c_void,
        i32,
        i32,
        *const u8,
    ) = std::mem::transmute(*vtable.add(208));

    let capacity = (get_array_len(env, out_byte_array).max(0) as usize).min(MAX_FFI_BATCH_BYTES);
    if capacity < ReverseUiEvent::BYTE_SIZE {
        return 0;
    }
    let mut temp = vec![0_u8; capacity];
    let written = harness.drain_reverse_events(&mut temp);
    if written > 0 {
        set_byte_array_region(
            env,
            out_byte_array,
            0,
            written as i32,
            temp.as_ptr(),
        );
    }
    written as i32
}

/// # Safety
/// `handle` must be a valid engine handle returned by `nativeCreate`.
#[no_mangle]
pub unsafe extern "C" fn Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeDestroy(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) {
    iguanodon_engine_destroy(handle as *mut IguanodonTestHarness);
}

#[cfg(test)]
mod tests {
    use super::*;
    use iguanodon_widgets::event_mask;

    #[test]
    fn test_batched_binary_ffi_roundtrip_all_opcodes() {
        let mut encoder = BinaryCommandEncoder::new();
        encoder.encode_set_theme(ThemeId::LinuxEliteObsidian);
        encoder.encode_set_label_text(bench_nodes::HEADLINE_LABEL.index, "KMP Compose -> Rust Vulkan");
        encoder.encode_click_node(bench_nodes::PRIMARY_BTN.index);
        encoder.encode_set_text_field("Kotlin @Composable Input");
        encoder.encode_scroll_node(bench_nodes::LAZY_COLUMN.index, 1800.0);
        encoder.encode_set_progress(0.88);
        encoder.encode_drag_and_drop(
            bench_nodes::DRAGGABLE_CARD_B.index,
            bench_nodes::DROP_TARGET_BUCKET.index,
            "KMP-Drag-Payload",
        );
        encoder.encode_right_click_node(bench_nodes::CONTEXT_MENU_REGION.index);
        encoder.encode_invoke_context_menu(1); // Toggle fire shader

        let mut harness = IguanodonTestHarness::new(1280.0, 800.0);
        let applied = apply_commands_to_harness(&mut harness, &encoder.bytes).unwrap();
        assert_eq!(applied, 9);

        assert_eq!(harness.app.active_theme_id, ThemeId::LinuxEliteObsidian);
        assert_eq!(harness.app.headline_text, "KMP Compose -> Rust Vulkan");
        assert_eq!(harness.app.button_click_count, 1);
        assert_eq!(harness.app.text_field.text, "Kotlin @Composable Input");
        assert!(harness.app.lazy_scroll_y >= 1700.0);
        assert_eq!(harness.app.dropped_items, vec!["KMP-Drag-Payload"]);
        assert!(!harness.app.context_menu.is_open);
    }

    #[test]
    fn test_applier_scene_opcodes_0x10_to_0x1a_and_reverse_event_polling() {
        let mut encoder = BinaryCommandEncoder::new();
        // 0x10 INSERT_CHILD: root(1) -> row(100), row(100) -> btn_a(101), btn_b(102)
        encoder.encode_insert_child(1, 0, 100, 1);
        encoder.encode_insert_child(100, 0, 101, 2);
        encoder.encode_insert_child(100, 1, 102, 2);

        // 0x12 SET_LAYOUT_STYLE on row(100) and btn_a(101)
        encoder.encode_set_layout_style(
            100,
            &ApplierLayoutStyle {
                flex_dir: 1, // Row
                align: 1,
                justify: 0,
                pad_l: 12.0,
                pad_t: 12.0,
                pad_r: 12.0,
                pad_b: 12.0,
                gap: 8.0,
                width: 600.0,
                height: 80.0,
                min_w: 48.0,
                max_w: 1200.0,
                weight: 0.0,
            },
        );
        encoder.encode_set_layout_style(
            101,
            &ApplierLayoutStyle {
                flex_dir: 0,
                align: 1,
                justify: 1,
                pad_l: 8.0,
                pad_t: 8.0,
                pad_r: 8.0,
                pad_b: 8.0,
                gap: 0.0,
                width: 180.0,
                height: 52.0,
                min_w: 48.0,
                max_w: 400.0,
                weight: 0.0,
            },
        );

        // 0x13 SET_VISUAL_STYLE & 0x14 SET_SHADER_EFFECT
        encoder.encode_set_visual_style(
            101,
            &ApplierVisualStyle {
                bg_rgba: 0xFF6750A4,
                border_rgba: 0xFFD0BCFF,
                border_w: 1.5,
                corner_radius: 14.0,
                clip: 1,
            },
        );
        encoder.encode_set_shader_effect(
            101,
            &ApplierShaderEffect {
                shader_id: 2, // ProceduralFire
                p0: 1.0,
                p1: 0.85,
                p2: 0.0,
                p3: 0.0,
            },
        );

        // 0x15 SET_TEXT, 0x16 SET_IMAGE_BYTES, 0x17 SET_SCROLL_STATE, 0x18 SET_EVENT_MASK
        encoder.encode_set_text(
            101,
            &ApplierTextPayload {
                color_rgba: 0xFFFFFFFF,
                font_size_sp: 15.0,
                font_weight: 500,
                max_lines: 1,
                text: "Pure Compose Runtime Button".into(),
            },
        );
        encoder.encode_set_image_bytes(102, 2, 2, &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]);
        encoder.encode_set_scroll_state(100, 1, 24.0);
        encoder.encode_set_event_mask(101, event_mask::CLICK | event_mask::HOVER | event_mask::DRAG);

        // 0x11 MOVE_CHILDREN: swap children in row(100), then swap back
        encoder.encode_move_children(100, 0, 1, 1);
        encoder.encode_move_children(100, 1, 0, 1);

        let mut harness = IguanodonTestHarness::new(1280.0, 800.0);
        let applied = apply_commands_to_harness(&mut harness, &encoder.bytes).unwrap();
        assert_eq!(applied, 13);

        let node_101 = harness.applier_scene.nodes.get(&101).unwrap();
        assert_eq!(
            node_101.text.as_ref().unwrap().text,
            "Pure Compose Runtime Button"
        );
        assert!(node_101.bounds.width >= 48.0 && node_101.bounds.height >= 48.0);

        // Simulate pointer hover + click + drag on node 101 and poll reverse events via C-ABI
        let center = iguanodon_core::Point::new(
            node_101.bounds.x + node_101.bounds.width * 0.5,
            node_101.bounds.y + node_101.bounds.height * 0.5,
        );
        harness.pending_reverse_events.clear();
        harness.on_pointer_move(center);
        harness.on_pointer_down(center, false);
        harness.on_pointer_move(iguanodon_core::Point::new(center.x + 18.0, center.y + 6.0));
        harness.on_pointer_up(iguanodon_core::Point::new(center.x + 18.0, center.y + 6.0));

        let mut raw_events = vec![0_u8; 512];
        let written = unsafe {
            iguanodon_engine_poll_events(&mut harness, raw_events.as_mut_ptr(), raw_events.len())
        };
        assert!(written >= ReverseUiEvent::BYTE_SIZE * 3);
        let mut decoded_events = Vec::new();
        for chunk in raw_events[..written].chunks_exact(ReverseUiEvent::BYTE_SIZE) {
            decoded_events.push(ReverseUiEvent::from_bytes(chunk).unwrap());
        }
        assert!(decoded_events
            .iter()
            .any(|e| e.event_type == ReverseEventType::HoverEnter && e.node_id == 101));
        assert!(decoded_events
            .iter()
            .any(|e| e.event_type == ReverseEventType::Click && e.node_id == 101));
        assert!(decoded_events
            .iter()
            .any(|e| e.event_type == ReverseEventType::DragDelta && e.node_id == 101));

        // 0x19 REMOVE_CHILDREN & 0x1A CLEAR_TREE
        let mut cleanup = BinaryCommandEncoder::new();
        cleanup.encode_remove_children(100, 1, 1); // removes 102
        assert_eq!(apply_commands_to_harness(&mut harness, &cleanup.bytes).unwrap(), 1);
        assert!(!harness.applier_scene.nodes.contains_key(&102));

        cleanup.clear();
        cleanup.encode_clear_tree();
        assert_eq!(apply_commands_to_harness(&mut harness, &cleanup.bytes).unwrap(), 1);
        assert_eq!(harness.applier_scene.nodes.len(), 1);
    }

    #[test]
    fn test_android_and_desktop_jni_lifecycle_and_touch_gestures() {
        unsafe {
            let handle = Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeCreate(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1080.0,
                720.0,
            );
            assert_ne!(handle, 0);

            // Simulate Kotlin Multiplatform `@Composable StandardBenchmarkApp` flushing initial state
            let mut encoder = BinaryCommandEncoder::new();
            encoder.encode_set_theme(ThemeId::Material3Expressive);
            encoder.encode_set_label_text(
                bench_nodes::HEADLINE_LABEL.index,
                "Iguanodon Android & Desktop KMP App",
            );
            let flushed =
                Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeFlushDirectBuffer(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    handle,
                    encoder.bytes.as_ptr() as i64,
                    encoder.bytes.len() as i32,
                );
            assert_eq!(flushed, 2);

            // Simulate Android touch-scroll on LazyColumn
            let harness = &*(handle as *const IguanodonTestHarness);
            let lazy_pt = harness.node_center(bench_nodes::LAZY_COLUMN);
            Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativePointerDown(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                handle,
                lazy_pt.x,
                lazy_pt.y,
                0,
            );
            Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativePointerMove(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                handle,
                lazy_pt.x,
                lazy_pt.y - 120.0,
            );
            Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativePointerUp(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                handle,
                lazy_pt.x,
                lazy_pt.y - 120.0,
            );
            assert!(
                harness.app.lazy_scroll_y >= 100.0,
                "Touch-panning up on LazyColumn should scroll list down"
            );

            // Simulate Android long-press on CONTEXT_MENU_REGION to open Frosted Glass Context Menu
            let ctx_pt = harness.node_center(bench_nodes::CONTEXT_MENU_REGION);
            Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativePointerDown(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                handle,
                ctx_pt.x,
                ctx_pt.y,
                1,
            );
            assert!(harness.app.context_menu.is_open);

            // Render frame into RGBA buffer (as used by ANativeWindow / SurfaceView)
            let mut rgba = vec![0_u8; 320 * 200 * 4];
            let frame_ms =
                Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeRenderFrameRgba(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    handle,
                    1.0 / 120.0,
                    320,
                    200,
                    rgba.as_mut_ptr() as i64,
                );
            assert!(frame_ms >= 0.0);
            assert!(rgba.iter().any(|&b| b != 0));

            Java_iguanodon_compose_bridge_IguanodonNativeBridge_nativeDestroy(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                handle,
            );
        }
    }
}
