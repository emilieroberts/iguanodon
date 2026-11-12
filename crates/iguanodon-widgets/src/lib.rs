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

//! Declarative & retained widget suite, the canonical **Standard Benchmark Sample App**,
//! and the deterministic **Headless UI Test Harness (`IguanodonTestHarness`)**.

use iguanodon_core::{
    Color, ContextMenuItem, ContextMenuManager, DragDropManager, FrameSample, NodeId,
    PerfTelemetry, Point, Rect, SpringSpec, SpringValue, TextEditState,
};
use iguanodon_layout::{
    Dimension, FlexDirection, LayoutEngine, LayoutStyle, LazyListLayout, ScrollbarMetrics,
    MIN_TOUCH_TARGET_DP,
};
use iguanodon_shaders::{
    pack_material_and_border, BorderShaderEffect, ShaderMaterialId, SdfPrimitiveInstance,
};
use iguanodon_text::{
    draw_text_rgba, draw_text_sized_rgba, CachedFontSet, MultilineTextLayout,
};
use iguanodon_theme::{InteractionShaderKind, ThemeDescriptor, ThemeId, ThemeMode};
pub use iguanodon_vk::{GpuImageTexture, VulkanRenderer};
use std::time::Instant;

/// Emits GPU glyph atlas quad instances (`ShaderMaterialId::GlyphAtlas`) for `text` at `pos`
/// clipped to `clip`.
pub fn push_gpu_text_glyphs(
    out: &mut Vec<SdfPrimitiveInstance>,
    pos: Point,
    text: &str,
    color: Color,
    clip: Rect,
    is_headline: bool,
) {
    let col = color.to_array();
    let clip_arr = clip.to_array();
    let mat = ShaderMaterialId::GlyphAtlas as u32 as f32;
    let mut quads = Vec::with_capacity(text.len());
    CachedFontSet::global().layout_glyph_quads(pos, text, clip, is_headline, &mut quads);
    for q in quads {
        out.push(SdfPrimitiveInstance {
            rect: q.rect,
            corner_radii: q.uv_rect,
            clip_rect: clip_arr,
            fill_color_start: col,
            fill_color_end: col,
            border_color: [0.0; 4],
            shadow_color: [0.0; 4],
            params: [0.0, 0.0, 2.0, mat],
        });
    }
}

pub mod catalog;
pub use catalog::*;

/// Power management profile for balancing 120Hz visual fidelity against battery life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PowerMode {
    /// 120 FPS target (`8.33 ms` frame budget), full volumetric Smoke/Fire border halos, and 1.0x shader clock.
    Performance = 0,
    /// 30 FPS target (`33.33 ms` frame budget), zero exterior border halo overdraw, reduced shader tick rate, and idle sleep.
    PowerSaving = 1,
}

impl PowerMode {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::PowerSaving,
            _ => Self::Performance,
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Performance => Self::PowerSaving,
            Self::PowerSaving => Self::Performance,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Performance => "Perf 120FPS",
            Self::PowerSaving => "Eco 30FPS",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Performance => "Perf",
            Self::PowerSaving => "Eco",
        }
    }

    pub fn target_fps(self) -> u32 {
        match self {
            Self::Performance => 120,
            Self::PowerSaving => 30,
        }
    }

    pub fn target_frame_interval_sec(self) -> f32 {
        1.0 / (self.target_fps() as f32)
    }
}

/// Active UI renderer backend in the test/benchmark application (`Iguanodon` default vs `ComposeUi` reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RendererBackend {
    /// Default Iguanodon Vulkan 1.3 SDF superellipse + volumetric shader + water-drop wallpaper engine.
    Iguanodon = 0,
    /// Stock Compose-UI (Material 3 circular-arc + flat tonal surfaces) comparison mode.
    ComposeUi = 1,
}

impl RendererBackend {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::ComposeUi,
            _ => Self::Iguanodon,
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Iguanodon => Self::ComposeUi,
            Self::ComposeUi => Self::Iguanodon,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Iguanodon => "Iguanodon",
            Self::ComposeUi => "Compose-UI",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Iguanodon => "Iguanodon",
            Self::ComposeUi => "ComposeUI",
        }
    }
}

/// Canonical Node IDs for the Standard Benchmark Sample App so both unit tests,
/// FFI bridges, and user interactions can address widgets deterministically.
pub mod bench_nodes {
    use iguanodon_core::NodeId;
    pub const ROOT: NodeId = NodeId::from_raw(1);
    pub const TOP_BAR: NodeId = NodeId::from_raw(2);
    pub const PERF_OVERLAY: NodeId = NodeId::from_raw(3);
    pub const THEME_BTN: NodeId = NodeId::from_raw(4);
    pub const MAIN_ROW: NodeId = NodeId::from_raw(5);
    pub const LEFT_COL: NodeId = NodeId::from_raw(6);
    pub const MID_COL: NodeId = NodeId::from_raw(7);
    pub const RIGHT_COL: NodeId = NodeId::from_raw(8);
    pub const MODE_BTN: NodeId = NodeId::from_raw(9);

    // 1. Labels & 2. Buttons
    pub const HEADLINE_LABEL: NodeId = NodeId::from_raw(10);
    pub const STATUS_LABEL: NodeId = NodeId::from_raw(11);
    pub const PRIMARY_BTN: NodeId = NodeId::from_raw(12);
    pub const FIRE_SHADER_BTN: NodeId = NodeId::from_raw(13);
    pub const POWER_BTN: NodeId = NodeId::from_raw(14);
    pub const RENDERER_BTN: NodeId = NodeId::from_raw(15);

    // 3. Scrollable Text
    pub const SCROLLABLE_TEXT: NodeId = NodeId::from_raw(20);

    // 4. Lazy-Loaded List with Scrollbars
    pub const LAZY_COLUMN: NodeId = NodeId::from_raw(30);

    // 5. Progress Bars (Determinate & Indeterminate)
    pub const PROGRESS_DETERMINATE: NodeId = NodeId::from_raw(40);
    pub const PROGRESS_INDETERMINATE: NodeId = NodeId::from_raw(41);

    // 6. Drag & Drop Source and Target
    pub const DRAGGABLE_CARD_A: NodeId = NodeId::from_raw(50);
    pub const DRAGGABLE_CARD_B: NodeId = NodeId::from_raw(51);
    pub const DROP_TARGET_BUCKET: NodeId = NodeId::from_raw(52);

    // 7. Edit Text Entry
    pub const EDIT_TEXT_FIELD: NodeId = NodeId::from_raw(60);

    // 8. Right-Click Context Menu Region & Popup
    pub const CONTEXT_MENU_REGION: NodeId = NodeId::from_raw(70);
    pub const CONTEXT_MENU_POPUP: NodeId = NodeId::from_raw(71);

    // 9. Image Widget
    pub const SHOWCASE_IMAGE: NodeId = NodeId::from_raw(80);

    // 10. Animated Spring & Shader Playground Card + Animation Speed Slider
    pub const ANIMATED_SHADER_CARD: NodeId = NodeId::from_raw(90);
    pub const ANIM_SPEED_SLIDER: NodeId = NodeId::from_raw(95);
}

/// Complete state & scene controller for the **Standard Benchmark Sample App**.
/// Exercises all 11 required UI, animation, and performance telemetry features.
pub struct StandardBenchmarkApp {
    pub viewport: Rect,
    /// Edge-to-edge safe-area insets `[left, top, right, bottom]` in logical dp
    /// (populated by Android `WindowInsets` for status bar, camera cutout, and gesture navigation bar).
    pub safe_insets: [f32; 4],
    pub active_theme_id: ThemeId,
    pub active_theme_mode: ThemeMode,
    pub power_mode: PowerMode,
    pub renderer_backend: RendererBackend,
    pub current_theme: ThemeDescriptor,
    pub target_theme: ThemeDescriptor,
    pub theme_transition: SpringValue,

    pub layout: LayoutEngine,
    pub pointer_pos: Point,
    pub time_sec: f32,

    // 1. Labels & 2. Buttons state (including Jello Wobble click springs)
    pub headline_text: String,
    pub status_label_text: String,
    pub button_click_count: u32,
    pub fire_shader_enabled: bool,
    pub button_press_spring: SpringValue,
    pub button_jello_spring: SpringValue,
    pub shader_btn_jello_spring: SpringValue,

    // 3. Scrollable Text state (with elastic overspring bounce)
    pub scrollable_document: String,
    pub scrollable_text_scroll_y: f32,
    pub scrollable_text_overscroll_spring: SpringValue,
    pub scrollable_text_layout: MultilineTextLayout,
    pub scrollable_text_scrollbar: ScrollbarMetrics,

    // 4. Lazy-Loaded List (10,000 items) with Scrollbars & Elastic Overspring
    pub lazy_total_items: usize,
    pub lazy_scroll_y: f32,
    pub lazy_overscroll_spring: SpringValue,
    pub lazy_layout: LazyListLayout,
    pub selected_lazy_item: Option<usize>,

    // 5. Progress Bars (Determinate & Indeterminate)
    pub progress_spring: SpringValue,
    pub indeterminate_phase: f32,

    // 6. Drag-and-Drop Source & Target
    pub drag_drop: DragDropManager,
    pub dropped_items: Vec<String>,
    pub drop_target_glow_spring: SpringValue,

    // 7. Edit Text Entry (`TextField`)
    pub text_field: TextEditState,

    // 8. Right-Click Context Menu
    pub context_menu: ContextMenuManager,

    // 9. Image Widget
    pub image_texture_id: u32,
    pub image_water_ripple_active: bool,

    // 10. Animation Playground (120Hz Spatial Spring + Multi-Shader Showcase + Speed Slider)
    pub anim_card_expanded: bool,
    pub anim_card_scale_spring: SpringValue,
    pub anim_card_corner_spring: SpringValue,
    pub active_showcase_shader: ShaderMaterialId,
    pub showcase_shader_idx: usize,
    /// User-adjustable animation speed multiplier (`0.0..=2.0`, default `0.65` for calm motion).
    pub animation_speed: f32,

    // 11. Built-In Performance Telemetry Indicator (`PerfOverlay`)
    pub telemetry: PerfTelemetry,
}

impl Default for StandardBenchmarkApp {
    fn default() -> Self {
        Self::new(1280.0, 800.0)
    }
}

impl StandardBenchmarkApp {
    pub fn new(width: f32, height: f32) -> Self {
        let theme = ThemeDescriptor::material3_expressive();
        let doc = concat!(
            "Iguanodon Vulkan UI Toolkit — Scrollable Document Log\n",
            "• Zero-jank ahead-of-time SPIR-V shader compilation via naga.\n",
            "• Instanced 128-byte SDF Uber-Shader supporting circular arcs, iOS continuous squircles, and Linux Elite chamfers.\n",
            "• Material 3 Expressive 120Hz spatial & effects spring physics engine.\n",
            "• Dual-Kawase frosted glass backdrop blur, procedural SDF-contour fire, and 2D wave-equation water ripples.\n",
            "• O(1) virtualized LazyColumn handling 10,000+ items with proportional interactive scrollbars.\n",
            "• Full Drag-and-Drop, UTF-8 TextField editing, and Z-ordered right-click Context Menu.\n",
            "• Compatible with Linux, Android, Windows, and Apple MoltenVK / KosmicKrisp portability subset."
        )
        .to_string();

        let text_layout = MultilineTextLayout::shape(&doc, 18.0, 1.35, 340.0);
        let scroll_viewport = Rect::new(0.0, 0.0, 360.0, 160.0);
        let text_sb = ScrollbarMetrics::compute(scroll_viewport, text_layout.total_height, 0.0);

        let lazy_vp = Rect::new(0.0, 0.0, 360.0, 680.0);
        let lazy_layout =
            LazyListLayout::compute(lazy_vp, 10_000, MIN_TOUCH_TARGET_DP, 4.0, 0.0, 2);

        let mut app = Self {
            viewport: Rect::new(0.0, 0.0, width, height),
            safe_insets: [0.0; 4],
            active_theme_id: ThemeId::Material3Expressive,
            active_theme_mode: ThemeMode::Dark,
            power_mode: PowerMode::Performance,
            renderer_backend: RendererBackend::Iguanodon,
            current_theme: theme,
            target_theme: theme,
            theme_transition: SpringValue::new(1.0, theme.motion.fast_effects),
            layout: LayoutEngine::default(),
            pointer_pos: Point::new(width * 0.5, height * 0.5),
            time_sec: 0.0,

            headline_text: "Iguanodon Studio — Standard Benchmark App".to_string(),
            status_label_text: "Ready (Material 3 Expressive)".to_string(),
            button_click_count: 0,
            fire_shader_enabled: true,
            button_press_spring: SpringValue::new(0.0, theme.motion.fast_spatial),
            button_jello_spring: SpringValue::new(0.0, SpringSpec::new(0.22, 680.0)),
            shader_btn_jello_spring: SpringValue::new(0.0, SpringSpec::new(0.22, 680.0)),

            scrollable_document: doc,
            scrollable_text_scroll_y: 0.0,
            scrollable_text_overscroll_spring: SpringValue::new(0.0, SpringSpec::new(0.52, 420.0)),
            scrollable_text_layout: text_layout,
            scrollable_text_scrollbar: text_sb,

            lazy_total_items: 10_000,
            lazy_scroll_y: 0.0,
            lazy_overscroll_spring: SpringValue::new(0.0, SpringSpec::new(0.52, 420.0)),
            lazy_layout,
            selected_lazy_item: None,

            progress_spring: SpringValue::new(0.35, theme.motion.default_spatial),
            indeterminate_phase: 0.0,

            drag_drop: DragDropManager::default(),
            dropped_items: Vec::new(),
            drop_target_glow_spring: SpringValue::new(0.0, theme.motion.fast_effects),

            text_field: TextEditState::new(
                "Filter 10,000 Vulkan items...",
                "Type to search or edit...",
            ),

            context_menu: ContextMenuManager::default(),

            image_texture_id: 1,
            image_water_ripple_active: true,

            anim_card_expanded: false,
            anim_card_scale_spring: SpringValue::new(1.0, theme.motion.default_spatial),
            anim_card_corner_spring: SpringValue::new(
                theme.shapes.radius_md,
                theme.motion.default_spatial,
            ),
            active_showcase_shader: ShaderMaterialId::GentleSmoke,
            showcase_shader_idx: 1,
            animation_speed: 0.65,

            telemetry: PerfTelemetry::new(120),
        };
        app.rebuild_layout();
        app
    }

    /// Sets the shader animation speed multiplier (`0.0x` to `2.0x`).
    pub fn set_animation_speed(&mut self, speed: f32) {
        self.animation_speed = speed.clamp(0.0, 2.0);
        self.status_label_text = format!("Animation Speed: {:.2}x", self.animation_speed);
    }

    /// Switches to a new theme and smoothly interpolates colors, corner exponents, and springs at 120Hz.
    pub fn set_theme(&mut self, theme_id: ThemeId) {
        self.set_theme_and_mode(theme_id, self.active_theme_mode);
    }

    /// Switches polarity between `ThemeMode::Dark` and `ThemeMode::Light` for the active theme.
    pub fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.set_theme_and_mode(self.active_theme_id, mode);
    }

    /// Switches both [`ThemeId`] and [`ThemeMode`] (`Dark` / `Light`) and smoothly interpolates at 120Hz.
    pub fn set_theme_and_mode(&mut self, theme_id: ThemeId, mode: ThemeMode) {
        self.active_theme_id = theme_id;
        self.active_theme_mode = mode;
        self.target_theme = ThemeDescriptor::by_id_and_mode(theme_id, mode);
        self.theme_transition = SpringValue::new(0.0, self.target_theme.motion.default_effects);
        self.theme_transition.set_target(1.0);
        self.anim_card_corner_spring
            .set_target(self.target_theme.shapes.radius_md);
        self.status_label_text = format!(
            "Active Theme: {} ({})",
            theme_id.name(),
            mode.name()
        );
    }

    /// Toggles between `ThemeMode::Dark` and `ThemeMode::Light` for the current theme.
    pub fn toggle_theme_mode(&mut self) {
        let next_mode = self.active_theme_mode.toggle();
        self.set_theme_mode(next_mode);
    }

    pub fn cycle_theme(&mut self) {
        let next = match self.active_theme_id {
            ThemeId::Material3Expressive => ThemeId::WindowsFluent2,
            ThemeId::WindowsFluent2 => ThemeId::IosLiquidGlass,
            ThemeId::IosLiquidGlass => ThemeId::LinuxEliteObsidian,
            ThemeId::LinuxEliteObsidian => ThemeId::Material3Expressive,
        };
        self.set_theme(next);
    }

    /// Sets the engine power mode (`PowerMode::Performance` at 120 FPS vs `PowerMode::PowerSaving` at 30 FPS with shader optimizations).
    pub fn set_power_mode(&mut self, mode: PowerMode) {
        self.power_mode = mode;
        self.status_label_text = format!("Power Mode: {}", mode.name());
    }

    /// Toggles between `PowerMode::Performance` (120 FPS) and `PowerMode::PowerSaving` (30 FPS).
    pub fn toggle_power_mode(&mut self) {
        let next = self.power_mode.toggle();
        self.set_power_mode(next);
    }

    /// Sets the active renderer backend (`RendererBackend::Iguanodon` default vs `RendererBackend::ComposeUi`).
    pub fn set_renderer_backend(&mut self, backend: RendererBackend) {
        self.renderer_backend = backend;
        self.status_label_text = format!("Renderer: {}", backend.name());
    }

    /// Toggles between `RendererBackend::Iguanodon` and `RendererBackend::ComposeUi`.
    pub fn toggle_renderer_backend(&mut self) {
        let next = self.renderer_backend.toggle();
        self.set_renderer_backend(next);
    }

    /// Dynamically resizes the application viewport and reflows the responsive Flexbox layout
    /// (automatically switching between a 3-column Landscape layout and a single-column Portrait layout),
    /// `ScrollableText` word wrapping, and `LazyColumn` virtualization window.
    /// Enforces minimum viewport bounds so no interactive target ever drops below 48dp.
    pub fn resize(&mut self, width: f32, height: f32) {
        let (w, h) = if width < height {
            // Portrait orientation (e.g. Android phone held vertically)
            (width.max(460.0), height.max(1080.0))
        } else {
            // Landscape / Desktop orientation
            (width.max(680.0), height.max(520.0))
        };
        if (self.viewport.width - w).abs() > 0.5 || (self.viewport.height - h).abs() > 0.5 {
            self.viewport = Rect::new(0.0, 0.0, w, h);
            self.rebuild_layout();
        }
    }

    /// Updates the edge-to-edge safe-area insets (`left`, `top`, `right`, `bottom` in logical dp)
    /// from the platform window (such as Android `WindowInsets` for camera cutouts and system bars).
    /// The 1080p Water Drop Wallpaper continues to render edge-to-edge across the full `viewport`,
    /// while `ROOT` lays out interactive widgets inside the safe area.
    pub fn set_safe_insets(&mut self, left: f32, top: f32, right: f32, bottom: f32) {
        let next = [
            left.max(0.0),
            top.max(0.0),
            right.max(0.0),
            bottom.max(0.0),
        ];
        if (self.safe_insets[0] - next[0]).abs() > 0.5
            || (self.safe_insets[1] - next[1]).abs() > 0.5
            || (self.safe_insets[2] - next[2]).abs() > 0.5
            || (self.safe_insets[3] - next[3]).abs() > 0.5
        {
            self.safe_insets = next;
            self.rebuild_layout();
        }
    }

    /// Returns `true` when the active viewport is in Portrait orientation (`width < height`).
    pub fn is_portrait(&self) -> bool {
        self.viewport.width < self.viewport.height
    }

    /// Recomputes Flexbox layout, `ScrollableText` metrics, and `LazyColumn` virtualization window.
    /// Guarantees that every interactive touch/click target is at least `MIN_TOUCH_TARGET_DP` (`48.0`),
    /// supports both Landscape (3-column) and Portrait (single-column) layouts, and respects edge-to-edge safe insets.
    pub fn rebuild_layout(&mut self) {
        use bench_nodes::*;
        let vw = self.viewport.width;
        let vh = self.viewport.height;
        let inset_l = self.safe_insets[0].clamp(0.0, vw * 0.18);
        let inset_t = self.safe_insets[1].clamp(0.0, vh * 0.12);
        let inset_r = self.safe_insets[2].clamp(0.0, vw * 0.18);
        let inset_b = self.safe_insets[3].clamp(0.0, vh * 0.12);
        let safe_rect = Rect::new(
            inset_l,
            inset_t,
            (vw - inset_l - inset_r).max(440.0),
            (vh - inset_t - inset_b).max(520.0),
        );
        let is_portrait = safe_rect.width < safe_rect.height;

        if is_portrait {
            // ================================================================
            // PORTRAIT LAYOUT: Single Vertical Column with Water-Drop Gaps
            // ================================================================
            let extra_p = ((safe_rect.height - 1080.0) / 300.0).clamp(0.0, 1.5);
            self.layout.set_style(
                ROOT,
                LayoutStyle {
                    direction: FlexDirection::Column,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    padding: 10.0 + 6.0 * extra_p,
                    gap: 12.0 + 6.0 * extra_p,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                TOP_BAR,
                LayoutStyle {
                    direction: FlexDirection::Row,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(64.0),
                    padding: 8.0,
                    gap: 6.0,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                HEADLINE_LABEL,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                RENDERER_BTN,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.15).clamp(76.0, 140.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                POWER_BTN,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.14).clamp(72.0, 132.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                THEME_BTN,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.16).clamp(80.0, 155.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                MODE_BTN,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.13).clamp(68.0, 124.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                PERF_OVERLAY,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.18).clamp(88.0, 165.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_children(
                TOP_BAR,
                vec![
                    HEADLINE_LABEL,
                    RENDERER_BTN,
                    POWER_BTN,
                    THEME_BTN,
                    MODE_BTN,
                    PERF_OVERLAY,
                ],
            );

            // Stack LEFT_COL, MID_COL, RIGHT_COL vertically in a single column
            self.layout.set_style(
                MAIN_ROW,
                LayoutStyle {
                    direction: FlexDirection::Column,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    gap: 16.0 + 8.0 * extra_p,
                    ..Default::default()
                },
            );

            let btn_h = (48.0 + 2.0 * extra_p).clamp(MIN_TOUCH_TARGET_DP, 54.0);
            self.layout.set_style(
                LEFT_COL,
                LayoutStyle {
                    direction: FlexDirection::Column,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.38),
                    padding: 8.0,
                    gap: 7.0,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                STATUS_LABEL,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(24.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                PRIMARY_BTN,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(btn_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                FIRE_SHADER_BTN,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(btn_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                EDIT_TEXT_FIELD,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(btn_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                PROGRESS_DETERMINATE,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(22.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                PROGRESS_INDETERMINATE,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(14.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                SHOWCASE_IMAGE,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                ANIMATED_SHADER_CARD,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.15),
                    ..Default::default()
                },
            );
            self.layout.set_children(
                LEFT_COL,
                vec![
                    STATUS_LABEL,
                    PRIMARY_BTN,
                    FIRE_SHADER_BTN,
                    EDIT_TEXT_FIELD,
                    PROGRESS_DETERMINATE,
                    PROGRESS_INDETERMINATE,
                    SHOWCASE_IMAGE,
                    ANIMATED_SHADER_CARD,
                ],
            );

            self.layout.set_style(
                MID_COL,
                LayoutStyle {
                    direction: FlexDirection::Column,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.22),
                    padding: 8.0,
                    gap: 7.0,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                DRAGGABLE_CARD_A,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(btn_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                DRAGGABLE_CARD_B,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(btn_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                ANIM_SPEED_SLIDER,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(btn_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                DROP_TARGET_BUCKET,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                SCROLLABLE_TEXT,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.25),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                CONTEXT_MENU_REGION,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(0.95),
                    ..Default::default()
                },
            );
            self.layout.set_children(
                MID_COL,
                vec![
                    DRAGGABLE_CARD_A,
                    DRAGGABLE_CARD_B,
                    ANIM_SPEED_SLIDER,
                    DROP_TARGET_BUCKET,
                    SCROLLABLE_TEXT,
                    CONTEXT_MENU_REGION,
                ],
            );

            self.layout.set_style(
                RIGHT_COL,
                LayoutStyle {
                    direction: FlexDirection::Column,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(0.65),
                    padding: 8.0,
                    gap: 6.0,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                LAZY_COLUMN,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_children(RIGHT_COL, vec![LAZY_COLUMN]);
        } else {
            // ================================================================
            // LANDSCAPE LAYOUT: 3 Side-by-Side Columns with Wide Water-Drop Gaps
            // ================================================================
            let scale_v = (safe_rect.height / 760.0).clamp(0.85, 1.45);
            let scale_h = (safe_rect.width / 1180.0).clamp(0.80, 1.80);
            let extra_v = ((safe_rect.height - 520.0) / 240.0).clamp(0.0, 1.5);

            self.layout.set_style(
                ROOT,
                LayoutStyle {
                    direction: FlexDirection::Column,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    padding: 8.0 + 16.0 * extra_v,
                    gap: 6.0 + 16.0 * extra_v,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                TOP_BAR,
                LayoutStyle {
                    direction: FlexDirection::Row,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(64.0 + 6.0 * extra_v),
                    padding: 8.0,
                    gap: 8.0,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                HEADLINE_LABEL,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                RENDERER_BTN,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.14).clamp(84.0, 168.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                POWER_BTN,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.13).clamp(80.0, 156.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                THEME_BTN,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.16).clamp(96.0, 200.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                MODE_BTN,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.11).clamp(72.0, 136.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                PERF_OVERLAY,
                LayoutStyle {
                    width: Dimension::Px((safe_rect.width * 0.22).clamp(132.0, 290.0)),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_children(
                TOP_BAR,
                vec![
                    HEADLINE_LABEL,
                    RENDERER_BTN,
                    POWER_BTN,
                    THEME_BTN,
                    MODE_BTN,
                    PERF_OVERLAY,
                ],
            );

            // Wide horizontal gap between LEFT_COL, MID_COL, and RIGHT_COL so the water drop wallpaper shines through
            self.layout.set_style(
                MAIN_ROW,
                LayoutStyle {
                    direction: FlexDirection::Row,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    gap: (40.0 * scale_h).clamp(24.0, 72.0),
                    ..Default::default()
                },
            );

            // Left Column: All interactive targets (Buttons, TextField) >= 48dp
            let btn_h = (48.0 + 4.0 * extra_v).clamp(MIN_TOUCH_TARGET_DP, 60.0);
            let col_pad = 8.0 + 4.0 * extra_v;
            let col_gap = 6.0 + 4.0 * extra_v;
            self.layout.set_style(
                LEFT_COL,
                LayoutStyle {
                    direction: FlexDirection::Column,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    padding: col_pad,
                    gap: col_gap,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                STATUS_LABEL,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(26.0 + 4.0 * extra_v),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                PRIMARY_BTN,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(btn_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                FIRE_SHADER_BTN,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(btn_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                EDIT_TEXT_FIELD,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(btn_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                PROGRESS_DETERMINATE,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(22.0 + 3.0 * extra_v),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                PROGRESS_INDETERMINATE,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(16.0 + 2.0 * extra_v),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                SHOWCASE_IMAGE,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.05),
                    ..Default::default()
                },
            );
            let anim_weight = if self.anim_card_expanded { 1.35 } else { 1.05 };
            self.layout.set_style(
                ANIMATED_SHADER_CARD,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(anim_weight),
                    ..Default::default()
                },
            );
            self.layout.set_children(
                LEFT_COL,
                vec![
                    STATUS_LABEL,
                    PRIMARY_BTN,
                    FIRE_SHADER_BTN,
                    EDIT_TEXT_FIELD,
                    PROGRESS_DETERMINATE,
                    PROGRESS_INDETERMINATE,
                    SHOWCASE_IMAGE,
                    ANIMATED_SHADER_CARD,
                ],
            );

            // Center Column: Draggable cards, Animation Speed Slider, Drop Target Bucket, Scrollable Text, Context Menu Region (all >= 48dp)
            let card_h = (50.0 * scale_v).clamp(MIN_TOUCH_TARGET_DP, 60.0);
            self.layout.set_style(
                MID_COL,
                LayoutStyle {
                    direction: FlexDirection::Column,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    padding: col_pad,
                    gap: col_gap,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                DRAGGABLE_CARD_A,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(card_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                DRAGGABLE_CARD_B,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(card_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                ANIM_SPEED_SLIDER,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Px(card_h),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                DROP_TARGET_BUCKET,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(0.90),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                SCROLLABLE_TEXT,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.30),
                    ..Default::default()
                },
            );
            self.layout.set_style(
                CONTEXT_MENU_REGION,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(0.75),
                    ..Default::default()
                },
            );
            self.layout.set_children(
                MID_COL,
                vec![
                    DRAGGABLE_CARD_A,
                    DRAGGABLE_CARD_B,
                    ANIM_SPEED_SLIDER,
                    DROP_TARGET_BUCKET,
                    SCROLLABLE_TEXT,
                    CONTEXT_MENU_REGION,
                ],
            );

            // Right Column: 10,000-item LazyColumn with proportional Scrollbar (each item >= 48dp)
            self.layout.set_style(
                RIGHT_COL,
                LayoutStyle {
                    direction: FlexDirection::Column,
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    padding: col_pad,
                    gap: col_gap,
                    ..Default::default()
                },
            );
            self.layout.set_style(
                LAZY_COLUMN,
                LayoutStyle {
                    width: Dimension::Fill(1.0),
                    height: Dimension::Fill(1.0),
                    ..Default::default()
                },
            );
            self.layout.set_children(RIGHT_COL, vec![LAZY_COLUMN]);
        }

        self.layout
            .set_children(MAIN_ROW, vec![LEFT_COL, MID_COL, RIGHT_COL]);
        self.layout.set_children(ROOT, vec![TOP_BAR, MAIN_ROW]);
        self.layout.compute_layout(ROOT, safe_rect);

        // Update ScrollableText (21px Roboto font) & LazyColumn (48dp item height) viewport metrics
        if let Some(st_rect) = self.layout.bounds(SCROLLABLE_TEXT) {
            self.scrollable_text_layout = MultilineTextLayout::shape(
                &self.scrollable_document,
                21.0,
                1.35,
                (st_rect.width - 24.0).max(80.0),
            );
            self.scrollable_text_scrollbar = ScrollbarMetrics::compute(
                st_rect,
                self.scrollable_text_layout.total_height,
                self.scrollable_text_scroll_y,
            );
            self.scrollable_text_scroll_y = self.scrollable_text_scrollbar.clamped_scroll_y;
        }

        if let Some(lazy_rect) = self.layout.bounds(LAZY_COLUMN) {
            self.lazy_layout = LazyListLayout::compute(
                lazy_rect,
                self.lazy_total_items,
                MIN_TOUCH_TARGET_DP,
                4.0,
                self.lazy_scroll_y,
                2,
            );
            self.lazy_scroll_y = self.lazy_layout.scrollbar.clamped_scroll_y;
        }
    }

    /// Advances all 120Hz springs, continuous shader clocks (scaled by `animation_speed` and `power_mode`), and caret blink timers by `dt` seconds.
    pub fn step_animations(&mut self, dt: f32) {
        let power_scale = if self.power_mode == PowerMode::PowerSaving {
            0.35
        } else {
            1.0
        };
        self.time_sec += dt * self.animation_speed * power_scale;
        self.indeterminate_phase =
            (self.indeterminate_phase + dt * 0.32 * self.animation_speed * power_scale).fract();
        self.text_field.caret_blink_phase = (self.text_field.caret_blink_phase + dt * 2.0).fract();

        let t = self.theme_transition.step(dt);
        self.current_theme = self.current_theme.lerp(&self.target_theme, t);

        self.button_press_spring.step(dt);
        self.button_jello_spring.step(dt);
        self.shader_btn_jello_spring.step(dt);
        self.scrollable_text_overscroll_spring.step(dt);
        self.lazy_overscroll_spring.step(dt);
        self.progress_spring.step(dt);
        self.drop_target_glow_spring.step(dt);
        self.anim_card_scale_spring.step(dt);
        self.anim_card_corner_spring.step(dt);
    }

    /// Returns `true` if any spatial/effects spring is currently in flight or a drag gesture is active.
    pub fn has_active_spring_or_drag(&self) -> bool {
        !self.theme_transition.is_settled()
            || !self.button_press_spring.is_settled()
            || !self.button_jello_spring.is_settled()
            || !self.shader_btn_jello_spring.is_settled()
            || !self.scrollable_text_overscroll_spring.is_settled()
            || !self.lazy_overscroll_spring.is_settled()
            || !self.progress_spring.is_settled()
            || !self.drop_target_glow_spring.is_settled()
            || !self.anim_card_scale_spring.is_settled()
            || !self.anim_card_corner_spring.is_settled()
            || self.drag_drop.active_drag.is_some()
    }

    /// Computes volume-preserving jello squash-and-stretch bounds (`[x, y, width, height]`)
    /// from an underdamped spring displacement (`jello_val`).
    #[inline]
    pub fn apply_jello_wobble(r: Rect, jello_val: f32) -> [f32; 4] {
        let sx = (1.0 + jello_val * 0.085).clamp(0.82, 1.22);
        let sy = (1.0 - jello_val * 0.14).clamp(0.78, 1.24);
        let w = r.width * sx;
        let h = r.height * sy;
        [
            r.x + (r.width - w) * 0.5,
            r.y + (r.height - h) * 0.5,
            w,
            h,
        ]
    }

    /// Packs the entire visible scene (including virtualized `LazyColumn` items, scrollbars,
    /// floating drag-and-drop ghost preview, right-click frosted-glass context menu, and
    /// live performance sparkline) into 128-byte `SdfPrimitiveInstance`s for Vulkan rendering.
    pub fn build_sdf_instances(&self) -> Vec<SdfPrimitiveInstance> {
        use bench_nodes::*;
        let mut out = Vec::with_capacity(96);
        let th = &self.current_theme;
        let is_compose_ui = self.renderer_backend == RendererBackend::ComposeUi;
        let is_eco = self.power_mode == PowerMode::PowerSaving;
        let p = if is_compose_ui {
            2.0
        } else {
            th.shapes.shape_exponent_p
        };
        let full_clip = self.viewport.to_array();
        let filter_border = |b: BorderShaderEffect| -> BorderShaderEffect {
            if is_compose_ui || is_eco {
                BorderShaderEffect::None
            } else {
                b
            }
        };

        if is_compose_ui {
            // In Compose-UI comparison mode, render a flat Material 3 background surface over the wallpaper
            let bg = th.colors.background.with_alpha(1.0).to_array();
            out.push(SdfPrimitiveInstance {
                rect: full_clip,
                corner_radii: [0.0; 4],
                clip_rect: full_clip,
                fill_color_start: bg,
                fill_color_end: bg,
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                params: [0.0, 0.0, 2.0, ShaderMaterialId::StandardSdf as u32 as f32],
            });
        }

        let base_material = if is_compose_ui {
            ShaderMaterialId::StandardSdf
        } else {
            match th.interaction_shader {
                InteractionShaderKind::MaterialRipple => ShaderMaterialId::StandardSdf,
                InteractionShaderKind::FluentRevealHighlight => {
                    ShaderMaterialId::FluentRevealHighlight
                }
                InteractionShaderKind::CupertinoLiquidSpecular => ShaderMaterialId::FrostedGlass,
                InteractionShaderKind::ObsidianNeonPulse => ShaderMaterialId::ObsidianNeonPulse,
            }
        };

        // Top Bar & 3 Column Containers (demonstrating Gentle Smoke borders on TOP_BAR & MID_COL, and Fire border on LEFT_COL)
        let is_dark_mode = self.active_theme_mode == ThemeMode::Dark;
        let panel_alpha = if is_compose_ui {
            1.0
        } else if is_dark_mode {
            0.36
        } else {
            0.32
        };
        let elem_alpha = if is_compose_ui {
            1.0
        } else if is_dark_mode {
            0.50
        } else {
            0.46
        };
        let btn_alpha = if is_compose_ui {
            1.0
        } else if is_dark_mode {
            0.58
        } else {
            0.54
        };
        let subtle_glass_mat = if is_compose_ui {
            ShaderMaterialId::StandardSdf
        } else if base_material == ShaderMaterialId::ObsidianNeonPulse {
            ShaderMaterialId::ObsidianNeonPulse
        } else if base_material == ShaderMaterialId::FluentRevealHighlight {
            ShaderMaterialId::FluentRevealHighlight
        } else {
            ShaderMaterialId::SubtleGlassReflection
        };
        let panel_material = if is_compose_ui {
            ShaderMaterialId::StandardSdf
        } else if base_material == ShaderMaterialId::ObsidianNeonPulse {
            ShaderMaterialId::ObsidianNeonPulse
        } else {
            ShaderMaterialId::SubtleGlassReflection
        };
        for &panel in &[TOP_BAR, LEFT_COL, MID_COL, RIGHT_COL] {
            if let Some(r) = self.layout.bounds(panel) {
                let raw_border = if panel == LEFT_COL {
                    BorderShaderEffect::FireBorder
                } else if panel == TOP_BAR || panel == MID_COL {
                    BorderShaderEffect::GentleSmokeBorder
                } else {
                    BorderShaderEffect::None
                };
                let panel_border_effect = filter_border(raw_border);
                out.push(SdfPrimitiveInstance {
                    rect: r.to_array(),
                    corner_radii: [th.shapes.radius_lg; 4],
                    clip_rect: full_clip,
                    fill_color_start: th.colors.surface.with_alpha(panel_alpha).to_array(),
                    fill_color_end: th
                        .colors
                        .surface_container
                        .with_alpha(panel_alpha)
                        .to_array(),
                    border_color: th.colors.outline.to_array(),
                    shadow_color: [0.0, 0.0, 0.0, 0.45],
                    params: [
                        th.shapes.border_width.max(1.5),
                        if is_eco { 4.0 } else { th.optics.shadow_elevation_px },
                        p,
                        pack_material_and_border(panel_material, panel_border_effect),
                    ],
                });
            }
        }

        // 1. Labels & 2. Buttons (including Renderer Backend & Power Mode toggles in TOP_BAR)
        for &top_btn in &[RENDERER_BTN, POWER_BTN, THEME_BTN, MODE_BTN] {
            if let Some(r) = self.layout.bounds(top_btn) {
                let is_active_toggle = (top_btn == POWER_BTN && is_eco)
                    || (top_btn == RENDERER_BTN && !is_compose_ui);
                let fill_start = if is_active_toggle {
                    th.colors.primary_container.with_alpha(btn_alpha).to_array()
                } else {
                    th.colors.surface_container.with_alpha(elem_alpha).to_array()
                };
                out.push(SdfPrimitiveInstance {
                    rect: r.to_array(),
                    corner_radii: [th.shapes.radius_sm; 4],
                    clip_rect: full_clip,
                    fill_color_start: fill_start,
                    fill_color_end: th.colors.surface.with_alpha(elem_alpha).to_array(),
                    border_color: th.colors.accent_glow.to_array(),
                    shadow_color: [0.0, 0.0, 0.0, 0.35],
                    params: [
                        th.shapes.border_width,
                        if is_eco { 2.0 } else { 6.0 },
                        p,
                        pack_material_and_border(
                            subtle_glass_mat,
                            filter_border(BorderShaderEffect::GentleSmokeBorder),
                        ),
                    ],
                });
            }
        }

        if let Some(r) = self.layout.bounds(PRIMARY_BTN) {
            let morph_r = th.shapes.radius_md
                * (1.0 - self.button_press_spring.value * (1.0 - th.shapes.pressed_corner_morph));
            let btn_rect = if is_compose_ui {
                r.to_array()
            } else {
                Self::apply_jello_wobble(r, self.button_jello_spring.value)
            };
            out.push(SdfPrimitiveInstance {
                rect: btn_rect,
                corner_radii: [morph_r.max(4.0); 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.primary.with_alpha(btn_alpha).to_array(),
                fill_color_end: th.colors.secondary.with_alpha(btn_alpha).to_array(),
                border_color: th.colors.accent_glow.to_array(),
                shadow_color: [0.0, 0.0, 0.0, 0.35],
                params: [
                    th.shapes.border_width.max(1.5),
                    8.0,
                    p,
                    pack_material_and_border(
                        subtle_glass_mat,
                        filter_border(BorderShaderEffect::FireBorder),
                    ),
                ],
            });
        }

        if let Some(r) = self.layout.bounds(FIRE_SHADER_BTN) {
            let mat = if self.fire_shader_enabled && !is_compose_ui && !is_eco {
                ShaderMaterialId::ProceduralFire
            } else {
                subtle_glass_mat
            };
            let btn_rect = if is_compose_ui {
                r.to_array()
            } else {
                Self::apply_jello_wobble(r, self.shader_btn_jello_spring.value)
            };
            out.push(SdfPrimitiveInstance {
                rect: btn_rect,
                corner_radii: [th.shapes.radius_md; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.primary_container.with_alpha(btn_alpha).to_array(),
                fill_color_end: th.colors.tertiary.with_alpha(btn_alpha).to_array(),
                border_color: th.colors.accent_glow.to_array(),
                shadow_color: [1.0, 0.35, 0.05, 0.5],
                params: [
                    1.5,
                    12.0,
                    p,
                    pack_material_and_border(
                        mat,
                        filter_border(BorderShaderEffect::FireBorder),
                    ),
                ],
            });
        }

        // 3. Scrollable Text + Scrollbar Track & Thumb (with Gentle Smoke Border & overspring offset)
        if let Some(r) = self.layout.bounds(SCROLLABLE_TEXT) {
            out.push(SdfPrimitiveInstance {
                rect: r.to_array(),
                corner_radii: [th.shapes.radius_sm; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.surface_container.with_alpha(elem_alpha).to_array(),
                fill_color_end: th.colors.surface.with_alpha(elem_alpha).to_array(),
                border_color: th.colors.outline.to_array(),
                shadow_color: [0.0; 4],
                params: [
                    th.shapes.border_width.max(1.5),
                    0.0,
                    p,
                    pack_material_and_border(
                        subtle_glass_mat,
                        filter_border(BorderShaderEffect::GentleSmokeBorder),
                    ),
                ],
            });
            // Scrollbar track & thumb
            let sb = &self.scrollable_text_scrollbar;
            out.push(SdfPrimitiveInstance {
                rect: sb.track_rect.to_array(),
                corner_radii: [4.0; 4],
                clip_rect: r.to_array(),
                fill_color_start: th.colors.surface_bright.with_alpha(0.35).to_array(),
                fill_color_end: th.colors.surface_bright.with_alpha(0.35).to_array(),
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                params: [0.0, 0.0, 2.0, ShaderMaterialId::StandardSdf as u32 as f32],
            });
            let mut thumb_arr = sb.thumb_rect.to_array();
            thumb_arr[1] = (thumb_arr[1] - self.scrollable_text_overscroll_spring.value * 0.25)
                .clamp(r.y + 2.0, (r.y + r.height - thumb_arr[3] - 2.0).max(r.y + 2.0));
            out.push(SdfPrimitiveInstance {
                rect: thumb_arr,
                corner_radii: [4.0; 4],
                clip_rect: r.to_array(),
                fill_color_start: th.colors.primary.with_alpha(0.75).to_array(),
                fill_color_end: th.colors.accent_glow.with_alpha(0.75).to_array(),
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                params: [0.0, 0.0, 2.0, ShaderMaterialId::StandardSdf as u32 as f32],
            });
        }

        // 4. LazyColumn (10,000 items -> only visible slice materialized!) + Scrollbar + Overspring Bounce
        if let Some(r) = self.layout.bounds(LAZY_COLUMN) {
            let overscroll_y = self.lazy_overscroll_spring.value;
            for item_idx in self.lazy_layout.visible_range.clone() {
                let mut item_r = self.lazy_layout.item_rect(r, item_idx);
                item_r.y -= overscroll_y;
                let is_selected = self.selected_lazy_item == Some(item_idx);
                let fill = if is_selected {
                    th.colors.primary_container.with_alpha(btn_alpha)
                } else {
                    th.colors.surface_bright.with_alpha(elem_alpha)
                };
                out.push(SdfPrimitiveInstance {
                    rect: item_r.to_array(),
                    corner_radii: [th.shapes.radius_sm; 4],
                    clip_rect: r.to_array(),
                    fill_color_start: fill.to_array(),
                    fill_color_end: th.colors.surface_container.with_alpha(elem_alpha).to_array(),
                    border_color: th.colors.outline.to_array(),
                    shadow_color: [0.0; 4],
                    params: [th.shapes.border_width, 0.0, p, subtle_glass_mat as u32 as f32],
                });
            }
            if overscroll_y.abs() > 1.0 {
                let glow_h = (overscroll_y.abs() * 0.35).clamp(4.0, 18.0);
                let glow_y = if overscroll_y < 0.0 {
                    r.y
                } else {
                    r.y + r.height - glow_h
                };
                out.push(SdfPrimitiveInstance {
                    rect: [r.x + 6.0, glow_y, (r.width - 24.0).max(16.0), glow_h],
                    corner_radii: [glow_h * 0.5; 4],
                    clip_rect: r.to_array(),
                    fill_color_start: th.colors.primary.with_alpha(0.65).to_array(),
                    fill_color_end: th.colors.accent_glow.with_alpha(0.65).to_array(),
                    border_color: [0.0; 4],
                    shadow_color: [0.0; 4],
                    params: [
                        0.0,
                        4.0,
                        2.0,
                        ShaderMaterialId::ColorCyclingGlowbar as u32 as f32,
                    ],
                });
            }
            let sb = &self.lazy_layout.scrollbar;
            out.push(SdfPrimitiveInstance {
                rect: sb.track_rect.to_array(),
                corner_radii: [4.0; 4],
                clip_rect: r.to_array(),
                fill_color_start: th.colors.surface_container.with_alpha(0.40).to_array(),
                fill_color_end: th.colors.surface_container.with_alpha(0.40).to_array(),
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                params: [0.0, 0.0, 2.0, ShaderMaterialId::StandardSdf as u32 as f32],
            });
            out.push(SdfPrimitiveInstance {
                rect: sb.thumb_rect.to_array(),
                corner_radii: [4.0; 4],
                clip_rect: r.to_array(),
                fill_color_start: th.colors.primary.with_alpha(0.75).to_array(),
                fill_color_end: th.colors.accent_glow.with_alpha(0.75).to_array(),
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                params: [0.0, 0.0, 2.0, ShaderMaterialId::StandardSdf as u32 as f32],
            });
        }

        // 5. Progress Bars (Determinate & Indeterminate Color-Cycling Glowbar)
        if let Some(r) = self.layout.bounds(PROGRESS_DETERMINATE) {
            out.push(SdfPrimitiveInstance {
                rect: r.to_array(),
                corner_radii: [r.height * 0.5; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.surface_bright.with_alpha(elem_alpha).to_array(),
                fill_color_end: th.colors.surface_bright.with_alpha(elem_alpha).to_array(),
                border_color: th.colors.outline.to_array(),
                shadow_color: [0.0; 4],
                params: [th.shapes.border_width, 0.0, 2.0, subtle_glass_mat as u32 as f32],
            });
            let fill_w = (r.width * self.progress_spring.value.clamp(0.0, 1.0)).max(4.0);
            out.push(SdfPrimitiveInstance {
                rect: [r.x, r.y, fill_w, r.height],
                corner_radii: [r.height * 0.5; 4],
                clip_rect: r.to_array(),
                fill_color_start: th.colors.primary.with_alpha(0.68).to_array(),
                fill_color_end: th.colors.accent_glow.with_alpha(0.68).to_array(),
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                params: [
                    0.0,
                    4.0,
                    2.0,
                    ShaderMaterialId::ColorCyclingGlowbar as u32 as f32,
                ],
            });
        }

        if let Some(r) = self.layout.bounds(PROGRESS_INDETERMINATE) {
            let seg_w = r.width * 0.35;
            let seg_x = r.x + (r.width - seg_w) * self.indeterminate_phase;
            out.push(SdfPrimitiveInstance {
                rect: [seg_x, r.y, seg_w, r.height],
                corner_radii: [r.height * 0.5; 4],
                clip_rect: r.to_array(),
                fill_color_start: th.colors.secondary.with_alpha(0.68).to_array(),
                fill_color_end: th.colors.tertiary.with_alpha(0.68).to_array(),
                border_color: th.colors.outline.to_array(),
                shadow_color: [0.0; 4],
                params: [
                    th.shapes.border_width,
                    0.0,
                    2.0,
                    ShaderMaterialId::ColorCyclingGlowbar as u32 as f32,
                ],
            });
        }

        // 6. Drag & Drop Source Cards and Drop Target Bucket (Showcasing Flowing Gradient + Fire Border, Pulsing Lights + Smoke Border, Subtle Glass Reflection + Smoke Border)
        for &card in &[DRAGGABLE_CARD_A, DRAGGABLE_CARD_B, DROP_TARGET_BUCKET] {
            if let Some(r) = self.layout.bounds(card) {
                let is_drop_bucket = card == DROP_TARGET_BUCKET;
                let border_w = if is_drop_bucket {
                    (th.shapes.border_width + self.drop_target_glow_spring.value * 2.5).max(1.5)
                } else {
                    th.shapes.border_width.max(1.5)
                };
                let (raw_mat, raw_border, fill_start, fill_end) = match card {
                    DRAGGABLE_CARD_A => (
                        ShaderMaterialId::FlowingGentleGradient,
                        BorderShaderEffect::FireBorder,
                        [0.98, 0.84, 0.22, elem_alpha + 0.08],
                        [1.0, 0.98, 0.88, elem_alpha + 0.08],
                    ),
                    DRAGGABLE_CARD_B => (
                        ShaderMaterialId::GentlePulsingLights,
                        BorderShaderEffect::GentleSmokeBorder,
                        th.colors.surface_bright.with_alpha(elem_alpha).to_array(),
                        th.colors.surface_container.with_alpha(elem_alpha).to_array(),
                    ),
                    _ => (
                        ShaderMaterialId::SubtleGlassReflection,
                        BorderShaderEffect::GentleSmokeBorder,
                        th.colors.surface_bright.with_alpha(elem_alpha).to_array(),
                        th.colors.surface_container.with_alpha(elem_alpha).to_array(),
                    ),
                };
                let card_mat = if is_compose_ui {
                    ShaderMaterialId::StandardSdf
                } else {
                    raw_mat
                };
                let border_eff = filter_border(raw_border);
                out.push(SdfPrimitiveInstance {
                    rect: r.to_array(),
                    corner_radii: [th.shapes.radius_md; 4],
                    clip_rect: full_clip,
                    fill_color_start: fill_start,
                    fill_color_end: fill_end,
                    border_color: th.colors.accent_glow.to_array(),
                    shadow_color: [0.0, 0.0, 0.0, 0.4],
                    params: [
                        border_w,
                        if is_eco { 3.0 } else { 8.0 },
                        p,
                        pack_material_and_border(card_mat, border_eff),
                    ],
                });
            }
        }

        // Animation Speed Slider (`ANIM_SPEED_SLIDER`): Interactive 0.0x..2.0x speed control with Gentle Smoke border
        if let Some(r) = self.layout.bounds(ANIM_SPEED_SLIDER) {
            out.push(SdfPrimitiveInstance {
                rect: r.to_array(),
                corner_radii: [th.shapes.radius_md; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.surface_container.with_alpha(elem_alpha).to_array(),
                fill_color_end: th.colors.surface.with_alpha(elem_alpha).to_array(),
                border_color: th.colors.accent_glow.to_array(),
                shadow_color: [0.0, 0.0, 0.0, 0.35],
                params: [
                    th.shapes.border_width.max(1.5),
                    if is_eco { 2.0 } else { 6.0 },
                    p,
                    pack_material_and_border(
                        subtle_glass_mat,
                        filter_border(BorderShaderEffect::GentleSmokeBorder),
                    ),
                ],
            });
            let track_x = r.x + 14.0;
            let track_w = (r.width - 28.0).max(24.0);
            let track_h = 8.0_f32;
            let track_y = r.y + r.height - 14.0;
            let norm = (self.animation_speed / 2.0).clamp(0.0, 1.0);
            out.push(SdfPrimitiveInstance {
                rect: [track_x, track_y, track_w, track_h],
                corner_radii: [4.0; 4],
                clip_rect: r.to_array(),
                fill_color_start: th.colors.surface_bright.with_alpha(elem_alpha).to_array(),
                fill_color_end: th.colors.surface_bright.with_alpha(elem_alpha).to_array(),
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                params: [0.0, 0.0, 2.0, ShaderMaterialId::StandardSdf as u32 as f32],
            });
            let fill_w = (track_w * norm).max(6.0);
            out.push(SdfPrimitiveInstance {
                rect: [track_x, track_y, fill_w, track_h],
                corner_radii: [4.0; 4],
                clip_rect: r.to_array(),
                fill_color_start: th.colors.primary.with_alpha(0.68).to_array(),
                fill_color_end: th.colors.accent_glow.with_alpha(0.68).to_array(),
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                params: [
                    0.0,
                    2.0,
                    2.0,
                    ShaderMaterialId::ColorCyclingGlowbar as u32 as f32,
                ],
            });
            let thumb_r = 16.0_f32;
            let thumb_x = (track_x + track_w * norm - thumb_r * 0.5)
                .clamp(track_x - 2.0, track_x + track_w - thumb_r + 2.0);
            let thumb_y = track_y + (track_h - thumb_r) * 0.5;
            out.push(SdfPrimitiveInstance {
                rect: [thumb_x, thumb_y, thumb_r, thumb_r],
                corner_radii: [thumb_r * 0.5; 4],
                clip_rect: r.to_array(),
                fill_color_start: th.colors.accent_glow.with_alpha(0.85).to_array(),
                fill_color_end: th.colors.primary.with_alpha(0.85).to_array(),
                border_color: th.colors.on_primary.to_array(),
                shadow_color: [0.0, 0.0, 0.0, 0.45],
                params: [1.5, 4.0, 2.0, ShaderMaterialId::StandardSdf as u32 as f32],
            });
        }

        // 7. Edit Text Entry (`TextField`) + Blinking Caret
        if let Some(r) = self.layout.bounds(EDIT_TEXT_FIELD) {
            let border_c = if self.text_field.focused {
                th.colors.primary
            } else {
                th.colors.outline
            };
            out.push(SdfPrimitiveInstance {
                rect: r.to_array(),
                corner_radii: [th.shapes.radius_sm; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.surface_container.with_alpha(elem_alpha).to_array(),
                fill_color_end: th.colors.surface.with_alpha(elem_alpha).to_array(),
                border_color: border_c.to_array(),
                shadow_color: [0.0; 4],
                params: [
                    if self.text_field.focused {
                        2.0
                    } else {
                        th.shapes.border_width
                    },
                    0.0,
                    p,
                    subtle_glass_mat as u32 as f32,
                ],
            });
            if self.text_field.focused && self.text_field.caret_blink_phase < 0.5 {
                let caret_x =
                    r.x + 10.0 + (self.text_field.cursor_byte.min(32) as f32) * 7.5;
                out.push(SdfPrimitiveInstance {
                    rect: [caret_x, r.y + 8.0, 2.0, r.height - 16.0],
                    corner_radii: [1.0; 4],
                    clip_rect: r.to_array(),
                    fill_color_start: th.colors.accent_glow.to_array(),
                    fill_color_end: th.colors.accent_glow.to_array(),
                    border_color: [0.0; 4],
                    shadow_color: [0.0; 4],
                    params: [0.0, 0.0, 2.0, ShaderMaterialId::StandardSdf as u32 as f32],
                });
            }
        }

        if let Some(r) = self.layout.bounds(CONTEXT_MENU_REGION) {
            out.push(SdfPrimitiveInstance {
                rect: r.to_array(),
                corner_radii: [th.shapes.radius_sm; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.surface_container.with_alpha(elem_alpha).to_array(),
                fill_color_end: th.colors.surface.with_alpha(elem_alpha).to_array(),
                border_color: th.colors.outline.to_array(),
                shadow_color: [0.0; 4],
                params: [
                    th.shapes.border_width.max(1.5),
                    0.0,
                    p,
                    pack_material_and_border(
                        subtle_glass_mat,
                        filter_border(BorderShaderEffect::GentleSmokeBorder),
                    ),
                ],
            });
        }

        // 9. Image Widget (with SDF squircle corner clipping, Water Ripple shader & Fire Border)
        if let Some(r) = self.layout.bounds(SHOWCASE_IMAGE) {
            let mat = if self.image_water_ripple_active && !is_compose_ui && !is_eco {
                ShaderMaterialId::WaterRipple
            } else {
                ShaderMaterialId::TexturedImage
            };
            out.push(SdfPrimitiveInstance {
                rect: r.to_array(),
                corner_radii: [th.shapes.radius_lg; 4],
                clip_rect: full_clip,
                fill_color_start: [0.36, 0.78, 0.96, elem_alpha],
                fill_color_end: [0.62, 0.50, 0.96, elem_alpha],
                border_color: th.colors.outline.to_array(),
                shadow_color: [0.0, 0.0, 0.0, 0.45],
                params: [
                    th.shapes.border_width.max(1.5),
                    if is_eco { 4.0 } else { 10.0 },
                    p,
                    pack_material_and_border(
                        mat,
                        filter_border(BorderShaderEffect::FireBorder),
                    ),
                ],
            });
        }

        // 10. Animated Spring & Shader Playground Card (Cycles through Gentle Smoke, Pulsing Lights, Reflective Glass, Subtle Reflection, Glowbar, Flowing Gradient, Fire, Water Ripple)
        if let Some(r) = self.layout.bounds(ANIMATED_SHADER_CARD) {
            let scale = self.anim_card_scale_spring.value;
            let corner_r = self.anim_card_corner_spring.value;
            let w = r.width * scale;
            let h = r.height * scale;
            let scaled_rect = Rect::new(
                r.x + (r.width - w) * 0.5,
                r.y + (r.height - h) * 0.5,
                w,
                h,
            );
            let card_rect = if is_compose_ui {
                scaled_rect.to_array()
            } else {
                Self::apply_jello_wobble(scaled_rect, self.shader_btn_jello_spring.value * 0.6)
            };
            let showcase_mat = if is_compose_ui {
                ShaderMaterialId::StandardSdf
            } else {
                self.active_showcase_shader
            };
            out.push(SdfPrimitiveInstance {
                rect: card_rect,
                corner_radii: [corner_r; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.primary_container.with_alpha(btn_alpha).to_array(),
                fill_color_end: th.colors.tertiary.with_alpha(btn_alpha).to_array(),
                border_color: th.colors.accent_glow.to_array(),
                shadow_color: [0.0, 0.0, 0.0, 0.5],
                params: [
                    1.5,
                    if is_eco { 4.0 } else { 14.0 },
                    p,
                    pack_material_and_border(
                        showcase_mat,
                        filter_border(BorderShaderEffect::GentleSmokeBorder),
                    ),
                ],
            });
        }

        // 11. Built-In Performance Telemetry Indicator (`PerfOverlay`)
        if let Some(r) = self.layout.bounds(PERF_OVERLAY) {
            let overlay_mat = if is_compose_ui {
                ShaderMaterialId::StandardSdf
            } else if base_material == ShaderMaterialId::ObsidianNeonPulse {
                ShaderMaterialId::ObsidianNeonPulse
            } else {
                ShaderMaterialId::SubtleGlassReflection
            };
            out.push(SdfPrimitiveInstance {
                rect: r.to_array(),
                corner_radii: [th.shapes.radius_sm; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.surface_container.with_alpha(elem_alpha).to_array(),
                fill_color_end: th.colors.background.with_alpha(elem_alpha).to_array(),
                border_color: th.colors.accent_glow.to_array(),
                shadow_color: [0.0; 4],
                params: [th.shapes.border_width, 4.0, p, overlay_mat as u32 as f32],
            });
        }

        out
    }

    /// Builds top-layer Z-ordered overlay `SdfPrimitiveInstance`s (Right-Click Context Menu Popup
    /// and Floating Drag-and-Drop Ghost Preview) that composite on top of both base widgets and
    /// base widget text overlays.
    pub fn build_overlay_sdf_instances(&self) -> Vec<SdfPrimitiveInstance> {
        let mut out = Vec::new();
        let th = &self.current_theme;
        let p = if self.renderer_backend == RendererBackend::ComposeUi {
            2.0
        } else {
            th.shapes.shape_exponent_p
        };
        let full_clip = self.viewport.to_array();
        let is_elite = matches!(
            th.interaction_shader,
            InteractionShaderKind::ObsidianNeonPulse
        );
        let popup_mat = if is_elite {
            ShaderMaterialId::ObsidianNeonPulse
        } else {
            ShaderMaterialId::FrostedGlass
        };

        if let Some(drag) = &self.drag_drop.active_drag {
            out.push(SdfPrimitiveInstance {
                rect: [
                    drag.current_pos.x - 100.0,
                    drag.current_pos.y - 24.0,
                    200.0,
                    48.0,
                ],
                corner_radii: [th.shapes.radius_md; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.primary.with_alpha(0.82).to_array(),
                fill_color_end: th.colors.secondary.with_alpha(0.86).to_array(),
                border_color: th.colors.accent_glow.to_array(),
                shadow_color: [0.0, 0.0, 0.0, 0.65],
                params: [2.0, 18.0, p, popup_mat as u32 as f32],
            });
        }

        // 8. Top-Layer Z-Ordered Right-Click Context Menu Popup (Frosted Glass, 52dp per item)
        if self.context_menu.is_open {
            let menu_w = 296.0_f32;
            let item_h = 52.0_f32.max(MIN_TOUCH_TARGET_DP);
            let menu_h = (self.context_menu.items.len() as f32 * item_h) + 12.0;
            let mx = self
                .context_menu
                .anchor_pos
                .x
                .min((self.viewport.width - menu_w - 8.0).max(0.0));
            let my = self
                .context_menu
                .anchor_pos
                .y
                .min((self.viewport.height - menu_h - 8.0).max(0.0));
            out.push(SdfPrimitiveInstance {
                rect: [mx, my, menu_w, menu_h],
                corner_radii: [th.shapes.radius_md; 4],
                clip_rect: full_clip,
                fill_color_start: th.colors.surface_bright.with_alpha(0.76).to_array(),
                fill_color_end: th.colors.surface.with_alpha(0.84).to_array(),
                border_color: th.colors.accent_glow.to_array(),
                shadow_color: [0.0, 0.0, 0.0, 0.68],
                params: [
                    th.shapes.border_width,
                    20.0,
                    p,
                    popup_mat as u32 as f32,
                ],
            });

            // Hover highlight on whichever menu item the pointer is currently over
            let menu_rect = Rect::new(mx, my, menu_w, menu_h);
            if menu_rect.contains(self.pointer_pos) {
                let rel_y = (self.pointer_pos.y - my - 6.0).max(0.0);
                let hover_idx = (rel_y / item_h).floor() as usize;
                if hover_idx < self.context_menu.items.len() {
                    let hy = my + 6.0 + (hover_idx as f32) * item_h;
                    out.push(SdfPrimitiveInstance {
                        rect: [mx + 6.0, hy + 2.0, menu_w - 12.0, item_h - 4.0],
                        corner_radii: [th.shapes.radius_sm; 4],
                        clip_rect: menu_rect.to_array(),
                        fill_color_start: th.colors.primary_container.with_alpha(0.75).to_array(),
                        fill_color_end: th.colors.primary.with_alpha(0.45).to_array(),
                        border_color: th.colors.accent_glow.to_array(),
                        shadow_color: [0.0; 4],
                        params: [1.0, 0.0, p, popup_mat as u32 as f32],
                    });
                }
            }
        }

        out
    }

    /// Builds the complete GPU instance stream for hardware Vulkan rendering:
    /// 1. Fullscreen background (`ElectricJellyfish` or `StandardSdf`)
    /// 2. Base widget `SdfPrimitiveInstance`s
    /// 3. Base widget `GlyphAtlas` typography quads
    /// 4. Top-layer overlay `SdfPrimitiveInstance`s (context menu + drag ghost)
    /// 5. Top-layer overlay `GlyphAtlas` typography quads
    /// Returns `(gpu_instances, materialized_widget_node_count)`.
    pub fn build_complete_gpu_instances(&self) -> (Vec<SdfPrimitiveInstance>, usize) {
        let base_widgets = self.build_sdf_instances();
        let overlay_widgets = self.build_overlay_sdf_instances();
        let materialized_nodes = base_widgets.len() + overlay_widgets.len();

        let mut out = Vec::with_capacity(materialized_nodes + 384);
        let bg_mat = if self.renderer_backend == RendererBackend::ComposeUi {
            ShaderMaterialId::StandardSdf
        } else {
            ShaderMaterialId::ElectricJellyfish
        };
        let bg_col = self.current_theme.colors.background.to_array();
        out.push(SdfPrimitiveInstance {
            rect: [
                0.0,
                0.0,
                self.viewport.width.max(1.0),
                self.viewport.height.max(1.0),
            ],
            corner_radii: [0.0; 4],
            clip_rect: [
                0.0,
                0.0,
                self.viewport.width.max(1.0),
                self.viewport.height.max(1.0),
            ],
            fill_color_start: bg_col,
            fill_color_end: bg_col,
            border_color: [0.0; 4],
            shadow_color: [0.0; 4],
            params: [0.0, 0.0, 2.0, bg_mat as u32 as f32],
        });

        out.extend_from_slice(&base_widgets);
        self.append_text_glyph_instances(&mut out);

        if !overlay_widgets.is_empty() {
            out.extend_from_slice(&overlay_widgets);
            self.append_overlay_text_glyph_instances(&mut out);
        }

        (out, materialized_nodes)
    }

    /// Appends GPU `GlyphAtlas` quad instances for all 11 components in the benchmark app.
    pub fn append_text_glyph_instances(&self, out: &mut Vec<SdfPrimitiveInstance>) {
        use bench_nodes::*;
        if self.viewport.width < 320.0 || self.viewport.height < 200.0 {
            return;
        }
        let th = &self.current_theme;
        let on_surf = th.colors.on_surface;
        let on_prim = th.colors.on_primary;
        let accent = th.colors.accent_glow;

        if let Some(r) = self.layout.bounds(HEADLINE_LABEL) {
            let headline = if r.width < 240.0 {
                "Iguanodon"
            } else {
                self.headline_text.as_str()
            };
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 10.0, r.y + (r.height - 26.0) * 0.5),
                headline,
                on_surf,
                r,
                true,
            );
        }
        if let Some(r) = self.layout.bounds(RENDERER_BTN) {
            let label = if r.width < 130.0 {
                self.renderer_backend.short_name().to_string()
            } else {
                format!("UI: {}", self.renderer_backend.name())
            };
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &label,
                accent,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(POWER_BTN) {
            let label = if r.width < 110.0 {
                format!("Pwr:{}", self.power_mode.short_name())
            } else {
                format!("Pwr: {}", self.power_mode.short_name())
            };
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &label,
                accent,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(THEME_BTN) {
            let label = if r.width < 145.0 {
                self.active_theme_id.short_name().to_string()
            } else {
                format!("Theme: {}", self.active_theme_id.short_name())
            };
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &label,
                accent,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(MODE_BTN) {
            let label = if r.width < 110.0 {
                self.active_theme_mode.name().to_string()
            } else {
                format!("Mode: {}", self.active_theme_mode.name())
            };
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &label,
                accent,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(PERF_OVERLAY) {
            let perf_str = if r.width < 200.0 {
                format!("{:.0} FPS", self.telemetry.avg_fps())
            } else {
                format!(
                    "FPS:{:.0} 1%:{:.0} Nodes:{}/{}",
                    self.telemetry.avg_fps(),
                    self.telemetry.one_percent_low_fps(),
                    self.telemetry.latest().materialized_nodes,
                    self.lazy_total_items
                )
            };
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &perf_str,
                accent,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(STATUS_LABEL) {
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 6.0, r.y + 4.0),
                &self.status_label_text,
                on_surf,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(PRIMARY_BTN) {
            let txt = format!("Click Action Button (Clicks: {})", self.button_click_count);
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                &txt,
                on_prim,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(FIRE_SHADER_BTN) {
            let txt = if self.fire_shader_enabled {
                "SDF Fire Shader: ON [Click]"
            } else {
                "SDF Fire Shader: OFF [Click]"
            };
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                txt,
                on_surf,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(EDIT_TEXT_FIELD) {
            let display = if self.text_field.text.is_empty() {
                &self.text_field.placeholder
            } else {
                &self.text_field.text
            };
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 12.0, r.y + (r.height - 21.0) * 0.5),
                display,
                on_surf,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(PROGRESS_DETERMINATE) {
            let pct = format!("Progress: {:.0}%", self.progress_spring.value * 100.0);
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 12.0, r.y + 2.0),
                &pct,
                on_surf,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(SHOWCASE_IMAGE) {
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 14.0, r.y + 14.0),
                "Image + Ripple + Fire Border",
                on_surf,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(ANIMATED_SHADER_CARD) {
            let shader_label = format!(
                "Shader: {} [Click]",
                self.active_showcase_shader.display_name()
            );
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                &shader_label,
                on_surf,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(DRAGGABLE_CARD_A) {
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                "Drag: Flowing Grad + Fire Border",
                on_surf,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(DRAGGABLE_CARD_B) {
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                "Drag: Pulsing + Smoke Border",
                on_surf,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(ANIM_SPEED_SLIDER) {
            let speed_txt = format!("Anim Speed: {:.2}x [Slide]", self.animation_speed);
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 14.0, r.y + 6.0),
                &speed_txt,
                accent,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(DROP_TARGET_BUCKET) {
            let label = format!("Glass + Smoke Border (Dropped: {})", self.dropped_items.len());
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 14.0, r.y + 12.0),
                &label,
                accent,
                r,
                false,
            );
            if r.height >= 64.0 {
                if let Some(last) = self.dropped_items.last() {
                    let sub = format!("Last Accepted: {}", last);
                    push_gpu_text_glyphs(
                        out,
                        Point::new(r.x + 14.0, r.y + 38.0),
                        &sub,
                        on_surf,
                        r,
                        false,
                    );
                }
            }
        }
        if let Some(r) = self.layout.bounds(SCROLLABLE_TEXT) {
            let overscroll_y = self.scrollable_text_overscroll_spring.value;
            for line in self
                .scrollable_text_layout
                .visible_lines(self.scrollable_text_scroll_y, r.height)
            {
                let ly = r.y + 8.0 + line.y_offset - self.scrollable_text_scroll_y - overscroll_y;
                push_gpu_text_glyphs(
                    out,
                    Point::new(r.x + 10.0, ly),
                    &line.text,
                    on_surf,
                    r,
                    false,
                );
            }
        }
        if let Some(r) = self.layout.bounds(CONTEXT_MENU_REGION) {
            push_gpu_text_glyphs(
                out,
                Point::new(r.x + 12.0, r.y + (r.height - 21.0) * 0.5),
                "Right-Click Here: Context Menu",
                accent,
                r,
                false,
            );
        }
        if let Some(r) = self.layout.bounds(LAZY_COLUMN) {
            let overscroll_y = self.lazy_overscroll_spring.value;
            for item_idx in self.lazy_layout.visible_range.clone() {
                let mut item_r = self.lazy_layout.item_rect(r, item_idx);
                item_r.y -= overscroll_y;
                let txt = format!("#{:04}  Lazy Vulkan Item {}", item_idx, item_idx);
                let item_color = if self.selected_lazy_item == Some(item_idx) {
                    accent
                } else {
                    on_surf
                };
                push_gpu_text_glyphs(
                    out,
                    Point::new(item_r.x + 12.0, item_r.y + (item_r.height - 21.0) * 0.5),
                    &txt,
                    item_color,
                    r.intersect(&item_r),
                    false,
                );
            }
        }
    }

    /// Appends GPU `GlyphAtlas` quad instances for top-layer overlays (context menu + drag ghost).
    pub fn append_overlay_text_glyph_instances(&self, out: &mut Vec<SdfPrimitiveInstance>) {
        if self.viewport.width < 320.0 || self.viewport.height < 200.0 {
            return;
        }
        let th = &self.current_theme;
        let on_surf = th.colors.on_surface;
        let on_prim = th.colors.on_primary;
        let accent = th.colors.accent_glow;

        if let Some(drag) = &self.drag_drop.active_drag {
            let drag_rect = Rect::new(
                drag.current_pos.x - 100.0,
                drag.current_pos.y - 24.0,
                200.0,
                48.0,
            );
            push_gpu_text_glyphs(
                out,
                Point::new(drag_rect.x + 14.0, drag_rect.y + (drag_rect.height - 21.0) * 0.5),
                &drag.payload_label,
                on_prim,
                drag_rect,
                false,
            );
        }

        if self.context_menu.is_open {
            let menu_w = 296.0_f32;
            let item_h = 52.0_f32.max(MIN_TOUCH_TARGET_DP);
            let menu_h = (self.context_menu.items.len() as f32 * item_h) + 12.0;
            let mx = self
                .context_menu
                .anchor_pos
                .x
                .min((self.viewport.width - menu_w - 8.0).max(0.0));
            let my = self
                .context_menu
                .anchor_pos
                .y
                .min((self.viewport.height - menu_h - 8.0).max(0.0));
            let menu_rect = Rect::new(mx, my, menu_w, menu_h);
            let hovered_idx = if menu_rect.contains(self.pointer_pos) {
                Some(((self.pointer_pos.y - my - 6.0).max(0.0) / item_h).floor() as usize)
            } else {
                None
            };
            for (idx, item) in self.context_menu.items.iter().enumerate() {
                let iy = my + 6.0 + (idx as f32) * item_h + (item_h - 21.0) * 0.5;
                let col = if hovered_idx == Some(idx) {
                    accent
                } else {
                    on_surf
                };
                push_gpu_text_glyphs(
                    out,
                    Point::new(mx + 14.0, iy),
                    &item.label,
                    col,
                    menu_rect,
                    false,
                );
            }
        }
    }

    /// Executes a complete frame (animations -> layout -> SDF instance packing -> Vulkan GPU pass)
    /// and records the sample into the Built-In Performance Telemetry Indicator (`PerfOverlay`).
    pub fn render_benchmark_frame(
        &mut self,
        vk: &mut VulkanRenderer,
        dt: f32,
        framebuffer_w: u32,
        framebuffer_h: u32,
        framebuffer_rgba: &mut Vec<u8>,
    ) -> FrameSample {
        let frame_start = Instant::now();
        self.step_animations(dt);

        let layout_start = Instant::now();
        self.rebuild_layout();
        let layout_us = layout_start.elapsed().as_secs_f32() * 1_000_000.0;

        let pack_start = Instant::now();
        let (gpu_instances, materialized_nodes) = self.build_complete_gpu_instances();
        let sdf_pack_us = pack_start.elapsed().as_secs_f32() * 1_000_000.0;

        let vk_stats = vk.render_frame(
            &gpu_instances,
            self.pointer_pos,
            self.time_sec,
            self.current_theme.colors.background,
            framebuffer_w,
            framebuffer_h,
            framebuffer_rgba,
        );

        // If running on a system without hardware Vulkan, fall back to CPU text/overlay blitting
        if !vk_stats.hardware_vulkan_active {
            self.render_text_overlays(framebuffer_w, framebuffer_h, framebuffer_rgba);
            let overlay_instances = self.build_overlay_sdf_instances();
            if !overlay_instances.is_empty() {
                vk.render_overlay_instances(
                    &overlay_instances,
                    self.pointer_pos,
                    self.time_sec,
                    framebuffer_w,
                    framebuffer_h,
                    framebuffer_rgba,
                );
                self.render_overlay_text(framebuffer_w, framebuffer_h, framebuffer_rgba);
            }
        }

        let total_frame_ms = frame_start.elapsed().as_secs_f32() * 1_000.0;
        let sample = FrameSample {
            total_frame_ms,
            ffi_decode_us: 12.0,
            layout_us,
            sdf_pack_us,
            gpu_execution_us: vk_stats.gpu_timestamp_us,
            materialized_nodes,
            virtualized_items: self.lazy_total_items,
            draw_calls: vk_stats.draw_calls,
        };
        self.telemetry.vram_allocated_bytes = vk_stats.vram_allocated_bytes;
        self.telemetry.gpu_device_name = vk_stats.device_name;
        self.telemetry.record_frame(sample);
        sample
    }

    /// Renders anti-aliased 18px/22px Roboto (or NotoSans Material equivalent) typography labels
    /// across all 11 components in the framebuffer.
    pub fn render_text_overlays(&self, fb_w: u32, fb_h: u32, fb_rgba: &mut [u8]) {
        use bench_nodes::*;
        if fb_w < 320 || fb_h < 200 {
            return;
        }
        let th = &self.current_theme;
        let on_surf = th.colors.on_surface;
        let on_prim = th.colors.on_primary;
        let accent = th.colors.accent_glow;

        if let Some(r) = self.layout.bounds(HEADLINE_LABEL) {
            let headline = if r.width < 240.0 {
                "Iguanodon"
            } else {
                self.headline_text.as_str()
            };
            draw_text_sized_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 10.0, r.y + (r.height - 26.0) * 0.5),
                headline,
                on_surf,
                r,
                true,
            );
        }
        if let Some(r) = self.layout.bounds(RENDERER_BTN) {
            let label = if r.width < 130.0 {
                self.renderer_backend.short_name().to_string()
            } else {
                format!("UI: {}", self.renderer_backend.name())
            };
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &label,
                accent,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(POWER_BTN) {
            let label = if r.width < 110.0 {
                format!("Pwr:{}", self.power_mode.short_name())
            } else {
                format!("Pwr: {}", self.power_mode.short_name())
            };
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &label,
                accent,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(THEME_BTN) {
            let label = if r.width < 145.0 {
                self.active_theme_id.short_name().to_string()
            } else {
                format!("Theme: {}", self.active_theme_id.short_name())
            };
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &label,
                accent,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(MODE_BTN) {
            let label = if r.width < 110.0 {
                self.active_theme_mode.name().to_string()
            } else {
                format!("Mode: {}", self.active_theme_mode.name())
            };
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &label,
                accent,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(PERF_OVERLAY) {
            let perf_str = if r.width < 200.0 {
                format!("{:.0} FPS", self.telemetry.avg_fps())
            } else {
                format!(
                    "FPS:{:.0} 1%:{:.0} Nodes:{}/{}",
                    self.telemetry.avg_fps(),
                    self.telemetry.one_percent_low_fps(),
                    self.telemetry.latest().materialized_nodes,
                    self.lazy_total_items
                )
            };
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 8.0, r.y + (r.height - 21.0) * 0.5),
                &perf_str,
                accent,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(STATUS_LABEL) {
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 6.0, r.y + 4.0),
                &self.status_label_text,
                on_surf,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(PRIMARY_BTN) {
            let txt = format!("Click Action Button (Clicks: {})", self.button_click_count);
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                &txt,
                on_prim,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(FIRE_SHADER_BTN) {
            let txt = if self.fire_shader_enabled {
                "SDF Fire Shader: ON [Click]"
            } else {
                "SDF Fire Shader: OFF [Click]"
            };
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                txt,
                on_surf,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(EDIT_TEXT_FIELD) {
            let display = if self.text_field.text.is_empty() {
                &self.text_field.placeholder
            } else {
                &self.text_field.text
            };
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 12.0, r.y + (r.height - 21.0) * 0.5),
                display,
                on_surf,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(PROGRESS_DETERMINATE) {
            let pct = format!("Progress: {:.0}%", self.progress_spring.value * 100.0);
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 12.0, r.y + 2.0),
                &pct,
                on_surf,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(SHOWCASE_IMAGE) {
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 14.0, r.y + 14.0),
                "Image + Ripple + Fire Border",
                on_surf,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(ANIMATED_SHADER_CARD) {
            let shader_label = format!(
                "Shader: {} [Click]",
                self.active_showcase_shader.display_name()
            );
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                &shader_label,
                on_surf,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(DRAGGABLE_CARD_A) {
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                "Drag: Flowing Grad + Fire Border",
                on_surf,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(DRAGGABLE_CARD_B) {
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 14.0, r.y + (r.height - 21.0) * 0.5),
                "Drag: Pulsing + Smoke Border",
                on_surf,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(ANIM_SPEED_SLIDER) {
            let speed_txt = format!("Anim Speed: {:.2}x [Slide]", self.animation_speed);
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 14.0, r.y + 6.0),
                &speed_txt,
                accent,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(DROP_TARGET_BUCKET) {
            let label = format!("Glass + Smoke Border (Dropped: {})", self.dropped_items.len());
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 14.0, r.y + 12.0),
                &label,
                accent,
                r,
            );
            if r.height >= 64.0 {
                if let Some(last) = self.dropped_items.last() {
                    let sub = format!("Last Accepted: {}", last);
                    draw_text_rgba(
                        fb_rgba,
                        fb_w,
                        fb_h,
                        Point::new(r.x + 14.0, r.y + 38.0),
                        &sub,
                        on_surf,
                        r,
                    );
                }
            }
        }
        if let Some(r) = self.layout.bounds(SCROLLABLE_TEXT) {
            let overscroll_y = self.scrollable_text_overscroll_spring.value;
            for line in self
                .scrollable_text_layout
                .visible_lines(self.scrollable_text_scroll_y, r.height)
            {
                let ly = r.y + 8.0 + line.y_offset - self.scrollable_text_scroll_y - overscroll_y;
                draw_text_rgba(
                    fb_rgba,
                    fb_w,
                    fb_h,
                    Point::new(r.x + 10.0, ly),
                    &line.text,
                    on_surf,
                    r,
                );
            }
        }
        if let Some(r) = self.layout.bounds(CONTEXT_MENU_REGION) {
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(r.x + 12.0, r.y + (r.height - 21.0) * 0.5),
                "Right-Click Here: Context Menu",
                accent,
                r,
            );
        }
        if let Some(r) = self.layout.bounds(LAZY_COLUMN) {
            let overscroll_y = self.lazy_overscroll_spring.value;
            for item_idx in self.lazy_layout.visible_range.clone() {
                let mut item_r = self.lazy_layout.item_rect(r, item_idx);
                item_r.y -= overscroll_y;
                let txt = format!("#{:04}  Lazy Vulkan Item {}", item_idx, item_idx);
                let item_color = if self.selected_lazy_item == Some(item_idx) {
                    accent
                } else {
                    on_surf
                };
                draw_text_rgba(
                    fb_rgba,
                    fb_w,
                    fb_h,
                    Point::new(item_r.x + 12.0, item_r.y + (item_r.height - 21.0) * 0.5),
                    &txt,
                    item_color,
                    r.intersect(&item_r),
                );
            }
        }
    }

    /// Renders top-layer overlay text (Right-Click Context Menu items and Floating Drag Ghost label)
    /// strictly after `render_overlay_instances` so underlying widget text never bleeds through.
    pub fn render_overlay_text(&self, fb_w: u32, fb_h: u32, fb_rgba: &mut [u8]) {
        if fb_w < 320 || fb_h < 200 {
            return;
        }
        let th = &self.current_theme;
        let on_surf = th.colors.on_surface;
        let on_prim = th.colors.on_primary;
        let accent = th.colors.accent_glow;

        if let Some(drag) = &self.drag_drop.active_drag {
            let drag_rect = Rect::new(
                drag.current_pos.x - 100.0,
                drag.current_pos.y - 24.0,
                200.0,
                48.0,
            );
            draw_text_rgba(
                fb_rgba,
                fb_w,
                fb_h,
                Point::new(drag_rect.x + 14.0, drag_rect.y + (drag_rect.height - 21.0) * 0.5),
                &drag.payload_label,
                on_prim,
                drag_rect,
            );
        }

        if self.context_menu.is_open {
            let menu_w = 296.0_f32;
            let item_h = 52.0_f32.max(MIN_TOUCH_TARGET_DP);
            let menu_h = (self.context_menu.items.len() as f32 * item_h) + 12.0;
            let mx = self
                .context_menu
                .anchor_pos
                .x
                .min((self.viewport.width - menu_w - 8.0).max(0.0));
            let my = self
                .context_menu
                .anchor_pos
                .y
                .min((self.viewport.height - menu_h - 8.0).max(0.0));
            let menu_rect = Rect::new(mx, my, menu_w, menu_h);
            let hovered_idx = if menu_rect.contains(self.pointer_pos) {
                Some(((self.pointer_pos.y - my - 6.0).max(0.0) / item_h).floor() as usize)
            } else {
                None
            };
            for (idx, item) in self.context_menu.items.iter().enumerate() {
                let iy = my + 6.0 + (idx as f32) * item_h + (item_h - 21.0) * 0.5;
                let col = if hovered_idx == Some(idx) {
                    accent
                } else {
                    on_surf
                };
                draw_text_rgba(
                    fb_rgba,
                    fb_w,
                    fb_h,
                    Point::new(mx + 14.0, iy),
                    &item.label,
                    col,
                    menu_rect,
                );
            }
        }
    }
}

/// Reverse UI event types emitted from `libiguanodon_ffi.so` back to Kotlin's `IguanodonSession.pollEvents()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReverseEventType {
    Click = 1,
    LongOrSecondaryClick = 2,
    HoverEnter = 3,
    HoverExit = 4,
    DragDelta = 5,
    ScrollDelta = 6,
    KeyChar = 7,
    KeyBackspace = 8,
    WindowResize = 9,
    SafeInsets = 10,
    ThemeModeChanged = 11,
    PowerModeChanged = 12,
    RendererBackendChanged = 13,
}

/// Fixed 32-byte binary reverse event record queued for Kotlin's `iguanodon_engine_poll_events`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverseUiEvent {
    pub event_type: ReverseEventType,
    pub flags: u8,
    pub aux_u32: u32,
    pub node_id: u64,
    pub f0: f32,
    pub f1: f32,
    pub f2: f32,
    pub f3: f32,
}

impl ReverseUiEvent {
    pub const BYTE_SIZE: usize = 32;

    pub fn to_bytes(&self) -> [u8; Self::BYTE_SIZE] {
        let mut out = [0_u8; Self::BYTE_SIZE];
        out[0] = self.event_type as u8;
        out[1] = self.flags;
        out[4..8].copy_from_slice(&self.aux_u32.to_le_bytes());
        out[8..16].copy_from_slice(&self.node_id.to_le_bytes());
        out[16..20].copy_from_slice(&self.f0.to_le_bytes());
        out[20..24].copy_from_slice(&self.f1.to_le_bytes());
        out[24..28].copy_from_slice(&self.f2.to_le_bytes());
        out[28..32].copy_from_slice(&self.f3.to_le_bytes());
        out
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() < Self::BYTE_SIZE {
            return None;
        }
        let event_type = match b[0] {
            1 => ReverseEventType::Click,
            2 => ReverseEventType::LongOrSecondaryClick,
            3 => ReverseEventType::HoverEnter,
            4 => ReverseEventType::HoverExit,
            5 => ReverseEventType::DragDelta,
            6 => ReverseEventType::ScrollDelta,
            7 => ReverseEventType::KeyChar,
            8 => ReverseEventType::KeyBackspace,
            9 => ReverseEventType::WindowResize,
            10 => ReverseEventType::SafeInsets,
            11 => ReverseEventType::ThemeModeChanged,
            12 => ReverseEventType::PowerModeChanged,
            13 => ReverseEventType::RendererBackendChanged,
            _ => return None,
        };
        let flags = b[1];
        let aux_u32 = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
        let node_id = u64::from_le_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]);
        let f0 = f32::from_le_bytes([b[16], b[17], b[18], b[19]]);
        let f1 = f32::from_le_bytes([b[20], b[21], b[22], b[23]]);
        let f2 = f32::from_le_bytes([b[24], b[25], b[26], b[27]]);
        let f3 = f32::from_le_bytes([b[28], b[29], b[30], b[31]]);
        Some(Self {
            event_type,
            flags,
            aux_u32,
            node_id,
            f0,
            f1,
            f2,
            f3,
        })
    }
}

/// Bitmask constants for `SET_EVENT_MASK` (`0x18`).
pub mod event_mask {
    pub const CLICK: u32 = 1 << 0;
    pub const LONG_CLICK: u32 = 1 << 1;
    pub const HOVER: u32 = 1 << 2;
    pub const DRAG: u32 = 1 << 3;
    pub const KEY: u32 = 1 << 4;
    pub const TEXT_INPUT: u32 = 1 << 5;
    pub const SCROLL: u32 = 1 << 6;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApplierLayoutStyle {
    pub flex_dir: u8,
    pub align: u8,
    pub justify: u8,
    pub pad_l: f32,
    pub pad_t: f32,
    pub pad_r: f32,
    pub pad_b: f32,
    pub gap: f32,
    pub width: f32,
    pub height: f32,
    pub min_w: f32,
    pub max_w: f32,
    pub weight: f32,
}

impl Default for ApplierLayoutStyle {
    fn default() -> Self {
        Self {
            flex_dir: 0,
            align: 0,
            justify: 0,
            pad_l: 0.0,
            pad_t: 0.0,
            pad_r: 0.0,
            pad_b: 0.0,
            gap: 0.0,
            width: 0.0,
            height: 0.0,
            min_w: 0.0,
            max_w: f32::MAX,
            weight: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApplierVisualStyle {
    pub bg_rgba: u32,
    pub border_rgba: u32,
    pub border_w: f32,
    pub corner_radius: f32,
    pub clip: u8,
}

impl Default for ApplierVisualStyle {
    fn default() -> Self {
        Self {
            bg_rgba: 0,
            border_rgba: 0,
            border_w: 0.0,
            corner_radius: 0.0,
            clip: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApplierShaderEffect {
    pub shader_id: u8,
    pub p0: f32,
    pub p1: f32,
    pub p2: f32,
    pub p3: f32,
}

impl Default for ApplierShaderEffect {
    fn default() -> Self {
        Self {
            shader_id: 0,
            p0: 0.0,
            p1: 0.0,
            p2: 0.0,
            p3: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplierTextPayload {
    pub color_rgba: u32,
    pub font_size_sp: f32,
    pub font_weight: u16,
    pub max_lines: u16,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplierImagePayload {
    pub width: u32,
    pub height: u32,
    pub rgba_bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ApplierNode {
    pub node_id: u64,
    pub kind: u8,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub layout: ApplierLayoutStyle,
    pub visual: ApplierVisualStyle,
    pub shader: ApplierShaderEffect,
    pub text: Option<ApplierTextPayload>,
    pub image: Option<ApplierImagePayload>,
    pub scroll_axis: u8,
    pub scroll_offset_px: f32,
    pub event_mask: u32,
    pub bounds: Rect,
}

impl ApplierNode {
    pub fn new(node_id: u64, kind: u8) -> Self {
        Self {
            node_id,
            kind,
            parent_id: None,
            children: Vec::new(),
            layout: ApplierLayoutStyle::default(),
            visual: ApplierVisualStyle::default(),
            shader: ApplierShaderEffect::default(),
            text: None,
            image: None,
            scroll_axis: 0,
            scroll_offset_px: 0.0,
            event_mask: 0,
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }
}

/// Retained scene graph managed by Kotlin's `IguanodonApplier` (`AbstractApplier<IguanodonNode>`).
#[derive(Debug, Clone)]
pub struct ApplierSceneGraph {
    pub root_id: u64,
    pub nodes: std::collections::HashMap<u64, ApplierNode>,
    pub hovered_node_id: Option<u64>,
    pub focused_node_id: Option<u64>,
    pub active_drag_node_id: Option<(u64, Point)>,
}

impl Default for ApplierSceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplierSceneGraph {
    pub fn new() -> Self {
        let mut nodes = std::collections::HashMap::new();
        nodes.insert(1, ApplierNode::new(1, 0));
        Self {
            root_id: 1,
            nodes,
            hovered_node_id: None,
            focused_node_id: None,
            active_drag_node_id: None,
        }
    }

    pub fn ensure_node(&mut self, node_id: u64, kind: u8) -> &mut ApplierNode {
        self.nodes
            .entry(node_id)
            .or_insert_with(|| ApplierNode::new(node_id, kind))
    }

    pub fn insert_child(&mut self, parent_id: u64, index: usize, child_id: u64, kind: u8) {
        {
            let child = self.ensure_node(child_id, kind);
            child.kind = kind;
            child.parent_id = Some(parent_id);
        }
        let parent = self.ensure_node(parent_id, 0);
        parent.children.retain(|&id| id != child_id);
        let idx = index.min(parent.children.len());
        parent.children.insert(idx, child_id);
    }

    pub fn move_children(&mut self, parent_id: u64, from: usize, to: usize, count: usize) {
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            if count == 0 || from >= parent.children.len() {
                return;
            }
            let end = (from + count).min(parent.children.len());
            let moved: Vec<u64> = parent.children.drain(from..end).collect();
            let dst = to.min(parent.children.len());
            for (i, id) in moved.into_iter().enumerate() {
                parent.children.insert(dst + i, id);
            }
        }
    }

    pub fn remove_children(&mut self, parent_id: u64, index: usize, count: usize) {
        let removed_ids: Vec<u64> = if let Some(parent) = self.nodes.get_mut(&parent_id) {
            if index >= parent.children.len() || count == 0 {
                Vec::new()
            } else {
                let end = (index + count).min(parent.children.len());
                parent.children.drain(index..end).collect()
            }
        } else {
            Vec::new()
        };
        for id in removed_ids {
            self.remove_subtree(id);
        }
    }

    fn remove_subtree(&mut self, node_id: u64) {
        if let Some(node) = self.nodes.remove(&node_id) {
            for child in node.children {
                self.remove_subtree(child);
            }
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.nodes.insert(1, ApplierNode::new(1, 0));
        self.hovered_node_id = None;
        self.focused_node_id = None;
        self.active_drag_node_id = None;
    }

    /// Recomputes layout bounds (`Rect`) across the retained `ApplierSceneGraph` for hit-testing.
    pub fn recompute_layout(&mut self, viewport_w: f32, viewport_h: f32) {
        let root_rect = Rect::new(0.0, 0.0, viewport_w.max(1.0), viewport_h.max(1.0));
        self.layout_subtree(self.root_id, root_rect);
    }

    fn layout_subtree(&mut self, node_id: u64, rect: Rect) {
        let (children, layout, has_event) = match self.nodes.get_mut(&node_id) {
            Some(node) => {
                let mut b = rect;
                if node.event_mask != 0 {
                    b.width = b.width.max(MIN_TOUCH_TARGET_DP);
                    b.height = b.height.max(MIN_TOUCH_TARGET_DP);
                }
                node.bounds = b;
                (node.children.clone(), node.layout, node.event_mask != 0)
            }
            None => return,
        };
        let _ = has_event;
        if children.is_empty() {
            return;
        }
        let inner_x = rect.x + layout.pad_l;
        let inner_y = rect.y + layout.pad_t;
        let inner_w = (rect.width - layout.pad_l - layout.pad_r).max(0.0);
        let inner_h = (rect.height - layout.pad_t - layout.pad_b).max(0.0);
        let n = children.len() as f32;
        let total_gap = layout.gap * (children.len().saturating_sub(1) as f32);

        match layout.flex_dir {
            1 => {
                // Row (horizontal)
                let slice_w = ((inner_w - total_gap) / n).max(MIN_TOUCH_TARGET_DP);
                let mut cx = inner_x;
                for child_id in children {
                    let (cw, ch) = self
                        .nodes
                        .get(&child_id)
                        .map(|c| {
                            let w = if c.layout.width > 0.0 {
                                c.layout.width
                            } else {
                                slice_w
                            };
                            let h = if c.layout.height > 0.0 {
                                c.layout.height
                            } else {
                                inner_h.max(MIN_TOUCH_TARGET_DP)
                            };
                            (w.max(c.layout.min_w), h)
                        })
                        .unwrap_or((slice_w, inner_h));
                    self.layout_subtree(child_id, Rect::new(cx, inner_y, cw, ch));
                    cx += cw + layout.gap;
                }
            }
            2 => {
                // Box (stack)
                for child_id in children {
                    let (cw, ch) = self
                        .nodes
                        .get(&child_id)
                        .map(|c| {
                            let w = if c.layout.width > 0.0 {
                                c.layout.width
                            } else {
                                inner_w
                            };
                            let h = if c.layout.height > 0.0 {
                                c.layout.height
                            } else {
                                inner_h
                            };
                            (w.max(c.layout.min_w), h)
                        })
                        .unwrap_or((inner_w, inner_h));
                    self.layout_subtree(child_id, Rect::new(inner_x, inner_y, cw, ch));
                }
            }
            _ => {
                // Column (vertical default)
                let slice_h = ((inner_h - total_gap) / n).max(MIN_TOUCH_TARGET_DP);
                let mut cy = inner_y;
                for child_id in children {
                    let (cw, ch) = self
                        .nodes
                        .get(&child_id)
                        .map(|c| {
                            let w = if c.layout.width > 0.0 {
                                c.layout.width
                            } else {
                                inner_w.max(MIN_TOUCH_TARGET_DP)
                            };
                            let h = if c.layout.height > 0.0 {
                                c.layout.height
                            } else {
                                slice_h
                            };
                            (w.max(c.layout.min_w), h)
                        })
                        .unwrap_or((inner_w, slice_h));
                    self.layout_subtree(child_id, Rect::new(inner_x, cy, cw, ch));
                    cy += ch + layout.gap;
                }
            }
        }
    }

    /// Deepest-first hit-test against nodes in `ApplierSceneGraph` whose `event_mask & required_mask != 0`.
    pub fn hit_test_masked(&self, pt: Point, required_mask: u32) -> Option<u64> {
        self.hit_test_node(self.root_id, pt, required_mask)
    }

    fn hit_test_node(&self, node_id: u64, pt: Point, required_mask: u32) -> Option<u64> {
        let node = self.nodes.get(&node_id)?;
        for &child_id in node.children.iter().rev() {
            if let Some(hit) = self.hit_test_node(child_id, pt, required_mask) {
                return Some(hit);
            }
        }
        if (node.event_mask & required_mask) != 0 && node.bounds.contains(pt) {
            return Some(node_id);
        }
        None
    }

    /// Returns `true` when this `ApplierSceneGraph` holds a custom `@Composable` application tree
    /// (rather than the canonical `StandardBenchmarkApp` tree which registers `RENDERER_BTN = 15`).
    pub fn is_custom_app_scene(&self) -> bool {
        self.nodes.len() > 2
            && !self.nodes.contains_key(&15)
            && self.nodes.values().any(|n| {
                n.node_id >= 100
                    && (n.visual.bg_rgba != 0
                        || n.visual.border_w > 0.0
                        || n.shader.shader_id != 0
                        || n.text.is_some())
            })
    }

    /// Converts the retained `ApplierSceneGraph` into a complete GPU `SdfPrimitiveInstance` stream
    /// (including fullscreen `ElectricJellyfish` background, all node containers/shaders/borders, and
    /// all `GlyphAtlas` typography quads) so any third-party `@Composable` app renders 100% on the
    /// hardware Vulkan GPU.
    pub fn build_gpu_instances(
        &self,
        viewport: Rect,
        theme: &ThemeDescriptor,
    ) -> (Vec<SdfPrimitiveInstance>, usize) {
        let mut out = Vec::with_capacity(self.nodes.len() * 4 + 64);
        let bg_col = theme.colors.background.to_array();
        let full_clip = [
            0.0,
            0.0,
            viewport.width.max(1.0),
            viewport.height.max(1.0),
        ];
        out.push(SdfPrimitiveInstance {
            rect: full_clip,
            corner_radii: [0.0; 4],
            clip_rect: full_clip,
            fill_color_start: bg_col,
            fill_color_end: bg_col,
            border_color: [0.0; 4],
            shadow_color: [0.0; 4],
            params: [
                0.0,
                0.0,
                2.0,
                ShaderMaterialId::ElectricJellyfish as u32 as f32,
            ],
        });

        let mut widget_nodes = 0_usize;
        self.emit_node_gpu_instances(
            self.root_id,
            Rect::new(0.0, 0.0, viewport.width.max(1.0), viewport.height.max(1.0)),
            theme,
            &mut out,
            &mut widget_nodes,
        );
        (out, widget_nodes)
    }

    fn emit_node_gpu_instances(
        &self,
        node_id: u64,
        parent_clip: Rect,
        theme: &ThemeDescriptor,
        out: &mut Vec<SdfPrimitiveInstance>,
        widget_nodes: &mut usize,
    ) {
        let Some(node) = self.nodes.get(&node_id) else {
            return;
        };
        let b = node.bounds;
        if b.width <= 0.5 || b.height <= 0.5 {
            return;
        }
        let node_clip = if node.visual.clip != 0 {
            parent_clip.intersect(&b)
        } else {
            parent_clip
        };

        let has_box = node_id != self.root_id
            && (node.visual.bg_rgba != 0
                || node.visual.border_w > 0.0
                || node.shader.shader_id != 0
                || node.image.is_some()
                || node.kind == 5
                || node.kind == 6
                || node.kind == 7);

        if has_box {
            *widget_nodes += 1;
            let fill_c = if node.visual.bg_rgba != 0 {
                Color::from_hex(node.visual.bg_rgba)
            } else if node.kind == 5 {
                theme.colors.primary.with_alpha(0.76)
            } else {
                theme.colors.surface_container.with_alpha(0.68)
            };
            let border_c = if node.visual.border_rgba != 0 {
                Color::from_hex(node.visual.border_rgba)
            } else {
                theme.colors.outline
            };
            let mat = if node.image.is_some() {
                ShaderMaterialId::TexturedImage
            } else if node.shader.shader_id == 0 {
                ShaderMaterialId::SubtleGlassReflection
            } else {
                ShaderMaterialId::from_u32(node.shader.shader_id as u32)
            };
            let radius = if node.visual.corner_radius > 0.0 {
                node.visual.corner_radius
            } else {
                theme.shapes.radius_sm
            };
            let bw = if node.visual.border_w > 0.0 {
                node.visual.border_w
            } else {
                1.0
            };
            out.push(SdfPrimitiveInstance {
                rect: b.to_array(),
                corner_radii: [radius; 4],
                clip_rect: node_clip.to_array(),
                fill_color_start: fill_c.to_array(),
                fill_color_end: fill_c.to_array(),
                border_color: border_c.to_array(),
                shadow_color: [0.0, 0.0, 0.0, 0.35],
                params: [bw, 6.0, theme.shapes.shape_exponent_p, mat as u32 as f32],
            });
        }

        if let Some(txt) = &node.text {
            if !txt.text.is_empty() {
                let col = if txt.color_rgba != 0 {
                    Color::from_hex(txt.color_rgba)
                } else if node.kind == 5 {
                    theme.colors.on_primary
                } else {
                    theme.colors.on_surface
                };
                let is_headline = txt.font_size_sp >= 20.0 || txt.font_weight >= 600;
                let ty = b.y + ((b.height - if is_headline { 26.0 } else { 21.0 }) * 0.5).max(2.0);
                push_gpu_text_glyphs(
                    out,
                    Point::new(b.x + 10.0, ty),
                    &txt.text,
                    col,
                    node_clip.intersect(&b),
                    is_headline,
                );
            }
        }

        for &child_id in &node.children {
            self.emit_node_gpu_instances(child_id, node_clip, theme, out, widget_nodes);
        }
    }
}

/// Deterministic Headless UI Test Harness (`IguanodonTestHarness`) for driving synthetic
/// user interactions (clicks, right-clicks, drag-and-drop, text typing, scrolling, animations)
/// against `StandardBenchmarkApp` and the retained `ApplierSceneGraph` in unit and integration tests.
pub struct IguanodonTestHarness {
    pub app: StandardBenchmarkApp,
    pub vk: VulkanRenderer,
    pub framebuffer_rgba: Vec<u8>,
    pub active_scroll_drag: Option<(NodeId, f32, bool)>,
    pub active_slider_drag: bool,
    pub applier_scene: ApplierSceneGraph,
    pub pending_reverse_events: Vec<ReverseUiEvent>,
}

impl Default for IguanodonTestHarness {
    fn default() -> Self {
        Self::new(1280.0, 800.0)
    }
}

impl IguanodonTestHarness {
    pub fn new(width: f32, height: f32) -> Self {
        let mut applier_scene = ApplierSceneGraph::new();
        applier_scene.recompute_layout(width, height);
        Self {
            app: StandardBenchmarkApp::new(width, height),
            vk: VulkanRenderer::new(),
            framebuffer_rgba: Vec::new(),
            active_scroll_drag: None,
            active_slider_drag: false,
            applier_scene,
            pending_reverse_events: Vec::with_capacity(32),
        }
    }

    pub fn push_reverse_event(&mut self, ev: ReverseUiEvent) {
        if self.pending_reverse_events.len() < 1024 {
            self.pending_reverse_events.push(ev);
        }
    }

    /// Drains queued reverse events into `out_buf` (each record is 32 bytes) and returns the number of bytes written.
    pub fn drain_reverse_events(&mut self, out_buf: &mut [u8]) -> usize {
        let max_records = out_buf.len() / ReverseUiEvent::BYTE_SIZE;
        if max_records == 0 || self.pending_reverse_events.is_empty() {
            return 0;
        }
        let count = max_records.min(self.pending_reverse_events.len());
        let mut offset = 0;
        for ev in self.pending_reverse_events.drain(0..count) {
            let bytes = ev.to_bytes();
            out_buf[offset..offset + ReverseUiEvent::BYTE_SIZE].copy_from_slice(&bytes);
            offset += ReverseUiEvent::BYTE_SIZE;
        }
        offset
    }

    fn update_speed_slider_from_x(&mut self, pointer_x: f32) {
        if let Some(r) = self.app.layout.bounds(bench_nodes::ANIM_SPEED_SLIDER) {
            let track_x = r.x + 14.0;
            let track_w = (r.width - 28.0).max(24.0);
            let norm = ((pointer_x - track_x) / track_w).clamp(0.0, 1.0);
            self.app.set_animation_speed(norm * 2.0);
        }
    }

    pub fn node_center(&self, node: NodeId) -> Point {
        let r = self
            .app
            .layout
            .bounds(node)
            .expect("Target node must have computed layout bounds");
        Point::new(r.x + r.width * 0.5, r.y + r.height * 0.5)
    }

    /// Simulates a primary mouse click / tap on `node` (or at an arbitrary point if context menu is open).
    pub fn click_node(&mut self, node: NodeId) {
        use bench_nodes::*;
        let pt = self.node_center(node);
        self.app.pointer_pos = pt;

        if self.app.context_menu.is_open && node != CONTEXT_MENU_POPUP {
            self.app.context_menu.close();
        }

        match node {
            PRIMARY_BTN => {
                self.app.button_click_count += 1;
                self.app.button_press_spring.value = 1.0;
                self.app.button_press_spring.set_target(0.0);
                // Trigger underdamped jello click wobble (squash-and-stretch oscillation)
                self.app.button_jello_spring.value = 1.0;
                self.app.button_jello_spring.set_target(0.0);
                let next_prog =
                    ((self.app.progress_spring.target + 0.2).fract()).clamp(0.1, 1.0);
                self.app.progress_spring.set_target(next_prog);
                self.app.status_label_text =
                    format!("Button clicked {} times", self.app.button_click_count);
            }
            FIRE_SHADER_BTN => {
                self.app.fire_shader_enabled = !self.app.fire_shader_enabled;
                self.app.shader_btn_jello_spring.value = 1.0;
                self.app.shader_btn_jello_spring.set_target(0.0);
            }
            THEME_BTN => {
                self.app.cycle_theme();
            }
            MODE_BTN => {
                self.app.toggle_theme_mode();
                let mode_val = self.app.current_theme.mode as u32;
                self.push_reverse_event(ReverseUiEvent {
                    event_type: ReverseEventType::ThemeModeChanged,
                    flags: 0,
                    aux_u32: mode_val,
                    node_id: 0,
                    f0: 0.0,
                    f1: 0.0,
                    f2: 0.0,
                    f3: 0.0,
                });
            }
            POWER_BTN => {
                self.app.toggle_power_mode();
                let pwr_val = self.app.power_mode as u32;
                self.push_reverse_event(ReverseUiEvent {
                    event_type: ReverseEventType::PowerModeChanged,
                    flags: 0,
                    aux_u32: pwr_val,
                    node_id: 0,
                    f0: self.app.power_mode.target_fps() as f32,
                    f1: 0.0,
                    f2: 0.0,
                    f3: 0.0,
                });
            }
            RENDERER_BTN => {
                self.app.toggle_renderer_backend();
                let backend_val = self.app.renderer_backend as u32;
                self.push_reverse_event(ReverseUiEvent {
                    event_type: ReverseEventType::RendererBackendChanged,
                    flags: 0,
                    aux_u32: backend_val,
                    node_id: 0,
                    f0: 0.0,
                    f1: 0.0,
                    f2: 0.0,
                    f3: 0.0,
                });
            }
            EDIT_TEXT_FIELD => {
                self.app.text_field.focused = true;
            }
            ANIM_SPEED_SLIDER => {
                // Clicking the slider cycles through 0.25x -> 0.65x -> 1.0x -> 1.5x
                let next = if self.app.animation_speed < 0.4 {
                    0.65
                } else if self.app.animation_speed < 0.85 {
                    1.0
                } else if self.app.animation_speed < 1.3 {
                    1.5
                } else {
                    0.25
                };
                self.app.set_animation_speed(next);
            }
            ANIMATED_SHADER_CARD => {
                self.app.showcase_shader_idx =
                    (self.app.showcase_shader_idx + 1) % ShaderMaterialId::SHOWCASE_PRESETS.len();
                self.app.active_showcase_shader =
                    ShaderMaterialId::SHOWCASE_PRESETS[self.app.showcase_shader_idx];
                self.app.shader_btn_jello_spring.value = 1.0;
                self.app.shader_btn_jello_spring.set_target(0.0);
                self.app.anim_card_expanded = !self.app.anim_card_expanded;
                self.app
                    .anim_card_scale_spring
                    .set_target(if self.app.anim_card_expanded {
                        1.08
                    } else {
                        1.0
                    });
                self.app
                    .anim_card_corner_spring
                    .set_target(if self.app.anim_card_expanded {
                        self.app.current_theme.shapes.radius_xl
                    } else {
                        self.app.current_theme.shapes.radius_md
                    });
                self.app.status_label_text = format!(
                    "Shader: {}",
                    self.app.active_showcase_shader.display_name()
                );
                self.app.rebuild_layout();
            }
            _ => {}
        }
        self.push_reverse_event(ReverseUiEvent {
            event_type: ReverseEventType::Click,
            flags: 0,
            aux_u32: 0,
            node_id: node.index as u64,
            f0: pt.x,
            f1: pt.y,
            f2: 0.0,
            f3: 0.0,
        });
    }

    /// Simulates a right-click on `node`, opening the frosted-glass context menu.
    pub fn right_click_node(&mut self, node: NodeId) {
        let pt = self.node_center(node);
        self.app.pointer_pos = pt;
        self.app.context_menu.open(
            pt,
            Some(node),
            vec![
                ContextMenuItem {
                    id: "copy_label".into(),
                    label: "Copy Widget Label".into(),
                    shortcut: Some("Ctrl+C".into()),
                    enabled: true,
                },
                ContextMenuItem {
                    id: "toggle_fire".into(),
                    label: "Toggle Fire Shader".into(),
                    shortcut: Some("Ctrl+F".into()),
                    enabled: true,
                },
                ContextMenuItem {
                    id: "complete_progress".into(),
                    label: "Set Progress 100%".into(),
                    shortcut: Some("Ctrl+P".into()),
                    enabled: true,
                },
                ContextMenuItem {
                    id: "toggle_mode".into(),
                    label: "Toggle Light/Dark Mode".into(),
                    shortcut: Some("Ctrl+M".into()),
                    enabled: true,
                },
            ],
        );
    }

    /// Selects and invokes an item in the open right-click context menu.
    pub fn click_context_menu_item(&mut self, index: usize) -> Option<String> {
        let action = self.app.context_menu.invoke_index(index)?;
        match action.as_str() {
            "toggle_fire" => self.app.fire_shader_enabled = !self.app.fire_shader_enabled,
            "complete_progress" => self.app.progress_spring.set_target(1.0),
            "toggle_mode" => self.app.toggle_theme_mode(),
            _ => {}
        }
        Some(action)
    }

    /// Simulates dragging `source_node` and dropping it onto `target_node`.
    pub fn drag_and_drop(
        &mut self,
        source_node: NodeId,
        payload_id: &str,
        target_node: NodeId,
    ) -> bool {
        let start_pt = self.node_center(source_node);
        let end_pt = self.node_center(target_node);

        self.app
            .drag_drop
            .start_drag(source_node, payload_id, payload_id, start_pt);

        let hovered = self.app.layout.hit_test(bench_nodes::ROOT, end_pt);
        self.app.drag_drop.update_drag(end_pt, hovered);

        if hovered == Some(bench_nodes::DROP_TARGET_BUCKET) {
            self.app.drop_target_glow_spring.value = 1.0;
            self.app.drop_target_glow_spring.set_target(0.0);
        }

        if let Some((dropped_id, dst)) = self.app.drag_drop.complete_drop() {
            if dst == bench_nodes::DROP_TARGET_BUCKET {
                self.app.dropped_items.push(dropped_id);
                return true;
            }
        }
        false
    }

    /// Focuses the `TextField` and types `input` characters into it.
    pub fn type_into_text_field(&mut self, replace_all: bool, input: &str) {
        self.click_node(bench_nodes::EDIT_TEXT_FIELD);
        if replace_all {
            self.app.text_field.select_all();
        }
        self.app.text_field.insert_str(input);
    }

    /// Scrolls either `SCROLLABLE_TEXT` or `LAZY_COLUMN` by `delta_y` pixels.
    /// When scrolling past `< 0.0` or `> max_scroll_y`, accumulates rubber-band displacement into
    /// `scrollable_text_overscroll_spring` or `lazy_overscroll_spring` so the list bounces back smoothly.
    pub fn scroll_node(&mut self, node: NodeId, delta_y: f32) {
        use bench_nodes::*;
        match node {
            SCROLLABLE_TEXT => {
                let max_y = self.app.scrollable_text_scrollbar.max_scroll_y;
                let unclamped = self.app.scrollable_text_scroll_y + delta_y;
                if unclamped < 0.0 {
                    let excess = unclamped;
                    self.app.scrollable_text_overscroll_spring.value =
                        (self.app.scrollable_text_overscroll_spring.value + excess * 0.55)
                            .clamp(-80.0, 80.0);
                    self.app.scrollable_text_overscroll_spring.set_target(0.0);
                } else if unclamped > max_y {
                    let excess = unclamped - max_y;
                    self.app.scrollable_text_overscroll_spring.value =
                        (self.app.scrollable_text_overscroll_spring.value + excess * 0.55)
                            .clamp(-80.0, 80.0);
                    self.app.scrollable_text_overscroll_spring.set_target(0.0);
                }
                self.app.scrollable_text_scroll_y = unclamped.clamp(0.0, max_y);
                self.app.rebuild_layout();
            }
            LAZY_COLUMN => {
                let max_y = self.app.lazy_layout.scrollbar.max_scroll_y;
                let unclamped = self.app.lazy_scroll_y + delta_y;
                if unclamped < 0.0 {
                    let excess = unclamped;
                    self.app.lazy_overscroll_spring.value =
                        (self.app.lazy_overscroll_spring.value + excess * 0.55).clamp(-96.0, 96.0);
                    self.app.lazy_overscroll_spring.set_target(0.0);
                } else if unclamped > max_y {
                    let excess = unclamped - max_y;
                    self.app.lazy_overscroll_spring.value =
                        (self.app.lazy_overscroll_spring.value + excess * 0.55).clamp(-96.0, 96.0);
                    self.app.lazy_overscroll_spring.set_target(0.0);
                }
                self.app.lazy_scroll_y = unclamped.clamp(0.0, max_y);
                self.app.rebuild_layout();
            }
            _ => {}
        }
    }

    /// Advances `frames` at 120Hz (`dt = 1/120s`) and executes the Vulkan render pipeline.
    pub fn advance_frames(&mut self, frames: usize) {
        for _ in 0..frames {
            self.app.render_benchmark_frame(
                &mut self.vk,
                1.0 / 120.0,
                64,
                40,
                &mut self.framebuffer_rgba,
            );
        }
    }

    /// Coordinate-based pointer/touch movement handler for interactive windowing (Android & Desktop).
    pub fn on_pointer_move(&mut self, pt: Point) {
        self.app.pointer_pos = pt;

        // Track hover state & active drag across ApplierSceneGraph
        let new_hover = self.applier_scene.hit_test_masked(pt, event_mask::HOVER);
        if new_hover != self.applier_scene.hovered_node_id {
            if let Some(prev_id) = self.applier_scene.hovered_node_id {
                self.push_reverse_event(ReverseUiEvent {
                    event_type: ReverseEventType::HoverExit,
                    flags: 0,
                    aux_u32: 0,
                    node_id: prev_id,
                    f0: pt.x,
                    f1: pt.y,
                    f2: 0.0,
                    f3: 0.0,
                });
            }
            if let Some(next_id) = new_hover {
                self.push_reverse_event(ReverseUiEvent {
                    event_type: ReverseEventType::HoverEnter,
                    flags: 0,
                    aux_u32: 0,
                    node_id: next_id,
                    f0: pt.x,
                    f1: pt.y,
                    f2: 0.0,
                    f3: 0.0,
                });
            }
            self.applier_scene.hovered_node_id = new_hover;
        }

        if let Some((drag_node_id, prev_pt)) = self.applier_scene.active_drag_node_id {
            let dx = pt.x - prev_pt.x;
            let dy = pt.y - prev_pt.y;
            if dx.abs() > 0.1 || dy.abs() > 0.1 {
                self.applier_scene.active_drag_node_id = Some((drag_node_id, pt));
                self.push_reverse_event(ReverseUiEvent {
                    event_type: ReverseEventType::DragDelta,
                    flags: 0,
                    aux_u32: 0,
                    node_id: drag_node_id,
                    f0: dx,
                    f1: dy,
                    f2: pt.x,
                    f3: pt.y,
                });
            }
        }

        if self.active_slider_drag {
            self.update_speed_slider_from_x(pt.x);
            return;
        }
        if self.app.drag_drop.active_drag.is_some() {
            let hovered = self.app.layout.hit_test(bench_nodes::ROOT, pt);
            self.app.drag_drop.update_drag(pt, hovered);
            self.app.drop_target_glow_spring.set_target(
                if hovered == Some(bench_nodes::DROP_TARGET_BUCKET) {
                    1.0
                } else {
                    0.0
                },
            );
            return;
        }

        if let Some((scroll_node, prev_y, is_scrollbar_thumb)) = self.active_scroll_drag {
            use bench_nodes::*;
            if is_scrollbar_thumb {
                match scroll_node {
                    LAZY_COLUMN => {
                        self.app.lazy_scroll_y =
                            self.app.lazy_layout.scrollbar.scroll_y_from_pointer(pt.y);
                        self.app.rebuild_layout();
                    }
                    SCROLLABLE_TEXT => {
                        self.app.scrollable_text_scroll_y = self
                            .app
                            .scrollable_text_scrollbar
                            .scroll_y_from_pointer(pt.y);
                        self.app.rebuild_layout();
                    }
                    _ => {}
                }
            } else {
                // Touch-pan / drag scrolling (pulling finger up scrolls content down)
                let delta_y = prev_y - pt.y;
                if delta_y.abs() > 0.5 {
                    self.scroll_node(scroll_node, delta_y);
                }
            }
            self.active_scroll_drag = Some((scroll_node, pt.y, is_scrollbar_thumb));
        }
    }

    /// Coordinate-based primary button / touch press handler for interactive windowing.
    pub fn on_pointer_down(&mut self, pt: Point, is_right_button: bool) {
        use bench_nodes::*;
        self.app.pointer_pos = pt;
        self.active_scroll_drag = None;
        self.active_slider_drag = false;

        // Dispatch reverse events for custom retained ApplierSceneGraph nodes (IDs >= 100).
        // Canonical benchmark nodes (IDs < 100) are hit-tested against `self.app.layout` below
        // using their exact visual bounds so equal-slice Applier containers never intercept taps.
        if is_right_button {
            if let Some(hit_id) = self
                .applier_scene
                .hit_test_masked(pt, event_mask::LONG_CLICK | event_mask::CLICK)
            {
                if hit_id >= 100 {
                    self.push_reverse_event(ReverseUiEvent {
                        event_type: ReverseEventType::LongOrSecondaryClick,
                        flags: 1,
                        aux_u32: 0,
                        node_id: hit_id,
                        f0: pt.x,
                        f1: pt.y,
                        f2: 0.0,
                        f3: 0.0,
                    });
                }
            }
        } else {
            if let Some(drag_id) = self.applier_scene.hit_test_masked(pt, event_mask::DRAG) {
                if drag_id >= 100 {
                    self.applier_scene.active_drag_node_id = Some((drag_id, pt));
                }
            }
            if let Some(click_id) = self
                .applier_scene
                .hit_test_masked(pt, event_mask::CLICK | event_mask::TEXT_INPUT)
            {
                if click_id >= 100 {
                    self.applier_scene.focused_node_id = Some(click_id);
                    self.push_reverse_event(ReverseUiEvent {
                        event_type: ReverseEventType::Click,
                        flags: 0,
                        aux_u32: 0,
                        node_id: click_id,
                        f0: pt.x,
                        f1: pt.y,
                        f2: 0.0,
                        f3: 0.0,
                    });
                }
            }
        }

        if is_right_button {
            let hit = self
                .app
                .layout
                .hit_test(ROOT, pt)
                .unwrap_or(CONTEXT_MENU_REGION);
            self.app.context_menu.open(
                pt,
                Some(hit),
                vec![
                    ContextMenuItem {
                        id: "copy_label".into(),
                        label: "Copy Widget Label".into(),
                        shortcut: Some("Ctrl+C".into()),
                        enabled: true,
                    },
                    ContextMenuItem {
                        id: "toggle_fire".into(),
                        label: "Toggle Fire Shader".into(),
                        shortcut: Some("Ctrl+F".into()),
                        enabled: true,
                    },
                    ContextMenuItem {
                        id: "complete_progress".into(),
                        label: "Set Progress 100%".into(),
                        shortcut: Some("Ctrl+P".into()),
                        enabled: true,
                    },
                    ContextMenuItem {
                        id: "toggle_mode".into(),
                        label: "Toggle Light/Dark Mode".into(),
                        shortcut: Some("Ctrl+M".into()),
                        enabled: true,
                    },
                ],
            );
            return;
        }

        // Check if clicking inside open context menu popup (each item is 52dp tall)
        if self.app.context_menu.is_open {
            let menu_w = 296.0_f32;
            let item_h = 52.0_f32.max(MIN_TOUCH_TARGET_DP);
            let menu_h = (self.app.context_menu.items.len() as f32 * item_h) + 12.0;
            let mx = self
                .app
                .context_menu
                .anchor_pos
                .x
                .min((self.app.viewport.width - menu_w - 8.0).max(0.0));
            let my = self
                .app
                .context_menu
                .anchor_pos
                .y
                .min((self.app.viewport.height - menu_h - 8.0).max(0.0));
            let menu_rect = Rect::new(mx, my, menu_w, menu_h);
            if menu_rect.contains(pt) {
                let max_idx = self.app.context_menu.items.len().saturating_sub(1);
                let idx = (((pt.y - my - 6.0) / item_h).floor().max(0.0) as usize).min(max_idx);
                self.click_context_menu_item(idx);
                return;
            } else {
                self.app.context_menu.close();
            }
        }

        if let Some(hit) = self.app.layout.hit_test(ROOT, pt) {
            self.app.text_field.focused = hit == EDIT_TEXT_FIELD;
            match hit {
                DRAGGABLE_CARD_A => {
                    self.app.drag_drop.start_drag(
                        DRAGGABLE_CARD_A,
                        "Vulkan-Asset-01",
                        "Vulkan Asset #01",
                        pt,
                    );
                }
                DRAGGABLE_CARD_B => {
                    self.app.drag_drop.start_drag(
                        DRAGGABLE_CARD_B,
                        "M3-Token-Pack-02",
                        "M3 Token Pack #02",
                        pt,
                    );
                }
                LAZY_COLUMN => {
                    if let Some(r) = self.app.layout.bounds(LAZY_COLUMN) {
                        if self.app.lazy_layout.scrollbar.interactive_hit_rect.contains(pt) {
                            self.app.lazy_scroll_y =
                                self.app.lazy_layout.scrollbar.scroll_y_from_pointer(pt.y);
                            self.app.rebuild_layout();
                            self.active_scroll_drag = Some((LAZY_COLUMN, pt.y, true));
                        } else {
                            let stride = MIN_TOUCH_TARGET_DP + 4.0;
                            let idx = (((pt.y - r.y + self.app.lazy_scroll_y) / stride)
                                .floor()
                                .max(0.0) as usize)
                                .min(self.app.lazy_total_items.saturating_sub(1));
                            self.app.selected_lazy_item = Some(idx);
                            self.app.status_label_text =
                                format!("Selected Lazy Item #{:04}", idx);
                            self.active_scroll_drag = Some((LAZY_COLUMN, pt.y, false));
                        }
                    }
                }
                SCROLLABLE_TEXT => {
                    if self
                        .app
                        .scrollable_text_scrollbar
                        .interactive_hit_rect
                        .contains(pt)
                    {
                        self.app.scrollable_text_scroll_y = self
                            .app
                            .scrollable_text_scrollbar
                            .scroll_y_from_pointer(pt.y);
                        self.app.rebuild_layout();
                        self.active_scroll_drag = Some((SCROLLABLE_TEXT, pt.y, true));
                    } else {
                        self.active_scroll_drag = Some((SCROLLABLE_TEXT, pt.y, false));
                    }
                }
                SHOWCASE_IMAGE => {
                    self.app.image_water_ripple_active = !self.app.image_water_ripple_active;
                }
                ANIM_SPEED_SLIDER => {
                    self.active_slider_drag = true;
                    self.update_speed_slider_from_x(pt.x);
                }
                other => self.click_node(other),
            }
        }
    }

    /// Coordinate-based primary button / touch release handler (commits drag-and-drop if active).
    pub fn on_pointer_up(&mut self, pt: Point) {
        self.app.pointer_pos = pt;
        self.active_scroll_drag = None;
        self.active_slider_drag = false;
        self.applier_scene.active_drag_node_id = None;
        if self.app.drag_drop.active_drag.is_some() {
            let hovered = self.app.layout.hit_test(bench_nodes::ROOT, pt);
            self.app.drag_drop.update_drag(pt, hovered);
            self.app.drop_target_glow_spring.set_target(0.0);
            if let Some((payload, dst)) = self.app.drag_drop.complete_drop() {
                if dst == bench_nodes::DROP_TARGET_BUCKET {
                    self.app.status_label_text = format!("Dropped '{}' into Target!", payload);
                    self.app.dropped_items.push(payload);
                }
            }
        }
    }

    /// Coordinate-based mouse wheel scroll handler.
    pub fn on_scroll_wheel(&mut self, pt: Point, delta_y: f32) {
        if let Some(scroll_id) = self.applier_scene.hit_test_masked(pt, event_mask::SCROLL) {
            self.push_reverse_event(ReverseUiEvent {
                event_type: ReverseEventType::ScrollDelta,
                flags: 0,
                aux_u32: 0,
                node_id: scroll_id,
                f0: 0.0,
                f1: delta_y,
                f2: pt.x,
                f3: pt.y,
            });
        }
        if let Some(r) = self.app.layout.bounds(bench_nodes::SCROLLABLE_TEXT) {
            if r.contains(pt) {
                self.scroll_node(bench_nodes::SCROLLABLE_TEXT, delta_y);
                return;
            }
        }
        self.scroll_node(bench_nodes::LAZY_COLUMN, delta_y);
    }

    /// Resizes the window/viewport and recomputes responsive layout and virtualization bounds.
    pub fn resize(&mut self, width: f32, height: f32) {
        let changed = (self.app.viewport.width - width).abs() > 0.5
            || (self.app.viewport.height - height).abs() > 0.5;
        self.app.resize(width, height);
        self.applier_scene.recompute_layout(width, height);
        if changed {
            self.push_reverse_event(ReverseUiEvent {
                event_type: ReverseEventType::WindowResize,
                flags: 0,
                aux_u32: 0,
                node_id: 0,
                f0: width,
                f1: height,
                f2: 0.0,
                f3: 0.0,
            });
        }
    }

    /// Renders a complete frame directly to the attached hardware Vulkan swapchain
    /// (`VkSwapchainKHR` on Android `ANativeWindow` or Linux X11 `Window`) and presents it via
    /// `vkQueuePresentKHR`. Works identically for both `StandardBenchmarkApp` and any custom
    /// `@Composable` app tree managed by `IguanodonApplier`.
    pub fn render_and_present(&mut self, dt: f32, width: u32, height: u32) -> FrameSample {
        let frame_start = Instant::now();
        self.resize(width as f32, height as f32);
        self.app.step_animations(dt);

        let layout_start = Instant::now();
        self.app.rebuild_layout();
        self.applier_scene
            .recompute_layout(width as f32, height as f32);
        let layout_us = layout_start.elapsed().as_secs_f32() * 1_000_000.0;

        let pack_start = Instant::now();
        let (gpu_instances, materialized_nodes) = if self.applier_scene.is_custom_app_scene() {
            self.applier_scene
                .build_gpu_instances(self.app.viewport, &self.app.current_theme)
        } else {
            self.app.build_complete_gpu_instances()
        };
        let sdf_pack_us = pack_start.elapsed().as_secs_f32() * 1_000_000.0;

        let vk_stats_opt = self.vk.render_and_present(
            &gpu_instances,
            self.app.pointer_pos,
            self.app.time_sec,
            self.app.current_theme.colors.background,
            width,
            height,
        );

        let total_frame_ms = frame_start.elapsed().as_secs_f32() * 1_000.0;
        let (gpu_execution_us, draw_calls) = if let Some(vk_stats) = vk_stats_opt {
            self.app.telemetry.vram_allocated_bytes = vk_stats.vram_allocated_bytes;
            self.app.telemetry.gpu_device_name = vk_stats.device_name;
            (vk_stats.gpu_timestamp_us, vk_stats.draw_calls)
        } else {
            (0.0, 1)
        };
        let sample = FrameSample {
            total_frame_ms,
            ffi_decode_us: 12.0,
            layout_us,
            sdf_pack_us,
            gpu_execution_us,
            materialized_nodes,
            virtualized_items: self.app.lazy_total_items,
            draw_calls,
        };
        self.app.telemetry.record_frame(sample);
        sample
    }

    /// Renders a complete frame offscreen on the hardware Vulkan GPU (or CPU fallback) into
    /// `self.framebuffer_rgba`, automatically supporting both `StandardBenchmarkApp` and any custom
    /// `@Composable` app tree managed by `IguanodonApplier`.
    pub fn render_frame_rgba(&mut self, dt: f32, width: u32, height: u32) -> FrameSample {
        self.resize(width as f32, height as f32);
        if self.applier_scene.is_custom_app_scene() {
            let frame_start = Instant::now();
            self.app.step_animations(dt);
            let layout_start = Instant::now();
            self.applier_scene
                .recompute_layout(width as f32, height as f32);
            let layout_us = layout_start.elapsed().as_secs_f32() * 1_000_000.0;

            let pack_start = Instant::now();
            let (gpu_instances, materialized_nodes) = self
                .applier_scene
                .build_gpu_instances(self.app.viewport, &self.app.current_theme);
            let sdf_pack_us = pack_start.elapsed().as_secs_f32() * 1_000_000.0;

            let vk_stats = self.vk.render_frame(
                &gpu_instances,
                self.app.pointer_pos,
                self.app.time_sec,
                self.app.current_theme.colors.background,
                width,
                height,
                &mut self.framebuffer_rgba,
            );
            let total_frame_ms = frame_start.elapsed().as_secs_f32() * 1_000.0;
            let sample = FrameSample {
                total_frame_ms,
                ffi_decode_us: 12.0,
                layout_us,
                sdf_pack_us,
                gpu_execution_us: vk_stats.gpu_timestamp_us,
                materialized_nodes,
                virtualized_items: self.app.lazy_total_items,
                draw_calls: vk_stats.draw_calls,
            };
            self.app.telemetry.vram_allocated_bytes = vk_stats.vram_allocated_bytes;
            self.app.telemetry.gpu_device_name = vk_stats.device_name;
            self.app.telemetry.record_frame(sample);
            sample
        } else {
            self.app.render_benchmark_frame(
                &mut self.vk,
                dt,
                width,
                height,
                &mut self.framebuffer_rgba,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bench_nodes::*;

    #[test]
    fn test_responsive_window_resize_reflows_layout_and_lazy_list() {
        let mut harness = IguanodonTestHarness::new(960.0, 600.0);
        let initial_vis = harness.app.lazy_layout.materialized_count();
        let initial_right_col = harness.app.layout.bounds(RIGHT_COL).unwrap();

        // Resize to a tall ultrawide window (1920x1080)
        harness.resize(1920.0, 1080.0);
        let tall_vis = harness.app.lazy_layout.materialized_count();
        let tall_right_col = harness.app.layout.bounds(RIGHT_COL).unwrap();
        assert!(tall_right_col.width > initial_right_col.width);
        assert!(
            tall_vis > initial_vis,
            "Taller window should materialize more LazyColumn rows"
        );

        // Resize down to a compact window (680x520)
        harness.resize(680.0, 520.0);
        let compact_vis = harness.app.lazy_layout.materialized_count();
        assert!(compact_vis <= initial_vis);
        assert!(harness.app.scrollable_text_scrollbar.is_scrollable);
    }

    #[test]
    fn test_all_interactive_targets_meet_48dp_minimum() {
        let mut harness = IguanodonTestHarness::new(960.0, 600.0);
        // Check default landscape, compact landscape, and Android portrait (single-column) + edge-to-edge safe insets
        for (w, h, insets) in [
            (1280.0, 800.0, [0.0, 0.0, 0.0, 0.0]),
            (960.0, 600.0, [0.0, 0.0, 0.0, 0.0]),
            (680.0, 520.0, [0.0, 0.0, 0.0, 0.0]),
            (600.0, 1280.0, [0.0, 44.0, 0.0, 24.0]),
        ] {
            harness.resize(w, h);
            harness
                .app
                .set_safe_insets(insets[0], insets[1], insets[2], insets[3]);
            let interactive_nodes = [
                RENDERER_BTN,
                POWER_BTN,
                THEME_BTN,
                MODE_BTN,
                PRIMARY_BTN,
                FIRE_SHADER_BTN,
                EDIT_TEXT_FIELD,
                DRAGGABLE_CARD_A,
                DRAGGABLE_CARD_B,
                ANIM_SPEED_SLIDER,
                DROP_TARGET_BUCKET,
                SHOWCASE_IMAGE,
                ANIMATED_SHADER_CARD,
                SCROLLABLE_TEXT,
                CONTEXT_MENU_REGION,
                LAZY_COLUMN,
            ];
            for node in interactive_nodes {
                let r = harness.app.layout.bounds(node).unwrap();
                assert!(
                    r.width >= MIN_TOUCH_TARGET_DP && r.height >= MIN_TOUCH_TARGET_DP,
                    "Interactive node {:?} bounds {:?} violated 48dp minimum at {}x{}",
                    node,
                    r,
                    w,
                    h
                );
            }
            if w < h {
                // Verify Portrait stacks LEFT_COL, MID_COL, RIGHT_COL vertically in a single column
                let left_r = harness.app.layout.bounds(LEFT_COL).unwrap();
                let mid_r = harness.app.layout.bounds(MID_COL).unwrap();
                let right_r = harness.app.layout.bounds(RIGHT_COL).unwrap();
                assert!((left_r.x - mid_r.x).abs() < 1.0);
                assert!((mid_r.x - right_r.x).abs() < 1.0);
                assert!(mid_r.y > left_r.y + left_r.height);
                assert!(right_r.y > mid_r.y + mid_r.height);
                // Verify top bar respects top edge-to-edge camera/status-bar inset
                let top_r = harness.app.layout.bounds(TOP_BAR).unwrap();
                assert!(top_r.y >= insets[1]);
            }
            assert!(harness.app.lazy_layout.item_height >= MIN_TOUCH_TARGET_DP);
            assert!(
                harness
                    .app
                    .lazy_layout
                    .scrollbar
                    .interactive_hit_rect
                    .width
                    >= MIN_TOUCH_TARGET_DP
            );
            assert!(
                harness.app.lazy_layout.scrollbar.thumb_rect.height >= MIN_TOUCH_TARGET_DP
            );
        }
    }

    #[test]
    fn test_4k_uhd_render_performance() {
        let mut harness = IguanodonTestHarness::new(3840.0, 2160.0);
        let sample = harness.app.render_benchmark_frame(
            &mut harness.vk,
            1.0 / 120.0,
            3840,
            2160,
            &mut harness.framebuffer_rgba,
        );
        assert_eq!(harness.framebuffer_rgba.len(), 3840 * 2160 * 4);
        assert!(
            sample.total_frame_ms < 250.0,
            "4K UHD frame took {:.2} ms",
            sample.total_frame_ms
        );
    }

    #[test]
    fn test_benchmark_app_all_11_features_and_interactions() {
        let mut harness = IguanodonTestHarness::new(1280.0, 800.0);

        // 1 & 2. Labels + Buttons + 5. Progress Bar update
        assert_eq!(harness.app.button_click_count, 0);
        harness.click_node(PRIMARY_BTN);
        assert_eq!(harness.app.button_click_count, 1);
        assert!(harness.app.status_label_text.contains("1 times"));

        // 3. Scrollable Text with Scrollbar
        assert!(harness.app.scrollable_text_scrollbar.is_scrollable);
        harness.scroll_node(SCROLLABLE_TEXT, 45.0);
        assert!(harness.app.scrollable_text_scroll_y > 0.0);

        // 4. Lazy-Loaded List (10,000 items, each 48dp tall) with Scrollbars
        assert_eq!(harness.app.lazy_total_items, 10_000);
        assert!(
            harness.app.lazy_layout.materialized_count() < 30,
            "LazyColumn must only materialize visible + overscan items, not all 10,000"
        );
        harness.scroll_node(LAZY_COLUMN, 4000.0);
        assert!(harness.app.lazy_layout.visible_range.start >= 70);

        // 6. Drag and Drop (Draggable Card -> DropTarget Bucket)
        let ok = harness.drag_and_drop(
            DRAGGABLE_CARD_A,
            "Vulkan-Shader-Asset-01",
            DROP_TARGET_BUCKET,
        );
        assert!(ok, "Drag and drop onto DROP_TARGET_BUCKET should succeed");
        assert_eq!(harness.app.dropped_items, vec!["Vulkan-Shader-Asset-01"]);

        // 7. Edit Text Entry (`TextField`)
        harness.type_into_text_field(true, "Material 3 Expressive Vulkan");
        assert_eq!(harness.app.text_field.text, "Material 3 Expressive Vulkan");
        harness.app.text_field.backspace();
        assert_eq!(harness.app.text_field.text, "Material 3 Expressive Vulka");

        // 8. Right-Click Context Menu
        assert!(!harness.app.context_menu.is_open);
        harness.right_click_node(CONTEXT_MENU_REGION);
        assert!(harness.app.context_menu.is_open);
        let invoked = harness.click_context_menu_item(2);
        assert_eq!(invoked.as_deref(), Some("complete_progress"));
        assert!(!harness.app.context_menu.is_open);

        // 9. Image + 10. Animation + 11. Performance Telemetry HUD across all 4 themes in both Dark and Light modes
        for theme_id in ThemeId::ALL {
            for mode in ThemeMode::ALL {
                harness.app.set_theme_and_mode(theme_id, mode);
                harness.click_node(ANIMATED_SHADER_CARD);
                harness.advance_frames(8);
            }
        }

        assert!(harness.app.telemetry.history.len() >= 60);
        assert!(harness.app.telemetry.avg_fps() > 0.0);
        assert!((harness.app.progress_spring.value - 1.0).abs() < 0.05);
    }

    #[test]
    fn test_button_jello_wobble_on_click() {
        let mut harness = IguanodonTestHarness::new(1280.0, 800.0);
        let base_btn_rect = harness.app.layout.bounds(PRIMARY_BTN).unwrap();

        // Click PRIMARY_BTN -> triggers underdamped jello spring (zeta = 0.22)
        harness.click_node(PRIMARY_BTN);
        assert!((harness.app.button_jello_spring.value - 1.0).abs() < 1e-4);

        let initial_wobbled =
            StandardBenchmarkApp::apply_jello_wobble(base_btn_rect, harness.app.button_jello_spring.value);
        assert!(
            initial_wobbled[2] > base_btn_rect.width && initial_wobbled[3] < base_btn_rect.height,
            "Initial jello impulse should squash height and stretch width while preserving center"
        );

        // Step across frames and verify underdamped oscillation (crosses 0 into negative before settling)
        let mut saw_negative_overshoot = false;
        for _ in 0..90 {
            harness.app.step_animations(1.0 / 120.0);
            if harness.app.button_jello_spring.value < -0.05 {
                saw_negative_overshoot = true;
            }
        }
        assert!(
            saw_negative_overshoot,
            "Underdamped jello spring must oscillate past equilibrium like jello"
        );
        assert!(
            harness.app.button_jello_spring.value.abs() < 0.02,
            "Jello spring should settle back near 0.0"
        );
    }

    #[test]
    fn test_list_scroll_overspring_bounce() {
        let mut harness = IguanodonTestHarness::new(1280.0, 800.0);

        // Scroll past top of LAZY_COLUMN (delta_y < 0 when already at top 0.0)
        harness.scroll_node(LAZY_COLUMN, -120.0);
        assert_eq!(harness.app.lazy_scroll_y, 0.0);
        assert!(
            harness.app.lazy_overscroll_spring.value < -30.0,
            "Scrolling past top of list must deflect lazy_overscroll_spring negatively"
        );

        // Advance frames and verify overscroll spring bounces back to 0.0
        for _ in 0..90 {
            harness.app.step_animations(1.0 / 120.0);
        }
        assert!(
            harness.app.lazy_overscroll_spring.value.abs() < 0.5,
            "Top overscroll spring should rebound back to 0.0"
        );

        // Scroll past bottom of LAZY_COLUMN
        let max_y = harness.app.lazy_layout.scrollbar.max_scroll_y;
        harness.scroll_node(LAZY_COLUMN, max_y + 200.0);
        assert!((harness.app.lazy_scroll_y - max_y).abs() < 0.5);
        assert!(
            harness.app.lazy_overscroll_spring.value > 30.0,
            "Scrolling past end of list must deflect lazy_overscroll_spring positively"
        );
        for _ in 0..90 {
            harness.app.step_animations(1.0 / 120.0);
        }
        assert!(
            harness.app.lazy_overscroll_spring.value.abs() < 0.5,
            "Bottom overscroll spring should rebound back to 0.0"
        );
    }

    #[test]
    fn test_animation_speed_slider_and_smoke_fire_borders() {
        let mut harness = IguanodonTestHarness::new(1280.0, 800.0);
        assert!((harness.app.animation_speed - 0.65).abs() < 1e-4);

        // Verify SDF instances include both FireBorder and GentleSmokeBorder elements
        let instances = harness.app.build_sdf_instances();
        let mut has_fire_border = false;
        let mut has_smoke_border = false;
        for inst in &instances {
            let (_, border_eff) = iguanodon_shaders::unpack_material_and_border(inst.params[3]);
            if border_eff == BorderShaderEffect::FireBorder {
                has_fire_border = true;
            } else if border_eff == BorderShaderEffect::GentleSmokeBorder {
                has_smoke_border = true;
            }
        }
        assert!(has_fire_border, "Demo app must showcase FireBorder on selected elements");
        assert!(has_smoke_border, "Demo app must showcase GentleSmokeBorder on selected elements");

        // Drag the animation speed slider from near its left edge (slow) to near its right edge (fast)
        let slider_rect = harness.app.layout.bounds(ANIM_SPEED_SLIDER).unwrap();
        let y_mid = slider_rect.y + slider_rect.height * 0.5;
        harness.on_pointer_down(Point::new(slider_rect.x + 20.0, y_mid), false);
        assert!(harness.app.animation_speed < 0.25);

        harness.on_pointer_move(Point::new(slider_rect.x + slider_rect.width - 20.0, y_mid));
        assert!(harness.app.animation_speed > 1.75);
        harness.on_pointer_up(Point::new(slider_rect.x + slider_rect.width - 20.0, y_mid));

        // Verify animation clock advances proportionally to animation_speed
        harness.app.set_animation_speed(0.25);
        let t0 = harness.app.time_sec;
        harness.app.step_animations(1.0);
        let dt_slow = harness.app.time_sec - t0;

        harness.app.set_animation_speed(1.50);
        let t1 = harness.app.time_sec;
        harness.app.step_animations(1.0);
        let dt_fast = harness.app.time_sec - t1;
        assert!(
            (dt_fast / dt_slow - 6.0).abs() < 0.05,
            "1.50x animation speed must advance shader clocks 6x faster than 0.25x"
        );
    }

    #[test]
    fn test_power_saving_mode_and_renderer_backend_toggle() {
        let mut harness = IguanodonTestHarness::new(1280.0, 800.0);

        // Verify defaults: Performance (120 FPS) and Iguanodon renderer
        assert_eq!(harness.app.power_mode, PowerMode::Performance);
        assert_eq!(harness.app.power_mode.target_fps(), 120);
        assert_eq!(harness.app.renderer_backend, RendererBackend::Iguanodon);

        // Click POWER_BTN -> switches to PowerSaving (30 FPS, suppresses exterior border halos)
        harness.click_node(POWER_BTN);
        assert_eq!(harness.app.power_mode, PowerMode::PowerSaving);
        assert_eq!(harness.app.power_mode.target_fps(), 30);
        let eco_instances = harness.app.build_sdf_instances();
        for inst in &eco_instances {
            let (_, border_eff) = iguanodon_shaders::unpack_material_and_border(inst.params[3]);
            assert_eq!(
                border_eff,
                BorderShaderEffect::None,
                "Power-Saving mode must suppress exterior Fire/Smoke border overdraw halos"
            );
        }

        // Toggle back to Performance
        harness.click_node(POWER_BTN);
        assert_eq!(harness.app.power_mode, PowerMode::Performance);

        // Click RENDERER_BTN -> switches from Iguanodon to Compose-UI comparison mode
        harness.click_node(RENDERER_BTN);
        assert_eq!(harness.app.renderer_backend, RendererBackend::ComposeUi);
        let compose_instances = harness.app.build_sdf_instances();
        for inst in &compose_instances {
            assert!(
                (inst.params[2] - 2.0).abs() < 1e-4,
                "Compose-UI mode must use standard circular-arc corners (p = 2.0)"
            );
        }

        // Toggle back to Iguanodon (default)
        harness.click_node(RENDERER_BTN);
        assert_eq!(harness.app.renderer_backend, RendererBackend::Iguanodon);
    }
}
