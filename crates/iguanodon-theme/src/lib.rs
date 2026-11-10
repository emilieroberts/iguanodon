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

//! Multi-dimensional Theme Engine for Iguanodon.
//! Supports Material 3 Expressive (default), Windows Fluent 2, iOS Cupertino Liquid Glass,
//! and Linux Elite ("Iguanodon Obsidian"), controlling colors, typography, SDF corner geometry,
//! surface optics/shaders, and 120Hz spring motion schemes.

use iguanodon_core::{Color, MotionScheme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ThemeId {
    Material3Expressive = 0,
    WindowsFluent2 = 1,
    IosLiquidGlass = 2,
    LinuxEliteObsidian = 3,
}

impl ThemeId {
    pub const ALL: [ThemeId; 4] = [
        ThemeId::Material3Expressive,
        ThemeId::WindowsFluent2,
        ThemeId::IosLiquidGlass,
        ThemeId::LinuxEliteObsidian,
    ];

    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Material3Expressive,
            1 => Self::WindowsFluent2,
            2 => Self::IosLiquidGlass,
            _ => Self::LinuxEliteObsidian,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Material3Expressive => "Material 3 Expressive",
            Self::WindowsFluent2 => "Windows Fluent 2 (Acrylic/Reveal)",
            Self::IosLiquidGlass => "iOS Cupertino (Liquid Glass)",
            Self::LinuxEliteObsidian => "Linux Elite (Iguanodon Obsidian)",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Material3Expressive => "Material 3",
            Self::WindowsFluent2 => "Fluent 2",
            Self::IosLiquidGlass => "Liquid Glass",
            Self::LinuxEliteObsidian => "Linux Elite",
        }
    }
}

/// Polarity / color-scheme mode (`Dark` or `Light`) available for every [`ThemeId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ThemeMode {
    #[default]
    Dark = 0,
    Light = 1,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 2] = [ThemeMode::Dark, ThemeMode::Light];

    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::Light,
            _ => Self::Dark,
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

/// Signature GPU shader interaction style for a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionShaderKind {
    /// Material 3 Expressive sparkling radial ink ripple.
    MaterialRipple,
    /// Windows Fluent 2 cursor-tracked radial border & fill illumination.
    FluentRevealHighlight,
    /// iOS Cupertino specular rim refraction and luminance shift.
    CupertinoLiquidSpecular,
    /// Linux Elite Obsidian reactive plasma/neon edge pulse.
    ObsidianNeonPulse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPalette {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub background: Color,
    pub surface: Color,
    pub surface_container: Color,
    pub surface_bright: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub accent_glow: Color,
    pub error: Color,
}

impl ColorPalette {
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            primary: self.primary.lerp(other.primary, t),
            on_primary: self.on_primary.lerp(other.on_primary, t),
            primary_container: self.primary_container.lerp(other.primary_container, t),
            on_primary_container: self.on_primary_container.lerp(other.on_primary_container, t),
            secondary: self.secondary.lerp(other.secondary, t),
            tertiary: self.tertiary.lerp(other.tertiary, t),
            background: self.background.lerp(other.background, t),
            surface: self.surface.lerp(other.surface, t),
            surface_container: self.surface_container.lerp(other.surface_container, t),
            surface_bright: self.surface_bright.lerp(other.surface_bright, t),
            on_surface: self.on_surface.lerp(other.on_surface, t),
            on_surface_variant: self.on_surface_variant.lerp(other.on_surface_variant, t),
            outline: self.outline.lerp(other.outline, t),
            accent_glow: self.accent_glow.lerp(other.accent_glow, t),
            error: self.error.lerp(other.error, t),
        }
    }
}

/// SDF geometry tokens controlling corner radii and $L^p$ superellipse curvature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeTokens {
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub radius_xl: f32,
    /// $L^p$ exponent: `2.0` = circular, `4.2` = iOS squircle, `1.2` = chamfer/bevel.
    pub shape_exponent_p: f32,
    /// Pressed-state corner radius morph factor (central to Material 3 Expressive).
    pub pressed_corner_morph: f32,
    pub border_width: f32,
}

impl ShapeTokens {
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let l = |a: f32, b: f32| a + (b - a) * t;
        Self {
            radius_sm: l(self.radius_sm, other.radius_sm),
            radius_md: l(self.radius_md, other.radius_md),
            radius_lg: l(self.radius_lg, other.radius_lg),
            radius_xl: l(self.radius_xl, other.radius_xl),
            shape_exponent_p: l(self.shape_exponent_p, other.shape_exponent_p),
            pressed_corner_morph: l(self.pressed_corner_morph, other.pressed_corner_morph),
            border_width: l(self.border_width, other.border_width),
        }
    }
}

/// Surface optics and backdrop shader parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpticsTokens {
    pub backdrop_blur_px: f32,
    pub surface_translucency: f32,
    pub noise_grain: f32,
    pub specular_refraction: f32,
    pub shadow_elevation_px: f32,
}

impl OpticsTokens {
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let l = |a: f32, b: f32| a + (b - a) * t;
        Self {
            backdrop_blur_px: l(self.backdrop_blur_px, other.backdrop_blur_px),
            surface_translucency: l(self.surface_translucency, other.surface_translucency),
            noise_grain: l(self.noise_grain, other.noise_grain),
            specular_refraction: l(self.specular_refraction, other.specular_refraction),
            shadow_elevation_px: l(self.shadow_elevation_px, other.shadow_elevation_px),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypographyTokens {
    pub display_size: f32,
    pub headline_size: f32,
    pub title_size: f32,
    pub body_size: f32,
    pub label_size: f32,
    pub mono_size: f32,
    pub line_height_multiplier: f32,
}

/// Complete theme descriptor for Iguanodon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeDescriptor {
    pub id: ThemeId,
    pub mode: ThemeMode,
    pub colors: ColorPalette,
    pub shapes: ShapeTokens,
    pub optics: OpticsTokens,
    pub typography: TypographyTokens,
    pub motion: MotionScheme,
    pub interaction_shader: InteractionShaderKind,
}

impl Default for ThemeDescriptor {
    fn default() -> Self {
        Self::material3_expressive()
    }
}

impl ThemeDescriptor {
    pub fn by_id(id: ThemeId) -> Self {
        Self::by_id_and_mode(id, ThemeMode::Dark)
    }

    pub fn by_id_and_mode(id: ThemeId, mode: ThemeMode) -> Self {
        match (id, mode) {
            (ThemeId::Material3Expressive, ThemeMode::Dark) => Self::material3_expressive(),
            (ThemeId::Material3Expressive, ThemeMode::Light) => Self::material3_expressive_light(),
            (ThemeId::WindowsFluent2, ThemeMode::Dark) => Self::windows_fluent2(),
            (ThemeId::WindowsFluent2, ThemeMode::Light) => Self::windows_fluent2_light(),
            (ThemeId::IosLiquidGlass, ThemeMode::Dark) => Self::ios_liquid_glass(),
            (ThemeId::IosLiquidGlass, ThemeMode::Light) => Self::ios_liquid_glass_light(),
            (ThemeId::LinuxEliteObsidian, ThemeMode::Dark) => Self::linux_elite_obsidian(),
            (ThemeId::LinuxEliteObsidian, ThemeMode::Light) => Self::linux_elite_obsidian_light(),
        }
    }

    /// Default skin: Jetpack Compose Multiplatform 1.9+ Material 3 Expressive theme (Dark).
    pub const fn material3_expressive() -> Self {
        Self {
            id: ThemeId::Material3Expressive,
            mode: ThemeMode::Dark,
            colors: ColorPalette {
                primary: Color::from_hex(0x6750A4FF),
                on_primary: Color::from_hex(0xFFFFFFFF),
                primary_container: Color::from_hex(0xEADDFFFF),
                on_primary_container: Color::from_hex(0x21005DFF),
                secondary: Color::from_hex(0x7D5260FF),
                tertiary: Color::from_hex(0x006A6AFF),
                background: Color::from_hex(0x141218FF),
                surface: Color::from_hex(0x1D1B20BA),
                surface_container: Color::from_hex(0x2B2930C8),
                surface_bright: Color::from_hex(0x3B383ED6),
                on_surface: Color::from_hex(0xE6E0E9FF),
                on_surface_variant: Color::from_hex(0xCAC4D0FF),
                outline: Color::from_hex(0x938F99FF),
                accent_glow: Color::from_hex(0xD0BCFFFF),
                error: Color::from_hex(0xF2B8B5FF),
            },
            shapes: ShapeTokens {
                radius_sm: 12.0,
                radius_md: 18.0,
                radius_lg: 28.0,
                radius_xl: 36.0,
                shape_exponent_p: 2.6, // Expressive organic squircle-arc blend
                pressed_corner_morph: 0.55,
                border_width: 1.0,
            },
            optics: OpticsTokens {
                backdrop_blur_px: 14.0,
                surface_translucency: 0.90,
                noise_grain: 0.02,
                specular_refraction: 0.15,
                shadow_elevation_px: 12.0,
            },
            typography: TypographyTokens {
                display_size: 32.0,
                headline_size: 22.0,
                title_size: 17.0,
                body_size: 14.0,
                label_size: 12.5,
                mono_size: 12.0,
                line_height_multiplier: 1.35,
            },
            motion: MotionScheme::expressive(),
            interaction_shader: InteractionShaderKind::MaterialRipple,
        }
    }

    /// Jetpack Compose Multiplatform 1.9+ Material 3 Expressive theme (Light).
    pub const fn material3_expressive_light() -> Self {
        let base = Self::material3_expressive();
        Self {
            id: ThemeId::Material3Expressive,
            mode: ThemeMode::Light,
            colors: ColorPalette {
                primary: Color::from_hex(0x6750A4FF),
                on_primary: Color::from_hex(0xFFFFFFFF),
                primary_container: Color::from_hex(0xEADDFFEC),
                on_primary_container: Color::from_hex(0x21005DFF),
                secondary: Color::from_hex(0x625B71FF),
                tertiary: Color::from_hex(0x006A6AFF),
                background: Color::from_hex(0xFEF7FFFF),
                surface: Color::from_hex(0xFCF8FFC8),
                surface_container: Color::from_hex(0xF1ECF8D6),
                surface_bright: Color::from_hex(0xFFFFFFE6),
                on_surface: Color::from_hex(0x1D1B20FF),
                on_surface_variant: Color::from_hex(0x49454FFF),
                outline: Color::from_hex(0x79747EAA),
                accent_glow: Color::from_hex(0x5632A8FF),
                error: Color::from_hex(0xB3261EFF),
            },
            ..base
        }
    }

    /// Windows 11 Fluent 2 theme with Mica/Acrylic translucency and Reveal Highlight (Dark).
    pub const fn windows_fluent2() -> Self {
        Self {
            id: ThemeId::WindowsFluent2,
            mode: ThemeMode::Dark,
            colors: ColorPalette {
                primary: Color::from_hex(0x60CDFFFF),
                on_primary: Color::from_hex(0x001E30FF),
                primary_container: Color::from_hex(0x004B73FF),
                on_primary_container: Color::from_hex(0xD4F1FFFF),
                secondary: Color::from_hex(0x9EA7B3FF),
                tertiary: Color::from_hex(0x71E5B6FF),
                background: Color::from_hex(0x191B1FFF),
                surface: Color::from_hex(0x22252AB0),
                surface_container: Color::from_hex(0x2C3038C0),
                surface_bright: Color::from_hex(0x383D47D0),
                on_surface: Color::from_hex(0xF5F7FAFF),
                on_surface_variant: Color::from_hex(0xB8C0CCFF),
                outline: Color::from_hex(0x49505CFF),
                accent_glow: Color::from_hex(0x60CDFFFF),
                error: Color::from_hex(0xFF99A4FF),
            },
            shapes: ShapeTokens {
                radius_sm: 4.0,
                radius_md: 8.0,
                radius_lg: 8.0,
                radius_xl: 12.0,
                shape_exponent_p: 2.0, // Standard circular arcs
                pressed_corner_morph: 1.0,
                border_width: 1.0,
            },
            optics: OpticsTokens {
                backdrop_blur_px: 28.0, // Acrylic wide blur
                surface_translucency: 0.78,
                noise_grain: 0.06, // Classic Acrylic noise texture
                specular_refraction: 0.10,
                shadow_elevation_px: 16.0,
            },
            typography: TypographyTokens {
                display_size: 28.0,
                headline_size: 20.0,
                title_size: 15.0,
                body_size: 13.5,
                label_size: 12.0,
                mono_size: 12.0,
                line_height_multiplier: 1.30,
            },
            motion: MotionScheme::crisp_snappy(),
            interaction_shader: InteractionShaderKind::FluentRevealHighlight,
        }
    }

    /// Windows 11 Fluent 2 theme with Light Mica/Acrylic translucency and Reveal Highlight (Light).
    pub const fn windows_fluent2_light() -> Self {
        let base = Self::windows_fluent2();
        Self {
            id: ThemeId::WindowsFluent2,
            mode: ThemeMode::Light,
            colors: ColorPalette {
                primary: Color::from_hex(0x005FB8FF),
                on_primary: Color::from_hex(0xFFFFFFFF),
                primary_container: Color::from_hex(0xD4EBFFEA),
                on_primary_container: Color::from_hex(0x00224AFF),
                secondary: Color::from_hex(0x48586AFF),
                tertiary: Color::from_hex(0x0F7B6CFF),
                background: Color::from_hex(0xF3F6FAFF),
                surface: Color::from_hex(0xF6F9FCC2),
                surface_container: Color::from_hex(0xEBF1F8D2),
                surface_bright: Color::from_hex(0xFFFFFFE4),
                on_surface: Color::from_hex(0x14181FFF),
                on_surface_variant: Color::from_hex(0x444D59FF),
                outline: Color::from_hex(0x7B879699),
                accent_glow: Color::from_hex(0x004E98FF),
                error: Color::from_hex(0xC42B1CFF),
            },
            ..base
        }
    }

    /// iOS Cupertino / Liquid Glass theme with continuous superellipse squircles and refraction (Dark).
    pub const fn ios_liquid_glass() -> Self {
        Self {
            id: ThemeId::IosLiquidGlass,
            mode: ThemeMode::Dark,
            colors: ColorPalette {
                primary: Color::from_hex(0x0A84FFFF),
                on_primary: Color::from_hex(0xFFFFFFFF),
                primary_container: Color::from_hex(0x0A84FF44),
                on_primary_container: Color::from_hex(0xD8ECFFFF),
                secondary: Color::from_hex(0x5E5CE6FF),
                tertiary: Color::from_hex(0x30D158FF),
                background: Color::from_hex(0x0D1017FF),
                surface: Color::from_hex(0x1C212E88),
                surface_container: Color::from_hex(0x282F42A0),
                surface_bright: Color::from_hex(0x3A435CBA),
                on_surface: Color::from_hex(0xF2F5FAFF),
                on_surface_variant: Color::from_hex(0xAEBAD0FF),
                outline: Color::from_hex(0xFFFFFF33),
                accent_glow: Color::from_hex(0x64D2FFFF),
                error: Color::from_hex(0xFF453AFF),
            },
            shapes: ShapeTokens {
                radius_sm: 10.0,
                radius_md: 16.0,
                radius_lg: 24.0,
                radius_xl: 34.0,
                shape_exponent_p: 4.2, // Continuous superellipse squircle
                pressed_corner_morph: 0.94,
                border_width: 0.75,
            },
            optics: OpticsTokens {
                backdrop_blur_px: 36.0,
                surface_translucency: 0.62,
                noise_grain: 0.01,
                specular_refraction: 0.45, // Liquid glass lens refraction
                shadow_elevation_px: 20.0,
            },
            typography: TypographyTokens {
                display_size: 31.0,
                headline_size: 21.0,
                title_size: 16.5,
                body_size: 14.0,
                label_size: 12.0,
                mono_size: 12.0,
                line_height_multiplier: 1.32,
            },
            motion: MotionScheme::cupertino_fluid(),
            interaction_shader: InteractionShaderKind::CupertinoLiquidSpecular,
        }
    }

    /// iOS Cupertino / Liquid Glass theme in Daylight mode with crystal-milk squircles (Light).
    pub const fn ios_liquid_glass_light() -> Self {
        let base = Self::ios_liquid_glass();
        Self {
            id: ThemeId::IosLiquidGlass,
            mode: ThemeMode::Light,
            colors: ColorPalette {
                primary: Color::from_hex(0x007AFFFF),
                on_primary: Color::from_hex(0xFFFFFFFF),
                primary_container: Color::from_hex(0xD6EAFFCC),
                on_primary_container: Color::from_hex(0x002B5CFF),
                secondary: Color::from_hex(0x5856D6FF),
                tertiary: Color::from_hex(0x248A3DFF),
                background: Color::from_hex(0xF2F6FCFF),
                surface: Color::from_hex(0xF5F9FFB2),
                surface_container: Color::from_hex(0xEAF1FCC6),
                surface_bright: Color::from_hex(0xFFFFFFDE),
                on_surface: Color::from_hex(0x11151FFF),
                on_surface_variant: Color::from_hex(0x3C4659FF),
                outline: Color::from_hex(0xFFFFFFB8),
                accent_glow: Color::from_hex(0x0051CCFF),
                error: Color::from_hex(0xD70015FF),
            },
            ..base
        }
    }

    /// Elite Linux theme ("Iguanodon Obsidian" Dark): high-contrast volcanic obsidian glass,
    /// molten Planckian fire borders, crimson red highlight typography, and ultra-snappy response.
    pub const fn linux_elite_obsidian() -> Self {
        Self {
            id: ThemeId::LinuxEliteObsidian,
            mode: ThemeMode::Dark,
            colors: ColorPalette {
                primary: Color::from_hex(0xE62E2EFF), // Molten Crimson Red
                on_primary: Color::from_hex(0xFFFFFFFF),
                primary_container: Color::from_hex(0x5C0E14CC),
                on_primary_container: Color::from_hex(0xFFD6D6FF),
                secondary: Color::from_hex(0xFF6B2BFF), // Solar Flame Orange-Red
                tertiary: Color::from_hex(0xFFB703FF),  // Amber Gold
                background: Color::from_hex(0x090709FF),
                surface: Color::from_hex(0x140E12B4),
                surface_container: Color::from_hex(0x1F151AC4),
                surface_bright: Color::from_hex(0x2E1F26D4),
                on_surface: Color::from_hex(0xFFF2F2FF),
                on_surface_variant: Color::from_hex(0xD4B2B6FF),
                outline: Color::from_hex(0xFF333399),
                accent_glow: Color::from_hex(0xFF3333FF), // Crimson Red Highlight Text & Glow
                error: Color::from_hex(0xFF1E42FF),
            },
            shapes: ShapeTokens {
                radius_sm: 6.0,
                radius_md: 10.0,
                radius_lg: 14.0,
                radius_xl: 18.0,
                shape_exponent_p: 1.35, // Precision chamfer-arc hybrid
                pressed_corner_morph: 0.75,
                border_width: 1.5,
            },
            optics: OpticsTokens {
                backdrop_blur_px: 22.0,
                surface_translucency: 0.72,
                noise_grain: 0.035,
                specular_refraction: 0.25,
                shadow_elevation_px: 18.0,
            },
            typography: TypographyTokens {
                display_size: 30.0,
                headline_size: 20.0,
                title_size: 16.0,
                body_size: 13.5,
                label_size: 12.0,
                mono_size: 12.5,
                line_height_multiplier: 1.30,
            },
            motion: MotionScheme::crisp_snappy(),
            interaction_shader: InteractionShaderKind::ObsidianNeonPulse,
        }
    }

    /// Elite Linux theme ("Solar Alabaster" Light): warm pearl-quartz glass with molten Planckian
    /// fire on every border and high-contrast crimson red highlight text.
    pub const fn linux_elite_obsidian_light() -> Self {
        let base = Self::linux_elite_obsidian();
        Self {
            id: ThemeId::LinuxEliteObsidian,
            mode: ThemeMode::Light,
            colors: ColorPalette {
                primary: Color::from_hex(0xD91E2AFF), // Crimson Red
                on_primary: Color::from_hex(0xFFFFFFFF),
                primary_container: Color::from_hex(0xFFE0E2E6),
                on_primary_container: Color::from_hex(0x4A060DFF),
                secondary: Color::from_hex(0xE85D04FF), // Solar Ember Orange
                tertiary: Color::from_hex(0xF48C06FF),  // Molten Gold
                background: Color::from_hex(0xFFF8F6FF),
                surface: Color::from_hex(0xFFF9F7C6),
                surface_container: Color::from_hex(0xFCEEEBD4),
                surface_bright: Color::from_hex(0xFFFFFFE6),
                on_surface: Color::from_hex(0x1C0D10FF), // Deep Volcanic Espresso-Charcoal
                on_surface_variant: Color::from_hex(0x59353AFF),
                outline: Color::from_hex(0xFF3333B0),     // Triggers Planckian fire on every border
                accent_glow: Color::from_hex(0xC8102EFF), // High-contrast Crimson Red highlight text
                error: Color::from_hex(0xB80C24FF),
            },
            ..base
        }
    }

    /// Interpolates smoothly between two themes for 120Hz live theme transitions.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            id: if t < 0.5 { self.id } else { other.id },
            mode: if t < 0.5 { self.mode } else { other.mode },
            colors: self.colors.lerp(&other.colors, t),
            shapes: self.shapes.lerp(&other.shapes, t),
            optics: self.optics.lerp(&other.optics, t),
            typography: if t < 0.5 {
                self.typography
            } else {
                other.typography
            },
            motion: if t < 0.5 { self.motion } else { other.motion },
            interaction_shader: if t < 0.5 {
                self.interaction_shader
            } else {
                other.interaction_shader
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_four_themes_distinct_and_interpolable() {
        let m3 = ThemeDescriptor::material3_expressive();
        let win = ThemeDescriptor::windows_fluent2();
        let ios = ThemeDescriptor::ios_liquid_glass();
        let linux = ThemeDescriptor::linux_elite_obsidian();

        assert!(m3.shapes.shape_exponent_p > win.shapes.shape_exponent_p);
        assert!(ios.shapes.shape_exponent_p > 4.0);
        assert!(linux.shapes.shape_exponent_p < 1.5);

        let mid = m3.lerp(&linux, 0.5);
        assert!(mid.shapes.shape_exponent_p > linux.shapes.shape_exponent_p);
        assert!(mid.shapes.shape_exponent_p < m3.shapes.shape_exponent_p);

        // Verify both Dark and Light versions of all 4 themes have distinct contrast polarities
        for id in ThemeId::ALL {
            let dark = ThemeDescriptor::by_id_and_mode(id, ThemeMode::Dark);
            let light = ThemeDescriptor::by_id_and_mode(id, ThemeMode::Light);
            assert_eq!(dark.mode, ThemeMode::Dark);
            assert_eq!(light.mode, ThemeMode::Light);
            // Light mode surface is brighter than Dark mode surface; Light mode on_surface is darker
            assert!(light.colors.surface.r > dark.colors.surface.r);
            assert!(light.colors.on_surface.r < dark.colors.on_surface.r);
        }
    }
}
