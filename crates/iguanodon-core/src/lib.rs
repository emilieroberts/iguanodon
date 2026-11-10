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

#![forbid(unsafe_code)]

//! Core scene primitives, analytical 120Hz spring physics, interactive state machines
//! (text editing, drag-and-drop, context menus), and performance telemetry for Iguanodon.

use std::collections::VecDeque;

/// RGBA color in linear or sRGB space `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self:: rgba(0.0, 0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgba(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgba(0.0, 0.0, 0.0, 1.0);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let b = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let a = (hex & 0xFF) as f32 / 255.0;
        Self { r, g, b, a }
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, pt: Point) -> bool {
        pt.x >= self.x
            && pt.x <= self.x + self.width
            && pt.y >= self.y
            && pt.y <= self.y + self.height
    }

    pub fn intersect(&self, other: &Rect) -> Rect {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);
        Rect::new(x1, y1, (x2 - x1).max(0.0), (y2 - y1).max(0.0))
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
}

/// Second-order spring specification aligned with Jetpack Compose Material 3 Expressive `MotionScheme`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringSpec {
    /// Damping ratio ($\zeta$): `< 1.0` bouncy/expressive, `== 1.0` critically damped, `> 1.0` overdamped.
    pub damping_ratio: f32,
    /// Stiffness ($k$): higher values snap faster.
    pub stiffness: f32,
}

impl SpringSpec {
    pub const fn new(damping_ratio: f32, stiffness: f32) -> Self {
        Self {
            damping_ratio,
            stiffness,
        }
    }
}

/// Material 3 Expressive dual-spring motion scheme (Spatial vs. Effects).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionScheme {
    pub fast_spatial: SpringSpec,
    pub default_spatial: SpringSpec,
    pub slow_spatial: SpringSpec,
    pub fast_effects: SpringSpec,
    pub default_effects: SpringSpec,
    pub slow_effects: SpringSpec,
}

impl MotionScheme {
    /// Compose 1.9+ Material 3 Expressive default spring tokens.
    pub const fn expressive() -> Self {
        Self {
            fast_spatial: SpringSpec::new(0.6, 800.0),
            default_spatial: SpringSpec::new(0.65, 380.0),
            slow_spatial: SpringSpec::new(0.7, 200.0),
            fast_effects: SpringSpec::new(1.0, 3800.0),
            default_effects: SpringSpec::new(1.0, 1600.0),
            slow_effects: SpringSpec::new(1.0, 800.0),
        }
    }

    /// Crisp critically-damped motion scheme (Windows Fluent 2 / Linux Elite).
    pub const fn crisp_snappy() -> Self {
        Self {
            fast_spatial: SpringSpec::new(1.0, 1400.0),
            default_spatial: SpringSpec::new(0.95, 750.0),
            slow_spatial: SpringSpec::new(0.95, 400.0),
            fast_effects: SpringSpec::new(1.0, 4200.0),
            default_effects: SpringSpec::new(1.0, 2000.0),
            slow_effects: SpringSpec::new(1.0, 950.0),
        }
    }

    /// Fluid high-inertia iOS Cupertino spring tokens.
    pub const fn cupertino_fluid() -> Self {
        Self {
            fast_spatial: SpringSpec::new(0.78, 600.0),
            default_spatial: SpringSpec::new(0.82, 320.0),
            slow_spatial: SpringSpec::new(0.85, 180.0),
            fast_effects: SpringSpec::new(1.0, 2800.0),
            default_effects: SpringSpec::new(1.0, 1400.0),
            slow_effects: SpringSpec::new(1.0, 700.0),
        }
    }
}

/// Analytical 2nd-order harmonic oscillator spring state ticked at native refresh rate (e.g. 120Hz).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringValue {
    pub value: f32,
    pub target: f32,
    pub velocity: f32,
    pub spec: SpringSpec,
}

impl SpringValue {
    pub fn new(initial: f32, spec: SpringSpec) -> Self {
        Self {
            value: initial,
            target: initial,
            velocity: 0.0,
            spec,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn is_settled(&self) -> bool {
        (self.value - self.target).abs() < 0.001 && self.velocity.abs() < 0.01
    }

    /// Advances the spring analytically by `dt` seconds.
    pub fn step(&mut self, dt: f32) -> f32 {
        if dt <= 0.0 || self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
            return self.value;
        }

        let omega0 = self.spec.stiffness.max(1.0).sqrt();
        let zeta = self.spec.damping_ratio.max(0.05);
        let x0 = self.value - self.target;
        let v0 = self.velocity;

        if zeta < 1.0 {
            // Under-damped (expressive bounce)
            let omega_d = omega0 * (1.0 - zeta * zeta).sqrt();
            let env = (-zeta * omega0 * dt).exp();
            let cos_t = (omega_d * dt).cos();
            let sin_t = (omega_d * dt).sin();
            let c1 = x0;
            let c2 = (v0 + zeta * omega0 * x0) / omega_d;
            let x = env * (c1 * cos_t + c2 * sin_t);
            let v = env
                * ((c2 * omega_d - c1 * zeta * omega0) * cos_t
                    - (c1 * omega_d + c2 * zeta * omega0) * sin_t);
            self.value = self.target + x;
            self.velocity = v;
        } else if (zeta - 1.0).abs() < 1e-4 {
            // Critically damped
            let env = (-omega0 * dt).exp();
            let c1 = x0;
            let c2 = v0 + omega0 * x0;
            let x = (c1 + c2 * dt) * env;
            let v = (c2 - omega0 * (c1 + c2 * dt)) * env;
            self.value = self.target + x;
            self.velocity = v;
        } else {
            // Over-damped
            let sq = (zeta * zeta - 1.0).sqrt();
            let r1 = -omega0 * (zeta - sq);
            let r2 = -omega0 * (zeta + sq);
            let c2 = (v0 - r1 * x0) / (r2 - r1);
            let c1 = x0 - c2;
            let e1 = (r1 * dt).exp();
            let e2 = (r2 * dt).exp();
            self.value = self.target + c1 * e1 + c2 * e2;
            self.velocity = c1 * r1 * e1 + c2 * r2 * e2;
        }

        if self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
        }
        self.value
    }
}

/// First-class GPU visual effect modifiers supported by Iguanodon nodes.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectModifier {
    FrostedGlass {
        blur_radius: f32,
        tint: Color,
        noise_intensity: f32,
        refraction_index: f32,
    },
    WaterRipple {
        damping: f32,
        frequency: f32,
        phase: f32,
        epicenter: Point,
    },
    FireEmitter {
        intensity: f32,
        turbulence: f32,
        temperature: f32,
    },
    DropShadow {
        blur_radius: f32,
        offset: Point,
        color: Color,
    },
    RevealHighlight {
        radius: f32,
        border_boost: f32,
    },
    CustomShader {
        shader_name: String,
        wgsl_source: String,
        uniform_params: [f32; 4],
    },
}

/// Generational Node identifier in the retained scene graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub index: u32,
    pub generation: u32,
}

impl NodeId {
    pub const ROOT: Self = Self {
        index: 0,
        generation: 0,
    };

    pub const fn from_raw(index: u32) -> Self {
        Self {
            index,
            generation: 0,
        }
    }
}

/// UTF-8 safe single/multi-line editable text state machine for `TextField` widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct TextEditState {
    pub text: String,
    pub placeholder: String,
    /// Byte offset of cursor (always on a valid UTF-8 char boundary).
    pub cursor_byte: usize,
    /// Optional byte offset of selection anchor when selecting text.
    pub selection_anchor: Option<usize>,
    pub focused: bool,
    pub caret_blink_phase: f32,
}

impl TextEditState {
    pub fn new(initial: impl Into<String>, placeholder: impl Into<String>) -> Self {
        let text = initial.into();
        let cursor_byte = text.len();
        Self {
            text,
            placeholder: placeholder.into(),
            cursor_byte,
            selection_anchor: None,
            focused: false,
            caret_blink_phase: 0.0,
        }
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.and_then(|anchor| {
            if anchor == self.cursor_byte {
                None
            } else {
                Some((anchor.min(self.cursor_byte), anchor.max(self.cursor_byte)))
            }
        })
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.selection_range().map(|(s, e)| &self.text[s..e])
    }

    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            self.text.replace_range(start..end, "");
            self.cursor_byte = start;
            self.selection_anchor = None;
            true
        } else {
            false
        }
    }

    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor_byte, s);
        self.cursor_byte += s.len();
        self.caret_blink_phase = 0.0;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor_byte > 0 {
            if let Some((prev_idx, _)) = self.text[..self.cursor_byte].char_indices().next_back() {
                self.text.replace_range(prev_idx..self.cursor_byte, "");
                self.cursor_byte = prev_idx;
            }
        }
    }

    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor_byte < self.text.len() {
            if let Some(ch) = self.text[self.cursor_byte..].chars().next() {
                let next_idx = self.cursor_byte + ch.len_utf8();
                self.text.replace_range(self.cursor_byte..next_idx, "");
            }
        }
    }

    pub fn move_left(&mut self, extend_selection: bool) {
        self.update_anchor(extend_selection);
        if !extend_selection {
            if let Some((start, _)) = self.selection_range() {
                self.cursor_byte = start;
                self.selection_anchor = None;
                return;
            }
        }
        if self.cursor_byte > 0 {
            if let Some((prev_idx, _)) = self.text[..self.cursor_byte].char_indices().next_back() {
                self.cursor_byte = prev_idx;
            }
        }
    }

    pub fn move_right(&mut self, extend_selection: bool) {
        self.update_anchor(extend_selection);
        if !extend_selection {
            if let Some((_, end)) = self.selection_range() {
                self.cursor_byte = end;
                self.selection_anchor = None;
                return;
            }
        }
        if self.cursor_byte < self.text.len() {
            if let Some(ch) = self.text[self.cursor_byte..].chars().next() {
                self.cursor_byte += ch.len_utf8();
            }
        }
    }

    pub fn move_home(&mut self, extend_selection: bool) {
        self.update_anchor(extend_selection);
        self.cursor_byte = 0;
    }

    pub fn move_end(&mut self, extend_selection: bool) {
        self.update_anchor(extend_selection);
        self.cursor_byte = self.text.len();
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor_byte = self.text.len();
    }

    fn update_anchor(&mut self, extend_selection: bool) {
        if extend_selection {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_byte);
            }
        }
    }
}

/// Active drag session and drop target state machine.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveDrag {
    pub source_node: NodeId,
    pub payload_id: String,
    pub payload_label: String,
    pub origin_pos: Point,
    pub current_pos: Point,
    pub hovered_target: Option<NodeId>,
}

#[derive(Debug, Clone, Default)]
pub struct DragDropManager {
    pub active_drag: Option<ActiveDrag>,
    pub completed_drops: Vec<(String, NodeId)>,
}

impl DragDropManager {
    pub fn start_drag(
        &mut self,
        source_node: NodeId,
        payload_id: impl Into<String>,
        payload_label: impl Into<String>,
        pos: Point,
    ) {
        self.active_drag = Some(ActiveDrag {
            source_node,
            payload_id: payload_id.into(),
            payload_label: payload_label.into(),
            origin_pos: pos,
            current_pos: pos,
            hovered_target: None,
        });
    }

    pub fn update_drag(&mut self, pos: Point, hovered_target: Option<NodeId>) {
        if let Some(drag) = &mut self.active_drag {
            drag.current_pos = pos;
            drag.hovered_target = hovered_target;
        }
    }

    /// Commits the drop if hovering over a valid target; returns `(payload_id, target_node)` on success.
    pub fn complete_drop(&mut self) -> Option<(String, NodeId)> {
        let drag = self.active_drag.take()?;
        let target = drag.hovered_target?;
        let record = (drag.payload_id, target);
        self.completed_drops.push(record.clone());
        Some(record)
    }

    pub fn cancel_drag(&mut self) {
        self.active_drag = None;
    }
}

/// Right-click context menu item definition and popup state controller.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenuItem {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ContextMenuManager {
    pub is_open: bool,
    pub anchor_pos: Point,
    pub target_node: Option<NodeId>,
    pub items: Vec<ContextMenuItem>,
    pub hovered_index: Option<usize>,
    pub last_invoked_action: Option<String>,
}

impl ContextMenuManager {
    pub fn open(&mut self, pos: Point, target_node: Option<NodeId>, items: Vec<ContextMenuItem>) {
        self.is_open = true;
        self.anchor_pos = pos;
        self.target_node = target_node;
        self.items = items;
        self.hovered_index = if self.items.is_empty() { None } else { Some(0) };
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.hovered_index = None;
        self.target_node = None;
    }

    pub fn select_next(&mut self) {
        if !self.is_open || self.items.is_empty() {
            return;
        }
        let next = match self.hovered_index {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        };
        self.hovered_index = Some(next);
    }

    pub fn select_prev(&mut self) {
        if !self.is_open || self.items.is_empty() {
            return;
        }
        let prev = match self.hovered_index {
            Some(0) | None => self.items.len() - 1,
            Some(i) => i - 1,
        };
        self.hovered_index = Some(prev);
    }

    pub fn invoke_index(&mut self, index: usize) -> Option<String> {
        if !self.is_open {
            return None;
        }
        let item = self.items.get(index)?;
        if !item.enabled {
            return None;
        }
        let action_id = item.id.clone();
        self.last_invoked_action = Some(action_id.clone());
        self.close();
        Some(action_id)
    }

    pub fn invoke_hovered(&mut self) -> Option<String> {
        let idx = self.hovered_index?;
        self.invoke_index(idx)
    }
}

/// Per-frame timing sample for the Built-In Performance Telemetry Indicator (`PerfOverlay`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameSample {
    pub total_frame_ms: f32,
    pub ffi_decode_us: f32,
    pub layout_us: f32,
    pub sdf_pack_us: f32,
    pub gpu_execution_us: f32,
    pub materialized_nodes: usize,
    pub virtualized_items: usize,
    pub draw_calls: usize,
}

/// Rolling 120-frame performance telemetry collector for the benchmark HUD.
#[derive(Debug, Clone)]
pub struct PerfTelemetry {
    pub history: VecDeque<FrameSample>,
    pub max_samples: usize,
    pub vram_allocated_bytes: usize,
    pub gpu_device_name: String,
}

impl Default for PerfTelemetry {
    fn default() -> Self {
        Self::new(120)
    }
}

impl PerfTelemetry {
    pub fn new(max_samples: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_samples),
            max_samples: max_samples.max(1),
            vram_allocated_bytes: 0,
            gpu_device_name: "Vulkan GPU".to_string(),
        }
    }

    pub fn record_frame(&mut self, sample: FrameSample) {
        if self.history.len() >= self.max_samples {
            self.history.pop_front();
        }
        self.history.push_back(sample);
    }

    pub fn avg_frame_ms(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.history.iter().map(|s| s.total_frame_ms).sum();
        sum / (self.history.len() as f32)
    }

    pub fn avg_fps(&self) -> f32 {
        let ms = self.avg_frame_ms();
        if ms <= 0.0001 {
            0.0
        } else {
            1000.0 / ms
        }
    }

    /// Computes 1% low FPS (from worst 99th percentile frame time).
    pub fn one_percent_low_fps(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let mut times: Vec<f32> = self.history.iter().map(|s| s.total_frame_ms).collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let worst_ms = *times.last().unwrap_or(&0.0);
        if worst_ms <= 0.0001 {
            0.0
        } else {
            1000.0 / worst_ms
        }
    }

    pub fn sparkline_ms(&self) -> Vec<f32> {
        self.history.iter().map(|s| s.total_frame_ms).collect()
    }

    pub fn latest(&self) -> FrameSample {
        self.history.back().copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expressive_spring_convergence() {
        let scheme = MotionScheme::expressive();
        let mut spring = SpringValue::new(0.0, scheme.fast_spatial);
        spring.set_target(100.0);

        let mut max_val = 0.0_f32;
        for _ in 0..120 {
            let v = spring.step(1.0 / 120.0);
            max_val = max_val.max(v);
        }
        // Under-damped M3 Expressive spatial spring overshoots slightly before settling at 100.0
        assert!(max_val > 100.5, "Expected expressive spring overshoot, got {max_val}");
        assert!(spring.is_settled(), "Spring should settle within 1 second at 120Hz");
        assert!((spring.value - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_text_edit_utf8_and_selection() {
        let mut edit = TextEditState::new("Hello", "Enter text...");
        edit.insert_str(" 🦖 World");
        assert_eq!(edit.text, "Hello 🦖 World");

        // Move left across 'World' (5 chars) + ' ' (1) + '🦖' (4-byte emoji)
        for _ in 0..6 {
            edit.move_left(false);
        }
        edit.backspace(); // Deletes the dinosaur emoji cleanly on UTF-8 boundary
        assert_eq!(edit.text, "Hello  World");

        edit.select_all();
        assert_eq!(edit.selected_text(), Some("Hello  World"));
        edit.insert_str("Iguanodon");
        assert_eq!(edit.text, "Iguanodon");
    }

    #[test]
    fn test_drag_and_drop_lifecycle() {
        let mut dnd = DragDropManager::default();
        let src = NodeId::from_raw(10);
        let dst = NodeId::from_raw(25);

        dnd.start_drag(src, "card-42", "Shader Asset #42", Point::new(20.0, 20.0));
        dnd.update_drag(Point::new(150.0, 220.0), Some(dst));
        let dropped = dnd.complete_drop();
        assert_eq!(dropped, Some(("card-42".to_string(), dst)));
        assert_eq!(dnd.completed_drops.len(), 1);
    }

    #[test]
    fn test_context_menu_lifecycle() {
        let mut menu = ContextMenuManager::default();
        menu.open(
            Point::new(300.0, 400.0),
            Some(NodeId::from_raw(5)),
            vec![
                ContextMenuItem {
                    id: "copy".into(),
                    label: "Copy".into(),
                    shortcut: Some("Ctrl+C".into()),
                    enabled: true,
                },
                ContextMenuItem {
                    id: "duplicate".into(),
                    label: "Duplicate".into(),
                    shortcut: Some("Ctrl+D".into()),
                    enabled: true,
                },
            ],
        );
        assert!(menu.is_open);
        menu.select_next();
        assert_eq!(menu.hovered_index, Some(1));
        let action = menu.invoke_hovered();
        assert_eq!(action.as_deref(), Some("duplicate"));
        assert!(!menu.is_open);
    }

    #[test]
    fn test_perf_telemetry_metrics() {
        let mut perf = PerfTelemetry::new(120);
        for i in 0..60 {
            perf.record_frame(FrameSample {
                total_frame_ms: if i == 10 { 8.0 } else { 4.0 },
                ffi_decode_us: 45.0,
                layout_us: 180.0,
                sdf_pack_us: 90.0,
                gpu_execution_us: 320.0,
                materialized_nodes: 48,
                virtualized_items: 10_000,
                draw_calls: 2,
            });
        }
        assert!(perf.avg_fps() > 240.0);
        assert!((perf.one_percent_low_fps() - 125.0).abs() < 0.1);
        assert_eq!(perf.sparkline_ms().len(), 60);
    }
}
