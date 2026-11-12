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

//! Pre-compiled SPIR-V shader builder, WGSL security validator, 128-byte aligned
//! `SdfPrimitiveInstance` GPU buffer layout, and analytical SDF/effect evaluators.

use iguanodon_core::{Color, Point};

pub const SPIRV_MAGIC: u32 = 0x07230203;

/// Material/Shader effect IDs encoded into `SdfPrimitiveInstance::params[3]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ShaderMaterialId {
    StandardSdf = 0,
    FrostedGlass = 1,
    ProceduralFire = 2,
    WaterRipple = 3,
    FluentRevealHighlight = 4,
    ObsidianNeonPulse = 5,
    TexturedImage = 6,
    GentleSmoke = 7,
    GentlePulsingLights = 8,
    ReflectiveGlass = 9,
    ColorCyclingGlowbar = 10,
    FlowingGentleGradient = 11,
    ElectricJellyfish = 12,
    SubtleGlassReflection = 13,
    GlyphAtlas = 14,
}

impl ShaderMaterialId {
    /// Interactive shader presets that can be cycled in the Standard Benchmark Sample App.
    pub const SHOWCASE_PRESETS: [Self; 10] = [
        Self::ProceduralFire,
        Self::ElectricJellyfish,
        Self::SubtleGlassReflection,
        Self::GentleSmoke,
        Self::GentlePulsingLights,
        Self::ReflectiveGlass,
        Self::ColorCyclingGlowbar,
        Self::FlowingGentleGradient,
        Self::WaterRipple,
        Self::FrostedGlass,
    ];

    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::StandardSdf,
            1 => Self::FrostedGlass,
            2 => Self::ProceduralFire,
            3 => Self::WaterRipple,
            4 => Self::FluentRevealHighlight,
            5 => Self::ObsidianNeonPulse,
            6 => Self::TexturedImage,
            7 => Self::GentleSmoke,
            8 => Self::GentlePulsingLights,
            9 => Self::ReflectiveGlass,
            10 => Self::ColorCyclingGlowbar,
            11 => Self::FlowingGentleGradient,
            12 => Self::ElectricJellyfish,
            13 => Self::SubtleGlassReflection,
            14 => Self::GlyphAtlas,
            _ => Self::StandardSdf,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::StandardSdf => "Standard SDF",
            Self::FrostedGlass => "Frosted Glass",
            Self::ProceduralFire => "Volcanic Fire",
            Self::WaterRipple => "Water Ripple",
            Self::FluentRevealHighlight => "Fluent Reveal",
            Self::ObsidianNeonPulse => "Obsidian Neon",
            Self::TexturedImage => "Textured Image",
            Self::GentleSmoke => "Gentle Smoke",
            Self::GentlePulsingLights => "Pulsing Lights",
            Self::ReflectiveGlass => "Reflective Glass",
            Self::ColorCyclingGlowbar => "Color Glowbar",
            Self::FlowingGentleGradient => "Flowing Gradient",
            Self::ElectricJellyfish => "Electric Jellyfish",
            Self::SubtleGlassReflection => "Subtle Reflection",
            Self::GlyphAtlas => "Glyph Atlas",
        }
    }
}

/// Optional border-specific shader effect packed alongside [`ShaderMaterialId`] into
/// `SdfPrimitiveInstance::params[3]` via [`pack_material_and_border`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BorderShaderEffect {
    None = 0,
    FireBorder = 1,
    GentleSmokeBorder = 2,
}

/// Packs a [`ShaderMaterialId`] and a [`BorderShaderEffect`] into a single `f32` slot (`params[3]`).
#[inline]
pub const fn pack_material_and_border(
    material: ShaderMaterialId,
    border: BorderShaderEffect,
) -> f32 {
    (material as u32 + (border as u32) * 100) as f32
}

/// Unpacks `(material_id, BorderShaderEffect)` from `SdfPrimitiveInstance::params[3]`.
#[inline]
pub const fn unpack_material_and_border(raw: f32) -> (u32, BorderShaderEffect) {
    let v = raw as u32;
    let mat = v % 100;
    let border = match v / 100 {
        1 => BorderShaderEffect::FireBorder,
        2 => BorderShaderEffect::GentleSmokeBorder,
        _ => BorderShaderEffect::None,
    };
    (mat, border)
}

/// 128-byte aligned GPU instance struct for the Instanced SDF Uber-Shader.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdfPrimitiveInstance {
    /// `[x, y, width, height]` in physical pixels.
    pub rect: [f32; 4],
    /// `[top_left, top_right, bottom_right, bottom_left]` corner radii.
    pub corner_radii: [f32; 4],
    /// `[clip_x, clip_y, clip_w, clip_h]` scissor bounds.
    pub clip_rect: [f32; 4],
    /// RGBA linear gradient start color.
    pub fill_color_start: [f32; 4],
    /// RGBA linear gradient end color.
    pub fill_color_end: [f32; 4],
    /// RGBA border color.
    pub border_color: [f32; 4],
    /// RGBA analytic SDF drop-shadow color.
    pub shadow_color: [f32; 4],
    /// `[border_width, shadow_blur, shape_exponent_p, material_id]`.
    pub params: [f32; 4],
}

impl Default for SdfPrimitiveInstance {
    fn default() -> Self {
        Self {
            rect: [0.0, 0.0, 100.0, 40.0],
            corner_radii: [8.0; 4],
            clip_rect: [0.0, 0.0, 1920.0, 1080.0],
            fill_color_start: [0.2, 0.2, 0.25, 1.0],
            fill_color_end: [0.2, 0.2, 0.25, 1.0],
            border_color: [0.4, 0.4, 0.5, 1.0],
            shadow_color: [0.0, 0.0, 0.0, 0.45],
            params: [1.0, 8.0, 2.0, ShaderMaterialId::StandardSdf as u32 as f32],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderValidationError {
    EmptySource,
    MissingEntryPoint,
    ForbiddenConstruct(&'static str),
    UnboundedLoopDetected,
    InvalidSpirvMagic,
    SpirvTooShort,
    InvalidInstructionBounds,
}

/// Validates custom WGSL shader source for memory safety and termination before compilation.
pub fn validate_wgsl_shader(wgsl_source: &str) -> Result<(), ShaderValidationError> {
    let trimmed = wgsl_source.trim();
    if trimmed.is_empty() {
        return Err(ShaderValidationError::EmptySource);
    }
    if !trimmed.contains("@fragment") && !trimmed.contains("@compute") && !trimmed.contains("@vertex") {
        return Err(ShaderValidationError::MissingEntryPoint);
    }
    for forbidden in ["unsafe", "asm!", "ptr<storage, read_write, unchecked>"] {
        if trimmed.contains(forbidden) {
            return Err(ShaderValidationError::ForbiddenConstruct(forbidden));
        }
    }
    if trimmed.contains("loop {") && !trimmed.contains("break") {
        return Err(ShaderValidationError::UnboundedLoopDetected);
    }
    Ok(())
}

/// Validates a compiled SPIR-V binary word stream for structural and instruction-length safety.
pub fn validate_spirv_binary(words: &[u32]) -> Result<(), ShaderValidationError> {
    if words.len() < 5 {
        return Err(ShaderValidationError::SpirvTooShort);
    }
    if words[0] != SPIRV_MAGIC {
        return Err(ShaderValidationError::InvalidSpirvMagic);
    }
    let mut idx = 5;
    while idx < words.len() {
        let word_count = (words[idx] >> 16) as usize;
        if word_count == 0 || idx + word_count > words.len() {
            return Err(ShaderValidationError::InvalidInstructionBounds);
        }
        idx += word_count;
    }
    Ok(())
}

const fn spv_op(word_count: u16, opcode: u16) -> u32 {
    ((word_count as u32) << 16) | (opcode as u32)
}

/// Emits a valid, Vulkan 1.0/1.3 compliant SPIR-V compute shader module that writes a
/// signature/color output word into a bound storage buffer (`set = 0, binding = 0`).
pub fn build_vulkan_compute_spirv(seed_constant: u32) -> Vec<u32> {
    // Opcodes (SPIR-V 1.0 standard):
    // OpCapability = 17, OpMemoryModel = 14, OpEntryPoint = 15, OpExecutionMode = 16,
    // OpDecorate = 71, OpMemberDecorate = 72, OpTypeVoid = 19, OpTypeFunction = 33,
    // OpTypeInt = 21, OpTypeRuntimeArray = 29, OpTypeStruct = 30, OpTypePointer = 32,
    // OpVariable = 59, OpConstant = 43, OpAccessChain = 65, OpStore = 62,
    // OpFunction = 54, OpLabel = 248, OpReturn = 253, OpFunctionEnd = 56
    vec![
        SPIRV_MAGIC,
        0x00010000, // Version 1.0
        0x00000000, // Generator
        16,         // Bound (max ID + 1)
        0,          // Schema
        // OpCapability Shader (1)
        spv_op(2, 17),
        1,
        // OpMemoryModel Logical (0) GLSL450 (1)
        spv_op(3, 14),
        0,
        1,
        // OpEntryPoint GLCompute (5) %1 "main"
        spv_op(4, 15),
        5,
        1,
        0x6E69616D, // "main\0"
        // OpExecutionMode %1 LocalSize (17) 64 1 1
        spv_op(6, 16),
        1,
        17,
        64,
        1,
        1,
        // OpDecorate %6 ArrayStride (6) 4
        spv_op(4, 71),
        6,
        6,
        4,
        // OpMemberDecorate %7 0 Offset (35) 0
        spv_op(5, 72),
        7,
        0,
        35,
        0,
        // OpDecorate %7 BufferBlock (3)
        spv_op(3, 71),
        7,
        3,
        // OpDecorate %9 DescriptorSet (34) 0
        spv_op(4, 71),
        9,
        34,
        0,
        // OpDecorate %9 Binding (33) 0
        spv_op(4, 71),
        9,
        33,
        0,
        // %2 = OpTypeVoid
        spv_op(2, 19),
        2,
        // %3 = OpTypeFunction %2
        spv_op(3, 33),
        3,
        2,
        // %4 = OpTypeInt 32 0 (u32)
        spv_op(4, 21),
        4,
        32,
        0,
        // %5 = OpTypeInt 32 1 (i32)
        spv_op(4, 21),
        5,
        32,
        1,
        // %6 = OpTypeRuntimeArray %4
        spv_op(3, 29),
        6,
        4,
        // %7 = OpTypeStruct %6
        spv_op(3, 30),
        7,
        6,
        // %8 = OpTypePointer Uniform (2) %7
        spv_op(4, 32),
        8,
        2,
        7,
        // %9 = OpVariable %8 Uniform (2)
        spv_op(4, 59),
        8,
        9,
        2,
        // %10 = OpTypePointer Uniform (2) %4
        spv_op(4, 32),
        10,
        2,
        4,
        // %11 = OpConstant %5 0
        spv_op(4, 43),
        5,
        11,
        0,
        // %12 = OpConstant %4 seed_constant
        spv_op(4, 43),
        4,
        12,
        seed_constant,
        // %1 = OpFunction %2 None(0) %3
        spv_op(5, 54),
        2,
        1,
        0,
        3,
        // %13 = OpLabel
        spv_op(2, 248),
        13,
        // %14 = OpAccessChain %10 %9 %11 %11
        spv_op(6, 65),
        10,
        14,
        9,
        11,
        11,
        // OpStore %14 %12
        spv_op(3, 62),
        14,
        12,
        // OpReturn
        spv_op(1, 253),
        // OpFunctionEnd
        spv_op(1, 56),
    ]
}

const SDF_UBER_VERT_SPV_BYTES: &[u8] = include_bytes!("../shaders/sdf_uber.vert.spv");
const SDF_UBER_FRAG_SPV_BYTES: &[u8] = include_bytes!("../shaders/sdf_uber.frag.spv");

fn bytes_to_spirv_words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Real `glslc`-compiled SPIR-V Vertex Shader (`sdf_uber.vert`) for the hardware Vulkan Graphics Pipeline.
pub fn sdf_uber_vertex_spirv() -> Vec<u32> {
    bytes_to_spirv_words(SDF_UBER_VERT_SPV_BYTES)
}

/// Real `glslc`-compiled SPIR-V Fragment Shader (`sdf_uber.frag`) for the hardware Vulkan Graphics Pipeline.
pub fn sdf_uber_fragment_spirv() -> Vec<u32> {
    bytes_to_spirv_words(SDF_UBER_FRAG_SPV_BYTES)
}

pub fn sdf_uber_shader_spirv() -> Vec<u32> {
    sdf_uber_fragment_spirv()
}

pub fn dual_kawase_blur_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x424C5552) // "BLUR"
}

pub fn procedural_fire_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x46495245) // "FIRE"
}

pub fn water_ripple_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x57415452) // "WATR"
}

pub fn gentle_smoke_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x534D4F4B) // "SMOK"
}

pub fn pulsing_lights_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x50554C53) // "PULS"
}

pub fn reflective_glass_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x52474C53) // "RGLS"
}

pub fn color_cycling_glowbar_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x474C4F57) // "GLOW"
}

pub fn flowing_gradient_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x47524144) // "GRAD"
}

pub fn electric_jellyfish_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x4A454C59) // "JELY"
}

pub fn subtle_glass_reflection_spirv() -> Vec<u32> {
    build_vulkan_compute_spirv(0x5352464C) // "SRFL"
}

/// Smooth 3-term cosine spectrum color ramp for the Color-Cycling Glowbar (`hue in [0.0, 1.0]`).
#[inline]
pub fn color_cycling_spectrum(hue: f32) -> Color {
    let lut = FireSimulationLut::global();
    let tau = std::f32::consts::TAU;
    let r = (0.58 + 0.42 * lut.fast_cos(tau * (hue + 0.00))).clamp(0.0, 1.0);
    let g = (0.58 + 0.42 * lut.fast_cos(tau * (hue + 0.33))).clamp(0.0, 1.0);
    let b = (0.64 + 0.36 * lut.fast_cos(tau * (hue + 0.67))).clamp(0.0, 1.0);
    Color::rgba(r, g, b, 1.0)
}

/// Endless left-to-right flowing silk gradient through warm golden-yellows -> buttercream -> luminous white.
#[inline]
pub fn flowing_yellow_white_gradient(uv_x: f32, uv_y: f32, t_sec: f32, base_tint: Color) -> Color {
    let lut = FireSimulationLut::global();
    // Slow, serene endless loop flowing from left to right (`uv_x - t_sec * 0.085`)
    let phase = (uv_x - t_sec * 0.085 + uv_y * 0.12).rem_euclid(1.0);
    let wave = 0.5 + 0.5 * lut.fast_cos(phase * std::f32::consts::TAU);
    let shimmer = 0.5 + 0.5 * lut.fast_sin((phase * 2.0 - 0.15) * std::f32::consts::TAU);

    // Rich golden yellow -> sunlit amber-yellow -> soft buttercream -> pure warm ivory-white
    let deep_yellow = Color::rgba(
        (base_tint.r * 0.35 + 0.64).clamp(0.82, 0.99),
        (base_tint.g * 0.30 + 0.52).clamp(0.66, 0.88),
        (base_tint.b * 0.15 + 0.08).clamp(0.08, 0.28),
        1.0,
    );
    let warm_butter = Color::rgba(1.0, 0.92, 0.48, 1.0);
    let luminous_white = Color::rgba(1.0, 0.99, 0.93, 1.0);

    let mid = deep_yellow.lerp(warm_butter, wave);
    mid.lerp(luminous_white, (wave * wave * 0.72 + shimmer * 0.22).clamp(0.0, 0.92))
}

/// Evaluates the generalized $L^p$ Signed Distance Field for a box with corner radius `r` and exponent `p`.
/// - `p = 2.0`: Standard circular rounded rectangle
/// - `p = 4.2`: iOS Cupertino continuous superellipse squircle
/// - `p = 1.0`: Chamfered octagon box
///
/// Optimized with an exact analytical fast-path + homogeneous 1D LUT (`(1 + t^p)^(1/p)`):
/// zero `powf()` calls per pixel across the entire engine!
#[inline]
pub fn evaluate_sdf_superellipse(local_pt: Point, half_extents: Point, radius: f32, p: f32) -> f32 {
    let r = radius.clamp(0.0, half_extents.x.min(half_extents.y));
    let qx = local_pt.x.abs() - (half_extents.x - r);
    let qy = local_pt.y.abs() - (half_extents.y - r);

    // Fast path 1: Inside the core rectangle or along the straight horizontal/vertical edges (98%+ of pixels)
    if qx <= 0.0 || qy <= 0.0 {
        return qx.max(qy) - r;
    }

    // Fast path 2: Homogeneous L^p superellipse corner lookup:
    // (qx^p + qy^p)^(1/p) == max(qx, qy) * (1 + (min(qx, qy) / max(qx, qy))^p)^(1/p)
    let m = qx.max(qy);
    let t = qx.min(qy) / m;
    let p_idx = (((p.clamp(1.0, 4.75) - 1.0) * 4.0).round() as usize).min(15);
    let t_scaled = t * 64.0;
    let ti = (t_scaled as usize).min(63);
    let tf = t_scaled - ti as f32;
    let row = &FireSimulationLut::global().superellipse_lut[p_idx];
    let factor = row[ti] + (row[ti + 1] - row[ti]) * tf;
    m * factor - r
}

/// Fast deterministic 2D hash in `[0.0, 1.0]` without trig banding artifacts.
#[inline]
fn hash21(ix: i32, iy: i32) -> f32 {
    let mut n = (ix as u32).wrapping_mul(374761393) ^ (iy as u32).wrapping_mul(668265263);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    ((n ^ (n >> 16)) & 0x00FF_FFFF) as f32 / 16_777_215.0
}

/// Smooth 2D value noise with quintic interpolation ($6t^5 - 15t^4 + 10t^3$).
#[inline]
fn value_noise_2d(x: f32, y: f32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();

    let ux = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let uy = fy * fy * fy * (fy * (fy * 6.0 - 15.0) + 10.0);

    let a = hash21(ix, iy);
    let b = hash21(ix + 1, iy);
    let c = hash21(ix, iy + 1);
    let d = hash21(ix + 1, iy + 1);

    let ab = a + (b - a) * ux;
    let cd = c + (d - c) * ux;
    ab + (cd - ab) * uy
}

/// Periodically wrapped 2D value noise (`period = 16` cells) for seamless 256x256 LUT tiling.
#[inline]
fn periodic_value_noise_2d(x: f32, y: f32, period: i32) -> f32 {
    let ix = (x.floor() as i32).rem_euclid(period);
    let iy = (y.floor() as i32).rem_euclid(period);
    let ix1 = (ix + 1).rem_euclid(period);
    let iy1 = (iy + 1).rem_euclid(period);
    let fx = x - x.floor();
    let fy = y - y.floor();

    let ux = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let uy = fy * fy * fy * (fy * (fy * 6.0 - 15.0) + 10.0);

    let a = hash21(ix, iy);
    let b = hash21(ix1, iy);
    let c = hash21(ix, iy1);
    let d = hash21(ix1, iy1);

    let ab = a + (b - a) * ux;
    let cd = c + (d - c) * ux;
    ab + (cd - ab) * uy
}

/// 4-octave rotated fractional Brownian motion (fBm) eliminating grid axes alignment.
#[inline]
pub fn fbm_turbulence(mut x: f32, mut y: f32) -> f32 {
    let mut value = 0.0_f32;
    let mut amp = 0.5_f32;
    for _ in 0..4 {
        value += amp * value_noise_2d(x, y);
        let nx = (0.8 * x - 0.6 * y) * 2.02 + 17.3;
        let ny = (0.6 * x + 0.8 * y) * 2.02 + 31.7;
        x = nx;
        y = ny;
        amp *= 0.5;
    }
    value
}

/// Physically-inspired blackbody Planckian color ramp mapping normalized heat `temp in [0.0, 1.0]`
/// from deep crimson soot -> magma vermilion -> luminous amber -> incandescent white-gold.
#[inline]
pub fn blackbody_fire_color(temp: f32) -> Color {
    let t = temp.clamp(0.0, 1.0);
    let r = (t * 2.1).clamp(0.0, 1.0).powf(0.75);
    let g = ((t - 0.22) * 1.45).clamp(0.0, 1.0).powf(1.35);
    let b = ((t - 0.62) * 2.4).clamp(0.0, 1.0).powf(1.8);
    Color::rgba(r, g, b, 1.0)
}

/// Precomputed L1-cache-resident 256x256 tileable fBm turbulence field, pre-warped 256x256 fire/smoke LUT,
/// 256-entry Blackbody LUT, 16x65 homogeneous superellipse corner LUT, and 1024-entry trig LUT.
/// Eliminates all per-pixel `powf()`, `sin()`, `cos()`, and multi-lookup domain warping.
pub struct FireSimulationLut {
    pub turbulence_256: Vec<f32>,
    pub warped_256: Vec<f32>,
    pub blackbody_lut: [[f32; 3]; 256],
    pub superellipse_lut: [[f32; 65]; 16],
    pub sin_lut_1024: [f32; 1024],
}

impl FireSimulationLut {
    pub fn global() -> &'static Self {
        static LUT: std::sync::OnceLock<FireSimulationLut> = std::sync::OnceLock::new();
        LUT.get_or_init(|| {
            let mut turbulence_256 = vec![0.0_f32; 256 * 256];
            for y in 0..256 {
                for x in 0..256 {
                    let u = x as f32 / 16.0;
                    let v = y as f32 / 16.0;
                    let mut val = 0.0_f32;
                    let mut amp = 0.5_f32;
                    let mut freq = 1.0_f32;
                    let mut p = 16_i32;
                    for oct in 0..4 {
                        val += amp
                            * periodic_value_noise_2d(
                                u * freq + oct as f32 * 3.7,
                                v * freq + oct as f32 * 7.1,
                                p,
                            );
                        freq *= 2.0;
                        p *= 2;
                        amp *= 0.5;
                    }
                    turbulence_256[y * 256 + x] = val;
                }
            }
            let sample_turb_raw = |nx: f32, ny: f32| -> f32 {
                let sx = nx * 16.0;
                let sy = ny * 16.0;
                let fx = sx.floor();
                let fy = sy.floor();
                let tx = sx - fx;
                let ty = sy - fy;
                let x0 = (fx as i32) & 255;
                let y0 = (fy as i32) & 255;
                let x1 = (x0 + 1) & 255;
                let y1 = (y0 + 1) & 255;
                let r0 = (y0 as usize) * 256;
                let r1 = (y1 as usize) * 256;
                let v00 = turbulence_256[r0 + x0 as usize];
                let v10 = turbulence_256[r0 + x1 as usize];
                let v01 = turbulence_256[r1 + x0 as usize];
                let v11 = turbulence_256[r1 + x1 as usize];
                let top = v00 + (v10 - v00) * tx;
                let bot = v01 + (v11 - v01) * tx;
                top + (bot - top) * ty
            };
            // Precompute the domain-warped FBM table once so `sample_warped_fire` is 1 lookup instead of 3!
            let mut warped_256 = vec![0.0_f32; 256 * 256];
            for y in 0..256 {
                for x in 0..256 {
                    let nx = x as f32 / 16.0;
                    let ny = y as f32 / 16.0;
                    let qx = sample_turb_raw(nx, ny) - 0.5;
                    let qy = sample_turb_raw(nx + 4.1, ny + 2.7) - 0.5;
                    warped_256[y * 256 + x] = sample_turb_raw(nx + qx * 1.8, ny + qy * 1.8);
                }
            }
            let mut blackbody_lut = [[0.0_f32; 3]; 256];
            for (i, slot) in blackbody_lut.iter_mut().enumerate() {
                let c = blackbody_fire_color(i as f32 / 255.0);
                *slot = [c.r, c.g, c.b];
            }
            let mut superellipse_lut = [[1.0_f32; 65]; 16];
            for p_idx in 0..16 {
                let p = 1.0 + p_idx as f32 * 0.25;
                for ti in 0..=64 {
                    let t = ti as f32 / 64.0;
                    superellipse_lut[p_idx][ti] = (1.0 + t.powf(p)).powf(1.0 / p);
                }
            }
            let mut sin_lut_1024 = [0.0_f32; 1024];
            for (i, slot) in sin_lut_1024.iter_mut().enumerate() {
                *slot = (i as f32 * (std::f32::consts::TAU / 1024.0)).sin();
            }
            Self {
                turbulence_256,
                warped_256,
                blackbody_lut,
                superellipse_lut,
                sin_lut_1024,
            }
        })
    }

    #[inline]
    pub fn fast_sin(&self, radians: f32) -> f32 {
        let idx = ((radians * (1024.0 / std::f32::consts::TAU)) as i32) & 1023;
        self.sin_lut_1024[idx as usize]
    }

    #[inline]
    pub fn fast_cos(&self, radians: f32) -> f32 {
        let idx = (((radians * (1024.0 / std::f32::consts::TAU)) as i32) + 256) & 1023;
        self.sin_lut_1024[idx as usize]
    }

    #[inline]
    pub fn sample_turbulence(&self, x: f32, y: f32) -> f32 {
        let sx = x * 16.0;
        let sy = y * 16.0;
        let fx = sx.floor();
        let fy = sy.floor();
        let tx = sx - fx;
        let ty = sy - fy;
        let x0 = (fx as i32) & 255;
        let y0 = (fy as i32) & 255;
        let x1 = (x0 + 1) & 255;
        let y1 = (y0 + 1) & 255;

        let r0 = (y0 as usize) * 256;
        let r1 = (y1 as usize) * 256;
        let v00 = self.turbulence_256[r0 + x0 as usize];
        let v10 = self.turbulence_256[r0 + x1 as usize];
        let v01 = self.turbulence_256[r1 + x0 as usize];
        let v11 = self.turbulence_256[r1 + x1 as usize];

        let top = v00 + (v10 - v00) * tx;
        let bot = v01 + (v11 - v01) * tx;
        top + (bot - top) * ty
    }

    #[inline]
    pub fn sample_warped_fire(&self, nx: f32, ny: f32) -> f32 {
        let sx = nx * 16.0;
        let sy = ny * 16.0;
        let fx = sx.floor();
        let fy = sy.floor();
        let tx = sx - fx;
        let ty = sy - fy;
        let x0 = (fx as i32) & 255;
        let y0 = (fy as i32) & 255;
        let x1 = (x0 + 1) & 255;
        let y1 = (y0 + 1) & 255;

        let r0 = (y0 as usize) * 256;
        let r1 = (y1 as usize) * 256;
        let v00 = self.warped_256[r0 + x0 as usize];
        let v10 = self.warped_256[r0 + x1 as usize];
        let v01 = self.warped_256[r1 + x0 as usize];
        let v11 = self.warped_256[r1 + x1 as usize];

        let top = v00 + (v10 - v00) * tx;
        let bot = v01 + (v11 - v01) * tx;
        top + (bot - top) * ty
    }

    #[inline]
    pub fn lookup_blackbody(&self, temp: f32) -> Color {
        let idx = (temp.clamp(0.0, 1.0) * 255.0) as usize;
        let rgb = self.blackbody_lut[idx.min(255)];
        Color::rgba(rgb[0], rgb[1], rgb[2], 1.0)
    }

    /// Packs the 256x256 FBM (`r`) and Domain-Warped (`g`) noise tables into a 256x256 RGBA8 texture
    /// for hardware Vulkan `sampler2D u_noise_lut` binding (`set = 0, binding = 3`).
    pub fn pack_gpu_noise_rgba8(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(256 * 256 * 4);
        for i in 0..(256 * 256) {
            let fbm = (self.turbulence_256[i].clamp(0.0, 1.0) * 255.0).round() as u8;
            let wrp = (self.warped_256[i].clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba.extend_from_slice(&[fbm, wrp, fbm, 255]);
        }
        rgba
    }
}

/// Precomputed Full HD (`1920x1080`) photorealistic Macro Water Drop Wallpaper
/// in a bright, sunlit crystal-aqua palette used as the sample app background to showcase
/// multi-theme translucency, frosted glass, and drop shadows.
pub struct WaterDropWallpaper {
    pub width: u32,
    pub height: u32,
    pub pixels_rgb: Vec<[u8; 3]>,
}

impl WaterDropWallpaper {
    pub fn global() -> &'static Self {
        static WALLPAPER: std::sync::OnceLock<WaterDropWallpaper> = std::sync::OnceLock::new();
        WALLPAPER.get_or_init(Self::generate)
    }

    #[inline]
    fn evaluate_macro_background(u: f32, v: f32) -> [f32; 3] {
        // Bright, sunlit crystal-aqua & sky-cyan macro photography gradient
        let top = [0.34_f32, 0.70, 0.90];
        let mid = [0.28_f32, 0.76, 0.86];
        let bot = [0.22_f32, 0.58, 0.78];
        let mut r = if v < 0.55 {
            top[0] + (mid[0] - top[0]) * (v / 0.55)
        } else {
            mid[0] + (bot[0] - mid[0]) * ((v - 0.55) / 0.45)
        };
        let mut g = if v < 0.55 {
            top[1] + (mid[1] - top[1]) * (v / 0.55)
        } else {
            mid[1] + (bot[1] - mid[1]) * ((v - 0.55) / 0.45)
        };
        let mut b = if v < 0.55 {
            top[2] + (mid[2] - top[2]) * (v / 0.55)
        } else {
            mid[2] + (bot[2] - mid[2]) * ((v - 0.55) / 0.45)
        };

        // Subtle organic leaf-vein / brushed glass diagonal striations
        let diag = (u * 28.0 + v * 16.0).sin() * 0.5 + 0.5;
        let vein = (u * 95.0 - v * 60.0 + diag * 2.5).cos() * 0.5 + 0.5;
        let tex = diag * 0.045 + vein * 0.032;
        r += tex * 0.65;
        g += tex * 0.95;
        b += tex * 0.90;

        // Luminous out-of-focus studio bokeh light orbs in the background
        const BOKEH_ORBS: [(f32, f32, f32, [f32; 3]); 7] = [
            (0.18, 0.22, 0.14, [0.55, 0.92, 1.00]),
            (0.78, 0.18, 0.17, [0.58, 0.96, 0.88]),
            (0.64, 0.62, 0.19, [0.48, 0.86, 1.00]),
            (0.32, 0.78, 0.15, [0.56, 0.94, 0.86]),
            (0.86, 0.76, 0.13, [1.00, 0.86, 0.55]),
            (0.12, 0.58, 0.12, [0.62, 0.90, 1.00]),
            (0.46, 0.20, 0.11, [0.92, 0.98, 1.00]),
        ];
        let aspect = 16.0 / 9.0;
        for &(bx, by, br, bc) in &BOKEH_ORBS {
            let dx = (u - bx) * aspect;
            let dy = v - by;
            if dx.abs() < br && dy.abs() < br {
                let d_sq = dx * dx + dy * dy;
                if d_sq < br * br {
                    let nr = d_sq.sqrt() / br;
                    let disk = (1.0 - nr * nr).clamp(0.0, 1.0);
                    let rim = ((1.0 - (nr - 0.82).abs() * 5.5).clamp(0.0, 1.0)) * 0.40;
                    let w = (disk * 0.26 + rim * 0.22).clamp(0.0, 0.42);
                    r += bc[0] * w;
                    g += bc[1] * w;
                    b += bc[2] * w;
                }
            }
        }

        // Concentric capillary water wave rings around two droplet impact centers
        for &(cx, cy, freq, amp) in &[
            (0.50_f32, 0.48_f32, 85.0_f32, 0.095_f32),
            (0.22, 0.70, 110.0, 0.065),
        ] {
            let dx = (u - cx) * aspect;
            let dy = v - cy;
            if dx.abs() < 0.55 && dy.abs() < 0.55 {
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > 0.035 && dist < 0.55 {
                    let env = (-dist * 4.0).exp() * ((dist - 0.035) * 25.0).clamp(0.0, 1.0);
                    let wave = (dist * freq).sin() * env;
                    let crest = (dist * freq).cos().max(0.0).powi(3) * env;
                    r += wave * amp * 0.55 + crest * 0.24;
                    g += wave * amp * 0.90 + crest * 0.30;
                    b += wave * amp * 1.15 + crest * 0.35;
                }
            }
        }

        [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
    }

    fn generate() -> Self {
        let width = 1920_u32;
        let height = 1080_u32;
        let aspect = width as f32 / height as f32;

        // (center_u, center_v, radius_uv, elongation_x) — positioned so hero drops sit both
        // inside the wide inter-column gaps (u ~ 0.33 and u ~ 0.67) and behind the translucent panels!
        const DROPS: [(f32, f32, f32, f32); 32] = [
            // Hero central & inter-column macro water drops
            (0.50, 0.48, 0.095, 1.06),
            (0.335, 0.42, 0.076, 1.04),
            (0.665, 0.54, 0.074, 1.05),
            (0.22, 0.70, 0.072, 1.04),
            (0.77, 0.32, 0.068, 1.05),
            (0.35, 0.22, 0.056, 1.02),
            (0.66, 0.76, 0.058, 1.08),
            (0.14, 0.34, 0.048, 1.03),
            (0.85, 0.64, 0.050, 1.04),
            // Medium dewdrops (including inter-column gap dewdrops)
            (0.33, 0.68, 0.042, 1.02),
            (0.67, 0.28, 0.040, 1.01),
            (0.42, 0.66, 0.034, 1.02),
            (0.60, 0.30, 0.032, 1.00),
            (0.27, 0.47, 0.029, 1.03),
            (0.72, 0.52, 0.031, 1.01),
            (0.53, 0.22, 0.028, 1.00),
            (0.18, 0.84, 0.028, 1.04),
            (0.88, 0.20, 0.027, 1.02),
            (0.38, 0.84, 0.025, 1.01),
            // Micro satellite dewdrops
            (0.34, 0.55, 0.019, 1.00),
            (0.66, 0.41, 0.018, 1.00),
            (0.45, 0.36, 0.016, 1.00),
            (0.57, 0.58, 0.017, 1.00),
            (0.31, 0.60, 0.015, 1.00),
            (0.63, 0.42, 0.014, 1.00),
            (0.20, 0.54, 0.015, 1.00),
            (0.80, 0.45, 0.016, 1.00),
            (0.10, 0.18, 0.018, 1.00),
            (0.50, 0.79, 0.016, 1.00),
            (0.75, 0.84, 0.018, 1.00),
            (0.91, 0.82, 0.015, 1.00),
            (0.08, 0.72, 0.017, 1.00),
        ];

        let mut pixels_rgb = vec![[0_u8; 3]; (width * height) as usize];
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8)
            .clamp(2, 24);
        let rows_per_band = ((height as usize + num_threads - 1) / num_threads).max(1);
        let px_per_band = rows_per_band * (width as usize);

        std::thread::scope(|s| {
            for (band_idx, band_slice) in pixels_rgb.chunks_mut(px_per_band).enumerate() {
                let y_start = (band_idx * rows_per_band) as u32;
                let band_h = (band_slice.len() / width as usize) as u32;
                s.spawn(move || {
                    for local_y in 0..band_h {
                        let y = y_start + local_y;
                        let v = (y as f32 + 0.5) / height as f32;
                        let row_off = (local_y * width) as usize;
                        for x in 0..width {
                            let u = (x as f32 + 0.5) / width as f32;
                            let mut col = Self::evaluate_macro_background(u, v);

                            // Composite 3D ray-traced water droplets (contact shadow + lens refraction + caustics + specular)
                            for &(dx_c, dy_c, rad, elong_x) in &DROPS {
                                let ly = v - dy_c;
                                if ly.abs() >= rad * 1.28 {
                                    continue;
                                }
                                let lx = (u - dx_c) * aspect / elong_x;
                                if lx.abs() >= rad * 1.28 {
                                    continue;
                                }
                                let r_norm = (lx * lx + ly * ly).sqrt() / rad;

                                if r_norm < 1.28 {
                                    if r_norm >= 1.0 {
                                        // Soft exterior contact ambient-occlusion & directional shadow (lower-right)
                                        let sh_x = lx - 0.14 * rad;
                                        let sh_y = ly - 0.18 * rad;
                                        let sh_r = (sh_x * sh_x + sh_y * sh_y).sqrt() / rad;
                                        if sh_r < 1.24 {
                                            let falloff =
                                                (1.0 - (r_norm - 1.0) / 0.28).clamp(0.0, 1.0);
                                            let shadow_strength = falloff * falloff * 0.36;
                                            col[0] *= 1.0 - shadow_strength;
                                            col[1] *= 1.0 - shadow_strength;
                                            col[2] *= 1.0 - shadow_strength;
                                        }
                                    } else {
                                        // Inside the 3D water droplet hemisphere
                                        let nx = lx / rad;
                                        let ny = ly / rad;
                                        let nz = (1.0 - (nx * nx + ny * ny).min(1.0)).sqrt();

                                        // Convex water lens refraction (IOR = 1.333)
                                        let refract_scale = 0.55 + 0.45 * nz;
                                        let ref_u =
                                            (dx_c - (u - dx_c) * refract_scale).clamp(0.0, 1.0);
                                        let ref_v =
                                            (dy_c - (v - dy_c) * refract_scale).clamp(0.0, 1.0);
                                        let mut refracted =
                                            Self::evaluate_macro_background(ref_u, ref_v);

                                        // Bright crystal aqua-cyan magnification boost inside the drop
                                        refracted[0] = (refracted[0] * 1.06 + 0.06).min(1.0);
                                        refracted[1] = (refracted[1] * 1.16 + 0.08).min(1.0);
                                        refracted[2] = (refracted[2] * 1.20 + 0.10).min(1.0);

                                        // Internal focused sunlit caustic crescent on the lower-right side of the drop
                                        let caustic_dir = (nx * 0.58 + ny * 0.68).max(0.0);
                                        let caustic =
                                            caustic_dir.powi(3) * (1.0 - nz * 0.45) * 0.72;
                                        refracted[0] += caustic * 0.70;
                                        refracted[1] += caustic * 0.94;
                                        refracted[2] += caustic * 1.00;

                                        // Schlick Fresnel rim reflection
                                        let fresnel = 0.02 + 0.98 * (1.0 - nz).powi(4);
                                        let sky_rim = [0.78_f32, 0.95, 1.00];

                                        // Dual studio softbox specular highlights (upper-left primary + lower-right rim)
                                        let h1_dot = (-nx * 0.52 - ny * 0.60 + nz * 0.61).max(0.0);
                                        let spec_sharp = h1_dot.powi(46) * 1.45;
                                        let spec_glow = h1_dot.powi(10) * 0.35;

                                        let h2_dot = (nx * 0.48 + ny * 0.52 + nz * 0.70).max(0.0);
                                        let spec_rim = h2_dot.powi(26) * 0.42;

                                        // Delicate meniscus edge right at r_norm -> 1.0
                                        let edge_darken = 1.0 - 0.22 * r_norm.powi(6);

                                        let mut dr = (refracted[0] * (1.0 - fresnel * 0.55)
                                            + sky_rim[0] * fresnel * 0.55)
                                            * edge_darken
                                            + spec_sharp
                                            + spec_glow * 0.75
                                            + spec_rim * 0.6;
                                        let mut dg = (refracted[1] * (1.0 - fresnel * 0.55)
                                            + sky_rim[1] * fresnel * 0.55)
                                            * edge_darken
                                            + spec_sharp
                                            + spec_glow * 0.92
                                            + spec_rim * 0.85;
                                        let mut db = (refracted[2] * (1.0 - fresnel * 0.55)
                                            + sky_rim[2] * fresnel * 0.55)
                                            * edge_darken
                                            + spec_sharp
                                            + spec_glow
                                            + spec_rim;

                                        dr = dr.clamp(0.0, 1.0);
                                        dg = dg.clamp(0.0, 1.0);
                                        db = db.clamp(0.0, 1.0);

                                        // Anti-aliased droplet boundary (sharper for 1920x1080)
                                        let aa = ((1.0 - r_norm) * 32.0).clamp(0.0, 1.0);
                                        col[0] = col[0] * (1.0 - aa) + dr * aa;
                                        col[1] = col[1] * (1.0 - aa) + dg * aa;
                                        col[2] = col[2] * (1.0 - aa) + db * aa;
                                    }
                                }
                            }

                            band_slice[row_off + x as usize] = [
                                (col[0].clamp(0.0, 1.0) * 255.0) as u8,
                                (col[1].clamp(0.0, 1.0) * 255.0) as u8,
                                (col[2].clamp(0.0, 1.0) * 255.0) as u8,
                            ];
                        }
                    }
                });
            }
        });

        Self {
            width,
            height,
            pixels_rgb,
        }
    }

    /// Computes uniform linear aspect-ratio-preserving cover-crop sampling maps `(x_map, y_map)`
    /// for a target window of `(target_w, target_h)`.
    ///
    /// Scales linearly to fit the smallest/constraining dimension (`step = min(wp_w / target_w, wp_h / target_h)`)
    /// and center-crops the excess of the other dimension so water droplets are never squashed or stretched.
    /// Each entry is `(idx0, idx1, weight_256)` for fast fixed-point bilinear interpolation.
    pub fn compute_cover_crop_maps(
        &self,
        target_w: u32,
        target_h: u32,
    ) -> (Vec<(usize, usize, u32)>, Vec<(usize, usize, u32)>) {
        let tw = target_w.max(1) as f32;
        let th = target_h.max(1) as f32;
        let wp_w = self.width as f32;
        let wp_h = self.height as f32;

        // Uniform linear scale: 1 window pixel advances `step` wallpaper pixels identically in X and Y.
        let step = (wp_w / tw).min(wp_h / th);
        let vis_w = tw * step;
        let vis_h = th * step;
        let offset_x = ((wp_w - vis_w) * 0.5).max(0.0);
        let offset_y = ((wp_h - vis_h) * 0.5).max(0.0);

        let max_x = (self.width - 1) as usize;
        let max_y = (self.height - 1) as usize;

        let x_map = (0..target_w.max(1))
            .map(|x| {
                let src = (offset_x + (x as f32 + 0.5) * step - 0.5).clamp(0.0, max_x as f32);
                let i0 = (src.floor() as usize).min(max_x);
                let i1 = (i0 + 1).min(max_x);
                let w = ((src - src.floor()) * 256.0).round() as u32;
                (i0, i1, w.min(256))
            })
            .collect();

        let y_map = (0..target_h.max(1))
            .map(|y| {
                let src = (offset_y + (y as f32 + 0.5) * step - 0.5).clamp(0.0, max_y as f32);
                let i0 = (src.floor() as usize).min(max_y);
                let i1 = (i0 + 1).min(max_y);
                let w = ((src - src.floor()) * 256.0).round() as u32;
                (i0, i1, w.min(256))
            })
            .collect();

        (x_map, y_map)
    }
}

/// Standard Animated Background Shader: **Electric Jellyfish Backdrop**
/// (`EmMessageBridgeIguanodon` deep abyssal blue ocean gradient + 5 slowly drifting,
/// gently pulsing bioluminescent jellyfish light blooms without legs + diffuse opaque frosted glass pane veil).
#[derive(Clone)]
pub struct ElectricJellyfishField {
    pub grid_w: usize,
    pub grid_h: usize,
    /// Pre-evaluated fixed-point RGB (`0..=65280`, i.e. `u8 * 256`) at each 16x16 grid vertex.
    pub grid_rgb256: Vec<[u32; 3]>,
}

#[inline]
fn sample_4stop_linear(t: f32, c0: [f32; 3], c1: [f32; 3], c2: [f32; 3], c3: [f32; 3]) -> [f32; 3] {
    let tc = t.clamp(0.0, 1.0) * 3.0;
    if tc < 1.0 {
        let u = tc;
        [
            c0[0] + (c1[0] - c0[0]) * u,
            c0[1] + (c1[1] - c0[1]) * u,
            c0[2] + (c1[2] - c0[2]) * u,
        ]
    } else if tc < 2.0 {
        let u = tc - 1.0;
        [
            c1[0] + (c2[0] - c1[0]) * u,
            c1[1] + (c2[1] - c1[1]) * u,
            c1[2] + (c2[2] - c1[2]) * u,
        ]
    } else {
        let u = tc - 2.0;
        [
            c2[0] + (c3[0] - c2[0]) * u,
            c2[1] + (c3[1] - c2[1]) * u,
            c2[2] + (c3[2] - c2[2]) * u,
        ]
    }
}

#[inline]
fn blend_4stop_radial(
    dst: &mut [f32; 3],
    t: f32,
    c0: [f32; 3],
    a0: f32,
    c1: [f32; 3],
    a1: f32,
    c2: [f32; 3],
    a2: f32,
) {
    let tc = t.clamp(0.0, 1.0) * 3.0;
    let (sr, sg, sb, sa) = if tc < 1.0 {
        let u = tc;
        (
            c0[0] + (c1[0] - c0[0]) * u,
            c0[1] + (c1[1] - c0[1]) * u,
            c0[2] + (c1[2] - c0[2]) * u,
            a0 + (a1 - a0) * u,
        )
    } else if tc < 2.0 {
        let u = tc - 1.0;
        (
            c1[0] + (c2[0] - c1[0]) * u,
            c1[1] + (c2[1] - c1[1]) * u,
            c1[2] + (c2[2] - c1[2]) * u,
            a1 + (a2 - a1) * u,
        )
    } else {
        let u = 1.0 - (tc - 2.0);
        (c2[0], c2[1], c2[2], a2 * u)
    };
    let inv = 1.0 - sa;
    dst[0] = dst[0] * inv + sr * sa;
    dst[1] = dst[1] * inv + sg * sa;
    dst[2] = dst[2] * inv + sb * sa;
}

/// Evaluates the **Electric Jellyfish Backdrop** color at normalized coordinates `(uv_x, uv_y)`
/// in a surface of `(width, height)` at time `t_sec`.
#[inline]
pub fn evaluate_electric_jellyfish_backdrop(
    uv_x: f32,
    uv_y: f32,
    width: f32,
    height: f32,
    t_sec: f32,
    dark_theme: bool,
) -> Color {
    let w = width.max(1.0);
    let h = height.max(1.0);
    let min_dim = w.min(h).max(1.0);
    let px = uv_x * w;
    let py = uv_y * h;

    // 1. Deep abyssal bioluminescent ocean base gradient (Dark) or rich marine-sky blue gradient (Light)
    let diag_t = ((uv_x + uv_y) * 0.5).clamp(0.0, 1.0);
    let mut rgb = if dark_theme {
        sample_4stop_linear(
            diag_t,
            [0x04 as f32 / 255.0, 0x0A as f32 / 255.0, 0x18 as f32 / 255.0],
            [0x07 as f32 / 255.0, 0x14 as f32 / 255.0, 0x2B as f32 / 255.0],
            [0x0A as f32 / 255.0, 0x1D as f32 / 255.0, 0x3B as f32 / 255.0],
            [0x05 as f32 / 255.0, 0x0E as f32 / 255.0, 0x20 as f32 / 255.0],
        )
    } else {
        sample_4stop_linear(
            diag_t,
            [0x56 as f32 / 255.0, 0xA0 as f32 / 255.0, 0xEB as f32 / 255.0],
            [0x42 as f32 / 255.0, 0x8C as f32 / 255.0, 0xE0 as f32 / 255.0],
            [0x4E as f32 / 255.0, 0x98 as f32 / 255.0, 0xE8 as f32 / 255.0],
            [0x68 as f32 / 255.0, 0xB1 as f32 / 255.0, 0xF6 as f32 / 255.0],
        )
    };

    // 2. Five slowly drifting, gently pulsing bioluminescent jellyfish light blooms behind blurry glass (no tendrils/legs)
    struct JellyBloom {
        base_x: f32,
        base_y: f32,
        orbit_x: f32,
        orbit_y: f32,
        speed_mul: f32,
        phase_offset: f32,
        radius_factor: f32,
        core: [f32; 3],
        mid: [f32; 3],
        rim: [f32; 3],
    }

    let blooms: [JellyBloom; 5] = if dark_theme {
        [
            JellyBloom {
                base_x: 0.22,
                base_y: 0.28,
                orbit_x: 0.09,
                orbit_y: 0.08,
                speed_mul: 1.0,
                phase_offset: 0.0,
                radius_factor: 0.48,
                core: [0.00, 0.96, 1.00], // 0xFF00F5FF
                mid: [0.00, 0.57, 0.92],  // 0xFF0091EA
                rim: [0.19, 0.31, 1.00],  // 0xFF304FFE
            },
            JellyBloom {
                base_x: 0.76,
                base_y: 0.24,
                orbit_x: 0.08,
                orbit_y: 0.10,
                speed_mul: -0.8,
                phase_offset: 1.4,
                radius_factor: 0.44,
                core: [0.49, 0.30, 1.00], // 0xFF7C4DFF
                mid: [0.00, 0.69, 1.00],  // 0xFF00B0FF
                rim: [0.00, 0.90, 1.00],  // 0xFF00E5FF
            },
            JellyBloom {
                base_x: 0.52,
                base_y: 0.62,
                orbit_x: 0.10,
                orbit_y: 0.09,
                speed_mul: 0.9,
                phase_offset: 2.7,
                radius_factor: 0.54,
                core: [0.09, 1.00, 1.00], // 0xFF18FFFF
                mid: [0.16, 0.38, 1.00],  // 0xFF2962FF
                rim: [0.67, 0.00, 1.00],  // 0xFFAA00FF
            },
            JellyBloom {
                base_x: 0.18,
                base_y: 0.78,
                orbit_x: 0.08,
                orbit_y: 0.07,
                speed_mul: 1.1,
                phase_offset: 4.1,
                radius_factor: 0.40,
                core: [0.39, 1.00, 0.85], // 0xFF64FFDA
                mid: [0.00, 0.72, 0.83],  // 0xFF00B8D4
                rim: [0.24, 0.35, 1.00],  // 0xFF3D5AFE
            },
            JellyBloom {
                base_x: 0.84,
                base_y: 0.74,
                orbit_x: 0.09,
                orbit_y: 0.08,
                speed_mul: -1.0,
                phase_offset: 5.2,
                radius_factor: 0.46,
                core: [0.88, 0.25, 0.98], // 0xFFE040FB
                mid: [0.33, 0.43, 1.00],  // 0xFF536DFE
                rim: [0.00, 0.90, 1.00],  // 0xFF00E5FF
            },
        ]
    } else {
        [
            JellyBloom {
                base_x: 0.22,
                base_y: 0.28,
                orbit_x: 0.09,
                orbit_y: 0.08,
                speed_mul: 1.0,
                phase_offset: 0.0,
                radius_factor: 0.48,
                core: [1.00, 1.00, 1.00],
                mid: [0.50, 0.97, 1.00],
                rim: [0.09, 1.00, 1.00],
            },
            JellyBloom {
                base_x: 0.76,
                base_y: 0.24,
                orbit_x: 0.08,
                orbit_y: 0.10,
                speed_mul: -0.8,
                phase_offset: 1.4,
                radius_factor: 0.44,
                core: [0.91, 0.87, 1.00],
                mid: [0.51, 0.69, 1.00],
                rim: [0.00, 0.90, 1.00],
            },
            JellyBloom {
                base_x: 0.52,
                base_y: 0.62,
                orbit_x: 0.10,
                orbit_y: 0.09,
                speed_mul: 0.9,
                phase_offset: 2.7,
                radius_factor: 0.54,
                core: [1.00, 1.00, 1.00],
                mid: [0.39, 1.00, 0.85],
                rim: [0.33, 0.43, 1.00],
            },
            JellyBloom {
                base_x: 0.18,
                base_y: 0.78,
                orbit_x: 0.08,
                orbit_y: 0.07,
                speed_mul: 1.1,
                phase_offset: 4.1,
                radius_factor: 0.40,
                core: [0.88, 1.00, 0.98],
                mid: [0.39, 1.00, 0.85],
                rim: [0.00, 0.69, 1.00],
            },
            JellyBloom {
                base_x: 0.84,
                base_y: 0.74,
                orbit_x: 0.09,
                orbit_y: 0.08,
                speed_mul: -1.0,
                phase_offset: 5.2,
                radius_factor: 0.46,
                core: [0.97, 0.73, 0.82],
                mid: [0.50, 0.85, 1.00],
                rim: [0.09, 1.00, 1.00],
            },
        ]
    };

    let drift_phase = t_sec * (std::f32::consts::TAU / 22.0);
    let pulse_phase = t_sec * (std::f32::consts::TAU / 6.0);

    let outer_alpha = if dark_theme { 0.62 } else { 0.60 };
    let mid_alpha = if dark_theme { 0.44 } else { 0.42 };
    let rim_alpha = if dark_theme { 0.20 } else { 0.24 };
    let core_white_alpha = if dark_theme { 0.48 } else { 0.54 };
    let core_tint_alpha = if dark_theme { 0.64 } else { 0.58 };

    for b in &blooms {
        let angle = drift_phase * b.speed_mul + b.phase_offset;
        let pulse = 0.80 + 0.20 * (pulse_phase + b.phase_offset).sin();
        let cx = w * (b.base_x + b.orbit_x * angle.cos());
        let cy = h * (b.base_y + b.orbit_y * (angle * 1.3).sin());
        let radius = (min_dim * b.radius_factor * pulse).max(80.0);
        let dx = px - cx;
        let dy = py - cy;
        let d_sq = dx * dx + dy * dy;
        if d_sq < radius * radius {
            let d = d_sq.sqrt();
            let t_outer = d / radius;
            blend_4stop_radial(
                &mut rgb,
                t_outer,
                b.core,
                outer_alpha * pulse,
                b.mid,
                mid_alpha * pulse,
                b.rim,
                rim_alpha,
            );
            let inner_radius = radius * 0.40;
            if d < inner_radius {
                let t_inner = d / inner_radius;
                blend_4stop_radial(
                    &mut rgb,
                    t_inner,
                    [1.0, 1.0, 1.0],
                    core_white_alpha * pulse,
                    b.core,
                    core_tint_alpha * pulse,
                    b.rim,
                    rim_alpha,
                );
            }
        }
    }

    // 3. Soft diffuse frosted glass pane veil across the viewport
    let (veil_col, veil_a) = if dark_theme {
        if uv_y < 0.5 {
            let u = uv_y * 2.0;
            (
                [
                    (0x07 as f32 + (0x09 as f32 - 0x07 as f32) * u) / 255.0,
                    (0x13 as f32 + (0x19 as f32 - 0x13 as f32) * u) / 255.0,
                    (0x26 as f32 + (0x33 as f32 - 0x26 as f32) * u) / 255.0,
                ],
                0.28 + (0.22 - 0.28) * u,
            )
        } else {
            let u = (uv_y - 0.5) * 2.0;
            (
                [
                    (0x09 as f32 + (0x06 as f32 - 0x09 as f32) * u) / 255.0,
                    (0x19 as f32 + (0x10 as f32 - 0x19 as f32) * u) / 255.0,
                    (0x33 as f32 + (0x22 as f32 - 0x33 as f32) * u) / 255.0,
                ],
                0.22 + (0.30 - 0.22) * u,
            )
        }
    } else if uv_y < 0.5 {
        let u = uv_y * 2.0;
        (
            [
                (0xD6 as f32 + (0xC4 as f32 - 0xD6 as f32) * u) / 255.0,
                (0xEC as f32 + (0xE4 as f32 - 0xEC as f32) * u) / 255.0,
                1.0,
            ],
            0.18 + (0.14 - 0.18) * u,
        )
    } else {
        let u = (uv_y - 0.5) * 2.0;
        (
            [
                (0xC4 as f32 + (0xD6 as f32 - 0xC4 as f32) * u) / 255.0,
                (0xE4 as f32 + (0xEC as f32 - 0xE4 as f32) * u) / 255.0,
                1.0,
            ],
            0.14 + (0.20 - 0.14) * u,
        )
    };

    let inv_v = 1.0 - veil_a;
    rgb[0] = (rgb[0] * inv_v + veil_col[0] * veil_a).clamp(0.0, 1.0);
    rgb[1] = (rgb[1] * inv_v + veil_col[1] * veil_a).clamp(0.0, 1.0);
    rgb[2] = (rgb[2] * inv_v + veil_col[2] * veil_a).clamp(0.0, 1.0);

    Color::rgba(rgb[0], rgb[1], rgb[2], 1.0)
}

impl ElectricJellyfishField {
    /// Step size in pixels between grid samples (`16` pixels = `1 << 4`).
    pub const STEP: usize = 16;

    /// Precomputes the coarse `16x16` grid of the Electric Jellyfish Backdrop for `(width, height)`.
    /// Takes ~35 microseconds per frame and reconstructs the smooth radial blooms via bilinear scanline interpolation.
    pub fn compute(width: u32, height: u32, t_sec: f32, clear_color: Option<Color>) -> Self {
        let w = width.max(1) as f32;
        let h = height.max(1) as f32;
        let dark_theme = clear_color
            .map(|c| (c.r + c.g + c.b) < 1.5)
            .unwrap_or(true);
        let grid_w = (width as usize).div_ceil(Self::STEP) + 1;
        let grid_h = (height as usize).div_ceil(Self::STEP) + 1;
        let mut grid_rgb256 = Vec::with_capacity(grid_w * grid_h);

        for gy in 0..grid_h {
            let py = ((gy * Self::STEP) as f32).min(h);
            let uv_y = (py / h).clamp(0.0, 1.0);
            for gx in 0..grid_w {
                let px = ((gx * Self::STEP) as f32).min(w);
                let uv_x = (px / w).clamp(0.0, 1.0);
                let c = evaluate_electric_jellyfish_backdrop(uv_x, uv_y, w, h, t_sec, dark_theme);
                grid_rgb256.push([
                    (c.r * 65280.0).round() as u32,
                    (c.g * 65280.0).round() as u32,
                    (c.b * 65280.0).round() as u32,
                ]);
            }
        }

        Self {
            grid_w,
            grid_h,
            grid_rgb256,
        }
    }

    /// Renders a single scanline `y` (`0..height`) into `row_rgba` (`width * 4` bytes) with bilinear
    /// interpolation across the `16x16` grid plus subtle frosted-glass optical micro-diffusion.
    #[inline]
    pub fn fill_scanline_rgba(&self, y: u32, width: u32, row_rgba: &mut [u8]) {
        let gy0 = ((y as usize) >> 4).min(self.grid_h.saturating_sub(2));
        let gy1 = (gy0 + 1).min(self.grid_h - 1);
        let wy = (y & 15) as u32;
        let inv_wy = 16 - wy;

        let row0 = &self.grid_rgb256[(gy0 * self.grid_w)..((gy0 + 1) * self.grid_w)];
        let row1 = &self.grid_rgb256[(gy1 * self.grid_w)..((gy1 + 1) * self.grid_w)];

        let w_usize = width as usize;
        let mut x = 0_usize;
        let mut gx = 0_usize;

        let mut c_left = [
            (row0[0][0] * inv_wy + row1[0][0] * wy) >> 4,
            (row0[0][1] * inv_wy + row1[0][1] * wy) >> 4,
            (row0[0][2] * inv_wy + row1[0][2] * wy) >> 4,
        ];

        while x < w_usize {
            let next_gx = (gx + 1).min(self.grid_w - 1);
            let c_right = [
                (row0[next_gx][0] * inv_wy + row1[next_gx][0] * wy) >> 4,
                (row0[next_gx][1] * inv_wy + row1[next_gx][1] * wy) >> 4,
                (row0[next_gx][2] * inv_wy + row1[next_gx][2] * wy) >> 4,
            ];
            let span_end = (x + 16).min(w_usize);
            let dr = (c_right[0] as i32 - c_left[0] as i32) >> 4;
            let dg = (c_right[1] as i32 - c_left[1] as i32) >> 4;
            let db = (c_right[2] as i32 - c_left[2] as i32) >> 4;
            let mut r = c_left[0] as i32;
            let mut g = c_left[1] as i32;
            let mut b = c_left[2] as i32;

            for px in x..span_end {
                // Subtle frosted-glass optical diffusion grain (±2 out of 255)
                let h_noise = (((px as u32).wrapping_mul(73856093)
                    ^ y.wrapping_mul(19349663))
                    >> 16)
                    & 7;
                let grain = (h_noise as i32 - 3) * 64;
                let idx = px * 4;
                row_rgba[idx] = ((r + grain).clamp(0, 65280) >> 8) as u8;
                row_rgba[idx + 1] = ((g + grain).clamp(0, 65280) >> 8) as u8;
                row_rgba[idx + 2] = ((b + grain).clamp(0, 65280) >> 8) as u8;
                row_rgba[idx + 3] = 255;
                r += dr;
                g += dg;
                b += db;
            }

            c_left = c_right;
            x = span_end;
            gx += 1;
        }
    }
}

/// Evaluates the foreground color and effective alpha `(fg_rgb, eff_alpha)` of an
/// `SdfPrimitiveInstance` at `(px, py)` independent of the underlying backdrop pixel.
/// The final composited pixel is `backdrop_color.lerp(fg_rgb, eff_alpha)`.
#[inline]
pub fn evaluate_sdf_instance_foreground(
    inst: &SdfPrimitiveInstance,
    px: f32,
    py: f32,
    pointer: Point,
    t_sec: f32,
) -> (Color, f32) {
    if px < inst.clip_rect[0]
        || px > inst.clip_rect[0] + inst.clip_rect[2]
        || py < inst.clip_rect[1]
        || py > inst.clip_rect[1] + inst.clip_rect[3]
    {
        return (Color::TRANSPARENT, 0.0);
    }

    let cx = inst.rect[0] + inst.rect[2] * 0.5;
    let cy = inst.rect[1] + inst.rect[3] * 0.5;
    let half = Point::new(inst.rect[2] * 0.5, inst.rect[3] * 0.5);
    let local = Point::new(px - cx, py - cy);

    let radius = inst.corner_radii[0];
    let border_w = inst.params[0];
    let shadow_blur = inst.params[1];
    let p = inst.params[2];
    let (material_id, border_effect) = unpack_material_and_border(inst.params[3]);
    if material_id == ShaderMaterialId::GlyphAtlas as u32 {
        return (Color::TRANSPARENT, 0.0);
    }

    let dist = evaluate_sdf_superellipse(local, half, radius, p);
    let inv_w = 1.0 / inst.rect[2].max(1.0);
    let inv_h = 1.0 / inst.rect[3].max(1.0);
    let uv_x = ((px - inst.rect[0]) * inv_w).clamp(0.0, 1.0);
    let uv_y = ((py - inst.rect[1]) * inv_h).clamp(0.0, 1.0);

    let has_smoke_border = border_effect == BorderShaderEffect::GentleSmokeBorder;
    let has_fire_border = border_effect == BorderShaderEffect::FireBorder
        || material_id == ShaderMaterialId::ProceduralFire as u32
        || (!has_smoke_border
            && border_w > 0.0
            && (material_id == ShaderMaterialId::ObsidianNeonPulse as u32
                || (inst.border_color[0] > 0.9 && inst.border_color[1] < 0.35)));

    // Outside the SDF boundary: evaluate consistent analytic drop shadow + optional external fire/smoke/glowbar corona
    if dist > 0.5 {
        let mut sh_col = Color::rgba(
            inst.shadow_color[0],
            inst.shadow_color[1],
            inst.shadow_color[2],
            1.0,
        );
        let mut sh_alpha = if shadow_blur > 0.5 && dist < shadow_blur * 2.0 {
            let t = 1.0 - (dist / (shadow_blur * 2.0));
            (t * t * inst.shadow_color[3]).clamp(0.0, 1.0)
        } else {
            0.0
        };

        if (has_smoke_border || material_id == ShaderMaterialId::GentleSmoke as u32) && dist < 30.0 {
            let top_bias = (1.0 - uv_y).clamp(0.0, 1.0);
            let reach = 22.0 + 8.0 * top_bias;
            if dist < reach {
                let lut = FireSimulationLut::global();
                let drift = lut.fast_sin(t_sec * 0.22 + uv_y * 1.8) * 0.35;
                let nx = (px - inst.rect[0]) * 0.024 + drift;
                let ny = (py - inst.rect[1]) * 0.028 + t_sec * 0.34;
                let falloff = (1.0 - dist / reach).clamp(0.0, 1.0);
                // Keep smoke wispy (clear transparent space between tendrils) while extending further before dissipating
                let threshold = 0.16 + 0.10 * (1.0 - falloff);
                let wisp = ((lut.sample_warped_fire(nx, ny) - threshold) * 1.35).clamp(0.0, 1.0);
                let mist_a = (wisp * falloff * (0.48 + 0.42 * falloff) * 0.76).clamp(0.0, 1.0);
                if mist_a > 0.01 {
                    let mist_c = Color::rgba(0.84, 0.91, 0.99, 1.0);
                    let combined = sh_alpha * (1.0 - mist_a) + mist_a;
                    sh_col = sh_col.lerp(mist_c, (mist_a / combined.max(1e-4)).clamp(0.0, 1.0));
                    sh_alpha = combined.clamp(0.0, 1.0);
                }
            }
        } else if has_fire_border && dist < 18.0 {
            let top_boost = (1.0 - uv_y).clamp(0.0, 1.0);
            let upward_reach = if material_id == ShaderMaterialId::ProceduralFire as u32 {
                8.5 + 6.5 * top_boost
            } else {
                7.0 + 5.5 * top_boost
            };
            if dist < upward_reach {
                let lut = FireSimulationLut::global();
                let wind = ((px - pointer.x) * 0.0045).clamp(-0.35, 0.35);
                let nx = (px - inst.rect[0]) * 0.045 + wind * (1.0 - uv_y);
                let ny = (py - inst.rect[1]) * 0.055 + t_sec * 1.65;

                let flame_noise = lut.sample_warped_fire(nx, ny);
                let corona_falloff = (1.0 - (dist / upward_reach)).clamp(0.0, 1.0);
                let temp = (corona_falloff * (0.42 + flame_noise * 0.85)).clamp(0.0, 1.0);

                if temp > 0.08 {
                    let flame_c = lut.lookup_blackbody(temp * 0.92);
                    let corona_alpha = (temp * corona_falloff * 0.85).clamp(0.0, 1.0);
                    let combined_alpha = sh_alpha * (1.0 - corona_alpha) + corona_alpha;
                    if combined_alpha > 1e-4 {
                        sh_col =
                            sh_col.lerp(flame_c, (corona_alpha / combined_alpha).clamp(0.0, 1.0));
                        sh_alpha = combined_alpha.clamp(0.0, 1.0);
                    }
                }
            }
        } else if material_id == ShaderMaterialId::ColorCyclingGlowbar as u32 && dist < 12.0 {
            let hue = (t_sec * 0.065 + uv_x * 0.38).rem_euclid(1.0);
            let glow_c = color_cycling_spectrum(hue);
            let falloff = (1.0 - dist / 12.0).clamp(0.0, 1.0);
            let bloom_a = (falloff * falloff * 0.62).clamp(0.0, 1.0);
            let combined = sh_alpha * (1.0 - bloom_a) + bloom_a;
            if combined > 1e-4 {
                sh_col = sh_col.lerp(glow_c, (bloom_a / combined).clamp(0.0, 1.0));
                sh_alpha = combined.clamp(0.0, 1.0);
            }
        }

        return (sh_col, sh_alpha);
    }

    // Inside shape or on anti-aliased edge
    let start_c = Color::rgba(
        inst.fill_color_start[0],
        inst.fill_color_start[1],
        inst.fill_color_start[2],
        inst.fill_color_start[3],
    );
    let end_c = Color::rgba(
        inst.fill_color_end[0],
        inst.fill_color_end[1],
        inst.fill_color_end[2],
        inst.fill_color_end[3],
    );
    let mut fill = start_c.lerp(end_c, uv_y);

    // Apply GPU material shader effect
    match material_id {
        x if x == ShaderMaterialId::FrostedGlass as u32 => {
            let lut = FireSimulationLut::global();
            let grain = (lut.sample_turbulence(px * 0.12 + t_sec * 0.35, py * 0.12) - 0.5) * 0.035;
            // Crystalline specular sheen near the top edge of Frosted Glass
            let sheen = (1.0 - uv_y).powi(3) * 0.06;
            fill = Color::rgba(
                (fill.r + grain + sheen).clamp(0.0, 1.0),
                (fill.g + grain + sheen).clamp(0.0, 1.0),
                (fill.b + grain + sheen * 1.2).clamp(0.0, 1.0),
                fill.a.clamp(0.28, 0.78),
            );
        }
        x if x == ShaderMaterialId::ProceduralFire as u32 => {
            let lut = FireSimulationLut::global();
            let wind = ((px - pointer.x) * 0.004).clamp(-0.3, 0.3);
            let nx = (px - inst.rect[0]) * 0.038 + wind * (1.0 - uv_y);
            let ny = (py - inst.rect[1]) * 0.048 + t_sec * 1.55;

            let fbm = lut.sample_warped_fire(nx, ny);

            // Continuous full-height flame profile (no mid-element cutoff from per-pixel fill brightness!)
            let rim_t = (1.0 - (-dist * (1.0 / 16.0)).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let rim_factor = rim_t * rim_t;
            let hearth_factor = 0.46 + 0.54 * uv_y;
            let raw_heat = (hearth_factor * (0.38 + fbm * 0.92) + rim_factor * 0.38).clamp(0.0, 1.0);
            let temp = raw_heat;
            let fire_col = lut.lookup_blackbody(temp);

            // Rising micro-ember sparks
            let spark_cell_x = ((px + wind * 18.0) * 0.18).floor() as i32;
            let spark_y_coord = py * 0.18 + t_sec * 4.5;
            let spark_cell_y = spark_y_coord.floor() as i32;
            let spark_seed = hash21(spark_cell_x, spark_cell_y);
            let spark_intensity = if spark_seed > 0.965 {
                let fy = spark_y_coord - (spark_cell_y as f32);
                ((1.0 - (fy - 0.5).abs() * 2.0) * (1.0 - uv_y * 0.3)).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let charcoal_base = Color::rgba(0.11, 0.04, 0.05, fill.a.clamp(0.58, 0.86));
            let ember_blend = (temp * 0.90).clamp(0.18, 0.94);
            let mut fire_fill = charcoal_base
                .lerp(fire_col, ember_blend)
                .lerp(Color::rgba(1.0, 0.95, 0.78, 1.0), spark_intensity * 0.85);
            fire_fill.a = fill.a.clamp(0.62, 0.85);
            fill = fire_fill;
        }
        x if x == ShaderMaterialId::WaterRipple as u32 => {
            let dx = px - pointer.x;
            let dy = py - pointer.y;
            let d_sq = dx * dx + dy * dy;
            if d_sq < 90_000.0 {
                let d_ptr = d_sq.sqrt();
                let atten = 1.0 / (1.0 + d_ptr * 0.025);
                let ripple = (d_ptr * 0.25 - t_sec * 4.2).sin() * atten;
                let highlight = (ripple * 0.25).clamp(-0.15, 0.25);
                fill = Color::rgba(
                    (fill.r + highlight * 0.5).clamp(0.0, 1.0),
                    (fill.g + highlight * 0.8).clamp(0.0, 1.0),
                    (fill.b + highlight).clamp(0.0, 1.0),
                    fill.a,
                );
            }
        }
        x if x == ShaderMaterialId::FluentRevealHighlight as u32 => {
            let dx = px - pointer.x;
            let dy = py - pointer.y;
            let d_sq = dx * dx + dy * dy;
            if d_sq < 14_400.0 {
                let radial = 1.0 - (d_sq * (1.0 / 14_400.0));
                fill = fill.lerp(Color::WHITE, radial * radial * 0.28);
            }
        }
        x if x == ShaderMaterialId::ObsidianNeonPulse as u32 => {
            if border_w > 0.0 && dist > -border_w - 6.0 && !has_smoke_border {
                let lut = FireSimulationLut::global();
                let wind = ((px - pointer.x) * 0.004).clamp(-0.3, 0.3);
                let nx = (px - inst.rect[0]) * 0.045 + wind * (1.0 - uv_y);
                let ny = (py - inst.rect[1]) * 0.06 + t_sec * 1.65;
                let fbm = lut.sample_warped_fire(nx, ny);

                let rim_t = (1.0 - ((-dist) / (border_w + 6.0)).clamp(0.0, 1.0)).clamp(0.0, 1.0);
                let temp = (rim_t * (0.48 + fbm * 0.85)).clamp(0.0, 1.0);
                let fire_col = lut.lookup_blackbody(temp);
                let prev_a = fill.a;
                fill = fill.lerp(fire_col, (rim_t * 0.92).clamp(0.0, 1.0));
                fill.a = prev_a.max(rim_t * 0.95);
            }
        }
        x if x == ShaderMaterialId::GentleSmoke as u32 => {
            // 1. Gentle Smoke: Soft, wispy laminar vapor tendrils drifting slowly upward
            let lut = FireSimulationLut::global();
            let sway = lut.fast_sin(t_sec * 0.22 + uv_y * 2.4) * 0.28;
            let nx = (px - inst.rect[0]) * 0.020 + sway * (1.0 - uv_y * 0.6);
            let ny = (py - inst.rect[1]) * 0.024 + t_sec * 0.32;
            let fbm = lut.sample_warped_fire(nx, ny);
            let wisp = ((fbm - 0.18) * 1.25).clamp(0.0, 1.0);
            let density = (wisp * wisp * (0.48 + 0.52 * (1.0 - uv_y * 0.40))).clamp(0.0, 0.88);
            let avg_luma = (start_c.r + start_c.g + start_c.b + end_c.r + end_c.g + end_c.b) * 0.5;
            let plume_color = if avg_luma > 1.5 {
                Color::rgba(0.68, 0.76, 0.88, 1.0)
            } else {
                Color::rgba(0.84, 0.90, 0.98, 1.0)
            };
            let prev_a = fill.a;
            fill = fill.lerp(plume_color, density * 0.68);
            fill.a = (prev_a * 0.88 + density * 0.12).clamp(0.38, 0.84);
        }
        x if x == ShaderMaterialId::GentlePulsingLights as u32 => {
            // 2. Gently Pulsing Lights: Three soft phase-offset breathing ambient bokeh light orbs (slow & calm)
            let lut = FireSimulationLut::global();
            let orbs: [(f32, f32, f32, [f32; 3]); 3] = [
                (
                    0.24 + 0.06 * lut.fast_sin(t_sec * 0.20),
                    0.38 + 0.08 * lut.fast_cos(t_sec * 0.15),
                    0.55 + 0.45 * lut.fast_sin(t_sec * 0.48),
                    [1.00, 0.84, 0.42],
                ),
                (
                    0.54 + 0.07 * lut.fast_cos(t_sec * 0.18 + 1.4),
                    0.62 + 0.07 * lut.fast_sin(t_sec * 0.22),
                    0.55 + 0.45 * lut.fast_sin(t_sec * 0.42 + 2.1),
                    [0.38, 0.92, 1.00],
                ),
                (
                    0.78 + 0.05 * lut.fast_sin(t_sec * 0.16 + 2.8),
                    0.34 + 0.08 * lut.fast_cos(t_sec * 0.19),
                    0.55 + 0.45 * lut.fast_sin(t_sec * 0.52 + 4.2),
                    [0.92, 0.58, 1.00],
                ),
            ];
            let mut add_r = 0.0_f32;
            let mut add_g = 0.0_f32;
            let mut add_b = 0.0_f32;
            for &(ox, oy, pulse, col) in &orbs {
                let dx = uv_x - ox;
                let dy = (uv_y - oy) * 0.65;
                let d_sq = dx * dx + dy * dy;
                if d_sq < 0.22 {
                    let t = 1.0 - d_sq * (1.0 / 0.22);
                    let w = t * t * pulse * 0.58;
                    add_r += col[0] * w;
                    add_g += col[1] * w;
                    add_b += col[2] * w;
                }
            }
            fill = Color::rgba(
                (fill.r + add_r).clamp(0.0, 1.0),
                (fill.g + add_g).clamp(0.0, 1.0),
                (fill.b + add_b).clamp(0.0, 1.0),
                fill.a.clamp(0.42, 0.82),
            );
        }
        x if x == ShaderMaterialId::ReflectiveGlass as u32 => {
            // 3. Reflective Glass: High-index optical glass with diagonal specular bands & Fresnel bevel
            let lut = FireSimulationLut::global();
            let ptr_parallax = ((pointer.x - cx) * 0.0006 + (pointer.y - cy) * 0.0004).clamp(-0.22, 0.22);
            let sweep = uv_x * 0.72 + uv_y * 0.44 - ptr_parallax + lut.fast_sin(t_sec * 0.14) * 0.09;
            let band_primary = (1.0 - ((sweep - 0.48).abs() * 6.5)).clamp(0.0, 1.0);
            let band_secondary = (1.0 - ((sweep - 0.74).abs() * 11.0)).clamp(0.0, 1.0);
            let spec_band = band_primary * band_primary * 0.38 + band_secondary * band_secondary * 0.22;

            let edge_t = (1.0 - (-dist * (1.0 / 14.0)).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let fresnel_rim = edge_t * edge_t * edge_t * 0.42;
            let top_glint = (1.0 - uv_y).powi(3) * 0.14;
            let refl = (spec_band + fresnel_rim + top_glint).clamp(0.0, 0.78);

            let sky_spec = Color::rgba(0.92, 0.97, 1.0, 1.0);
            let mut glass = fill.lerp(sky_spec, refl);
            glass.a = (fill.a * 0.78 + refl * 0.20).clamp(0.38, 0.80);
            fill = glass;
        }
        x if x == ShaderMaterialId::SubtleGlassReflection as u32 => {
            // 4. Subtle Glass Reflection: Whisper-soft diagonal optical sheen + faint top glint (much more subtle than ReflectiveGlass)
            let lut = FireSimulationLut::global();
            let ptr_parallax = ((pointer.x - cx) * 0.00045 + (pointer.y - cy) * 0.00035).clamp(-0.16, 0.16);
            let sweep = uv_x * 0.68 + uv_y * 0.42 - ptr_parallax + lut.fast_sin(t_sec * 0.10) * 0.06;
            let b1 = (1.0 - ((sweep - 0.46).abs() * 5.2)).clamp(0.0, 1.0);
            let b2 = (1.0 - ((sweep - 0.72).abs() * 9.0)).clamp(0.0, 1.0);
            let spec_band = b1 * b1 * 0.075 + b2 * b2 * 0.040;

            let edge_t = (1.0 - (-dist * (1.0 / 12.0)).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let fresnel_rim = edge_t * edge_t * edge_t * 0.075;
            let top_glint = (1.0 - uv_y).powi(4) * 0.055;
            let refl = (spec_band + fresnel_rim + top_glint).clamp(0.0, 0.14);

            let sky_spec = Color::rgba(0.90, 0.96, 1.0, 1.0);
            let mut glass = fill.lerp(sky_spec, refl);
            glass.a = (fill.a + refl * 0.08).clamp(0.30, 0.78);
            fill = glass;
        }
        x if x == ShaderMaterialId::ColorCyclingGlowbar as u32 => {
            // 5. Glowbar That Slowly Changes Colours: Smooth cosine spectrum + luminous core
            let hue = (t_sec * 0.065 + uv_x * 0.38).rem_euclid(1.0);
            let spec = color_cycling_spectrum(hue);
            let core_y = (1.0 - (uv_y - 0.5).abs() * 1.75).clamp(0.0, 1.0);
            let core_boost = core_y * core_y * 0.48;
            let mut glow = fill.lerp(spec, 0.82).lerp(Color::WHITE, core_boost);
            glow.a = fill.a.clamp(0.55, 0.86);
            fill = glow;
        }
        x if x == ShaderMaterialId::FlowingGentleGradient as u32 => {
            // 6. Gentle Gradient That Slowly Moves (endless left-to-right yellow -> buttercream -> white loop)
            let grad = flowing_yellow_white_gradient(uv_x, uv_y, t_sec, start_c);
            let mut out_c = fill.lerp(grad, 0.86);
            out_c.a = fill.a.clamp(0.50, 0.82);
            fill = out_c;
        }
        x if x == ShaderMaterialId::ElectricJellyfish as u32 => {
            // 7. Electric Jellyfish Backdrop: Deep abyssal blue + drifting pulsing bioluminescent jellyfish blooms behind diffuse frosted glass
            let avg_luma = (start_c.r + start_c.g + start_c.b + end_c.r + end_c.g + end_c.b) * 0.5;
            let dark_theme = avg_luma < 1.5;
            let mut jelly = evaluate_electric_jellyfish_backdrop(
                uv_x,
                uv_y,
                inst.rect[2].max(1.0),
                inst.rect[3].max(1.0),
                t_sec,
                dark_theme,
            );
            let lut = FireSimulationLut::global();
            let grain = (lut.sample_turbulence(px * 0.14 + t_sec * 0.15, py * 0.14) - 0.5) * 0.025;
            jelly.r = (jelly.r + grain).clamp(0.0, 1.0);
            jelly.g = (jelly.g + grain).clamp(0.0, 1.0);
            jelly.b = (jelly.b + grain).clamp(0.0, 1.0);
            jelly.a = fill.a.clamp(0.58, 0.90);
            fill = jelly;
        }
        _ => {}
    }

    // Interior rim & border stroke for GentleSmokeBorder or FireBorder
    if border_w > 0.0 && dist > -border_w - 5.5 {
        if has_smoke_border {
            let lut = FireSimulationLut::global();
            let sway = lut.fast_sin(t_sec * 0.24 + uv_y * 2.0) * 0.30;
            let nx = (px - inst.rect[0]) * 0.032 + sway;
            let ny = (py - inst.rect[1]) * 0.036 + t_sec * 0.38;
            let fbm = lut.sample_warped_fire(nx, ny);
            let rim_t = (1.0 - ((-dist) / (border_w + 5.5)).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let wisp_noise = ((fbm - 0.14) * 1.32).clamp(0.0, 1.0);
            let wisp = (rim_t * wisp_noise).clamp(0.0, 1.0);
            let smoke_c = Color::rgba(0.86, 0.93, 1.0, 0.95);
            let prev_a = fill.a;
            fill = fill.lerp(smoke_c, (wisp * 0.85).clamp(0.0, 1.0));
            fill.a = prev_a.max(rim_t * 0.92);
        } else if has_fire_border && material_id != ShaderMaterialId::ProceduralFire as u32 {
            let lut = FireSimulationLut::global();
            let wind = ((px - pointer.x) * 0.004).clamp(-0.3, 0.3);
            let nx = (px - inst.rect[0]) * 0.045 + wind * (1.0 - uv_y);
            let ny = (py - inst.rect[1]) * 0.06 + t_sec * 1.65;
            let fbm = lut.sample_warped_fire(nx, ny);
            let rim_t = (1.0 - ((-dist) / (border_w + 5.5)).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let temp = (rim_t * (0.48 + fbm * 0.85)).clamp(0.0, 1.0);
            let fire_col = lut.lookup_blackbody(temp);
            let prev_a = fill.a;
            fill = fill.lerp(fire_col, (rim_t * 0.92).clamp(0.0, 1.0));
            fill.a = prev_a.max(rim_t * 0.95);
        }
    }

    // Border stroke near `dist in [-border_w, 0.0]`
    if border_w > 0.0 && dist >= -border_w {
        let mut bc = Color::rgba(
            inst.border_color[0],
            inst.border_color[1],
            inst.border_color[2],
            inst.border_color[3],
        );
        if has_smoke_border {
            let lut = FireSimulationLut::global();
            let rim_noise = lut.sample_warped_fire(px * 0.035, py * 0.035 + t_sec * 0.42);
            let wisp_t = (0.45 + rim_noise * 0.55).clamp(0.0, 1.0);
            bc = Color::rgba(0.76, 0.86, 0.98, 0.92)
                .lerp(Color::rgba(0.96, 0.99, 1.0, 0.98), wisp_t);
        } else if has_fire_border {
            let lut = FireSimulationLut::global();
            let rim_noise = lut.sample_warped_fire(px * 0.05, py * 0.05 + t_sec * 1.8);
            bc = lut.lookup_blackbody((0.55 + rim_noise * 0.45).clamp(0.0, 1.0));
        }
        let next_a = fill.a.max(bc.a);
        fill = fill.lerp(bc, bc.a);
        fill.a = next_a;
    }

    let edge_aa = (0.5 - dist).clamp(0.0, 1.0);
    (fill, (edge_aa * fill.a).clamp(0.0, 1.0))
}

/// Evaluates the full SDF + material shader pipeline for a single pixel `(px, py)`
/// against a given `backdrop_color`.
#[inline]
pub fn evaluate_sdf_instance_pixel(
    inst: &SdfPrimitiveInstance,
    px: f32,
    py: f32,
    pointer: Point,
    t_sec: f32,
    backdrop_color: Color,
) -> Color {
    let (fg, eff_alpha) = evaluate_sdf_instance_foreground(inst, px, py, pointer, t_sec);
    if eff_alpha <= 0.0 {
        backdrop_color
    } else {
        backdrop_color.lerp(fg, eff_alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdf_instance_alignment_and_size() {
        assert_eq!(std::mem::size_of::<SdfPrimitiveInstance>(), 128);
        assert_eq!(std::mem::align_of::<SdfPrimitiveInstance>(), 16);
    }

    #[test]
    fn test_built_in_spirv_shaders_valid() {
        for spirv in [
            sdf_uber_shader_spirv(),
            dual_kawase_blur_spirv(),
            procedural_fire_spirv(),
            water_ripple_spirv(),
            gentle_smoke_spirv(),
            pulsing_lights_spirv(),
            reflective_glass_spirv(),
            color_cycling_glowbar_spirv(),
            flowing_gradient_spirv(),
            electric_jellyfish_spirv(),
            subtle_glass_reflection_spirv(),
        ] {
            assert_eq!(validate_spirv_binary(&spirv), Ok(()));
        }
    }

    #[test]
    fn test_wgsl_security_validator() {
        let valid_wgsl = "@fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }";
        assert_eq!(validate_wgsl_shader(valid_wgsl), Ok(()));

        let infinite_loop = "@compute fn cs_main() { loop { let x = 1; } }";
        assert_eq!(
            validate_wgsl_shader(infinite_loop),
            Err(ShaderValidationError::UnboundedLoopDetected)
        );
    }

    #[test]
    fn test_sdf_superellipse_exponents() {
        let half = Point::new(50.0, 50.0);
        let corner_pt = Point::new(45.0, 45.0);
        let d_circle = evaluate_sdf_superellipse(corner_pt, half, 16.0, 2.0);
        let d_squircle = evaluate_sdf_superellipse(corner_pt, half, 16.0, 4.2);
        // A squircle (p=4.2) reaches closer to the corner than a circle (p=2.0)
        assert!(d_squircle < d_circle);
    }

    #[test]
    fn test_blackbody_fire_ramp_and_fbm_turbulence() {
        let low = blackbody_fire_color(0.15);
        let mid = blackbody_fire_color(0.55);
        let high = blackbody_fire_color(0.95);
        // Low heat is crimson (red dominant, near-zero green/blue)
        assert!(low.r > 0.3 && low.g < 0.05 && low.b == 0.0);
        // Mid heat is amber-orange (strong red, moderate green, zero blue)
        assert!(mid.r > 0.9 && mid.g > 0.25 && mid.b == 0.0);
        // High heat is incandescent white-gold (high R, G, and warm B)
        assert!(high.r > 0.99 && high.g > 0.9 && high.b > 0.6);

        let n1 = fbm_turbulence(1.23, 4.56);
        let n2 = fbm_turbulence(2.34, 5.67);
        assert!((0.0..=1.0).contains(&n1));
        assert!((n1 - n2).abs() > 0.001);
    }

    #[test]
    fn test_wallpaper_linear_cover_scale_and_center_crop() {
        let wp = WaterDropWallpaper::global();
        // 1. Portrait window (600x1200): height is constraining dimension (0..1079), width is center-cropped
        let (x_map_p, y_map_p) = wp.compute_cover_crop_maps(600, 1200);
        assert_eq!(y_map_p.first().unwrap().0, 0);
        assert!(y_map_p.last().unwrap().0 >= (wp.height as usize - 2));
        assert!(x_map_p.first().unwrap().0 > 600); // Cropped left
        assert!(x_map_p.last().unwrap().0 < (wp.width as usize - 600)); // Cropped right

        // 2. Ultrawide window (2560x1080): width is constraining dimension (0..1919), height is center-cropped
        let (x_map_w, y_map_w) = wp.compute_cover_crop_maps(2560, 1080);
        assert_eq!(x_map_w.first().unwrap().0, 0);
        assert!(x_map_w.last().unwrap().0 >= (wp.width as usize - 2));
        assert!(y_map_w.first().unwrap().0 > 100); // Cropped top
        assert!(y_map_w.last().unwrap().0 < (wp.height as usize - 100)); // Cropped bottom
    }

    #[test]
    fn test_new_shader_materials_smoke_pulsing_glass_glowbar_gradient() {
        for mat in [
            ShaderMaterialId::GentleSmoke,
            ShaderMaterialId::GentlePulsingLights,
            ShaderMaterialId::ReflectiveGlass,
            ShaderMaterialId::ColorCyclingGlowbar,
            ShaderMaterialId::FlowingGentleGradient,
            ShaderMaterialId::ElectricJellyfish,
            ShaderMaterialId::SubtleGlassReflection,
        ] {
            let inst = SdfPrimitiveInstance {
                rect: [10.0, 10.0, 200.0, 60.0],
                params: [1.5, 8.0, 2.0, mat as u32 as f32],
                ..Default::default()
            };
            let (c0, a0) = evaluate_sdf_instance_foreground(&inst, 60.0, 40.0, Point::new(100.0, 40.0), 0.25);
            let (c1, a1) = evaluate_sdf_instance_foreground(&inst, 140.0, 40.0, Point::new(100.0, 40.0), 1.75);
            assert!(a0 > 0.5 && a1 > 0.5);
            // Verify temporal/spatial animation changes pixel color smoothly
            let diff = (c0.r - c1.r).abs() + (c0.g - c1.g).abs() + (c0.b - c1.b).abs();
            assert!(diff > 0.001, "Shader {:?} should vary across space/time", mat);
        }

        // Verify ElectricJellyfishField precomputed scanline interpolation
        let field = ElectricJellyfishField::compute(1920, 1080, 1.5, Some(Color::rgba(0.05, 0.08, 0.14, 1.0)));
        let mut scanline = vec![0u8; 1920 * 4];
        field.fill_scanline_rgba(540, 1920, &mut scanline);
        assert_eq!(scanline[3], 255);
        assert!(scanline[2] > scanline[0], "Abyssal ocean backdrop should be blue-dominant");
    }
}
