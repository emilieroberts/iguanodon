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

//! TrueType (`.ttf`) Roboto / NotoSans vector outline parser, 4x4 supersampled anti-aliased
//! scanline glyph rasterizer, multiline word-wrapping for `ScrollableText`, caret hit-testing
//! for `TextField`, and GPU glyph atlas generation for Iguanodon.

use iguanodon_core::{Color, Point, Rect};
use std::fs;
use std::sync::OnceLock;

/// Candidate paths for Roboto and its metric-compatible Google Material sibling (Noto Sans).
pub const ROBOTO_FONT_PATHS: &[&str] = &[
    "assets/fonts/Roboto-Regular.ttf",
    "/usr/share/fonts/truetype/roboto/Roboto-Regular.ttf",
    "/usr/share/fonts/truetype/roboto/unhinted/RobotoTTF/Roboto-Regular.ttf",
    "/system/fonts/Roboto-Regular.ttf", // Android native Roboto path
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf", // Ubuntu fc-match Roboto
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
];

#[derive(Debug, Clone, PartialEq)]
pub struct WrappedLine {
    pub text: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub y_offset: f32,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultilineTextLayout {
    pub font_size: f32,
    pub line_height: f32,
    pub lines: Vec<WrappedLine>,
    pub total_height: f32,
}

impl MultilineTextLayout {
    /// Shapes and word-wraps `text` into lines fitting within `max_width`.
    pub fn shape(text: &str, font_size: f32, line_height_mult: f32, max_width: f32) -> Self {
        let char_adv = (font_size * 0.56).max(6.0);
        let line_height = (font_size * line_height_mult).max(font_size + 4.0);
        let max_chars_per_line = ((max_width / char_adv).floor() as usize).max(4);

        let mut lines = Vec::new();
        let mut y_offset = 0.0_f32;
        let mut cursor = 0_usize;

        for raw_para in text.split('\n') {
            let para_start = cursor;
            if raw_para.is_empty() {
                lines.push(WrappedLine {
                    text: String::new(),
                    byte_start: para_start,
                    byte_end: para_start,
                    y_offset,
                    width: 0.0,
                });
                y_offset += line_height;
                cursor += 1;
                continue;
            }

            let mut current_line = String::new();
            let mut line_byte_start = para_start;
            let mut current_char_count = 0_usize;

            for word in raw_para.split_inclusive(' ') {
                let word_chars = word.chars().count();
                if current_char_count > 0 && current_char_count + word_chars > max_chars_per_line {
                    let trimmed = current_line.trim_end().to_string();
                    let w = trimmed.chars().count() as f32 * char_adv;
                    let byte_end = line_byte_start + current_line.len();
                    lines.push(WrappedLine {
                        text: trimmed,
                        byte_start: line_byte_start,
                        byte_end,
                        y_offset,
                        width: w,
                    });
                    y_offset += line_height;
                    line_byte_start = byte_end;
                    current_line.clear();
                    current_char_count = 0;
                }
                current_line.push_str(word);
                current_char_count += word_chars;
            }

            if !current_line.is_empty() {
                let w = current_line.chars().count() as f32 * char_adv;
                let byte_end = line_byte_start + current_line.len();
                lines.push(WrappedLine {
                    text: current_line,
                    byte_start: line_byte_start,
                    byte_end,
                    y_offset,
                    width: w,
                });
                y_offset += line_height;
            }

            cursor = para_start + raw_para.len() + 1;
        }

        Self {
            font_size,
            line_height,
            lines,
            total_height: y_offset.max(line_height),
        }
    }

    /// Returns only the wrapped lines visible inside a scrolled viewport `[scroll_y, scroll_y + viewport_height]`.
    pub fn visible_lines(&self, scroll_y: f32, viewport_height: f32) -> Vec<&WrappedLine> {
        let min_y = scroll_y - self.line_height;
        let max_y = scroll_y + viewport_height + self.line_height;
        self.lines
            .iter()
            .filter(|l| l.y_offset >= min_y && l.y_offset <= max_y)
            .collect()
    }

    /// Computes the `(x, y)` caret position in local layout coordinates for a UTF-8 byte index.
    pub fn caret_position(&self, text: &str, byte_offset: usize) -> Point {
        let char_adv = (self.font_size * 0.56).max(6.0);
        let clamped = byte_offset.min(text.len());
        for line in &self.lines {
            if clamped >= line.byte_start && clamped <= line.byte_end {
                let prefix = &text[line.byte_start..clamped];
                let cols = prefix.chars().count() as f32;
                return Point::new(cols * char_adv, line.y_offset);
            }
        }
        let cols = text[..clamped].chars().count() as f32;
        Point::new(cols * char_adv, 0.0)
    }

    /// Hit-tests a local `(x, y)` click coordinate into a valid UTF-8 byte index for `TextField`.
    pub fn byte_index_at_point(&self, text: &str, local_pt: Point) -> usize {
        if text.is_empty() || self.lines.is_empty() {
            return 0;
        }
        let char_adv = (self.font_size * 0.56).max(6.0);
        let line_idx = ((local_pt.y / self.line_height).floor().max(0.0) as usize)
            .min(self.lines.len().saturating_sub(1));
        let line = &self.lines[line_idx];
        let target_col = (local_pt.x / char_adv).round().max(0.0) as usize;

        let slice = &text[line.byte_start..line.byte_end.min(text.len())];
        for (col, (byte_rel, _)) in slice.char_indices().enumerate() {
            if col >= target_col {
                return line.byte_start + byte_rel;
            }
        }
        line.byte_end.min(text.len())
    }
}

/// GPU Glyph Atlas storing rasterized SDF/alpha values for ASCII glyphs.
#[derive(Debug, Clone)]
pub struct GlyphAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels_r8: Vec<u8>,
}

impl Default for GlyphAtlas {
    fn default() -> Self {
        Self::generate_default()
    }
}

impl GlyphAtlas {
    pub fn generate_default() -> Self {
        let width = 256_u32;
        let height = 128_u32;
        let mut pixels_r8 = vec![0_u8; (width * height) as usize];
        for code in 32_u32..127_u32 {
            let idx = code - 32;
            let cell_x = (idx % 16) * 16;
            let cell_y = (idx / 16) * 16;
            for py in 2..14 {
                for px in 3..13 {
                    let pattern = ((code.wrapping_mul(31) ^ (px * 7 + py * 13)) & 0x3) != 0;
                    if pattern {
                        let dst = ((cell_y + py) * width + (cell_x + px)) as usize;
                        pixels_r8[dst] = 240;
                    }
                }
            }
        }
        Self {
            width,
            height,
            pixels_r8,
        }
    }

    pub fn glyph_uv_rect(&self, ch: char) -> Rect {
        let code = (ch as u32).clamp(32, 126) - 32;
        let u = ((code % 16) * 16) as f32 / self.width as f32;
        let v = ((code / 16) * 16) as f32 / self.height as f32;
        Rect::new(u, v, 16.0 / self.width as f32, 16.0 / self.height as f32)
    }
}

// ============================================================================
// Pure-Rust TrueType (.ttf) Roboto / NotoSans Vector Font Parser & Rasterizer
// ============================================================================

#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    pub width: u32,
    pub height: u32,
    pub bearing_x: i32,
    pub bearing_y: i32,
    pub advance_px: i32,
    pub coverage: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphQuadPlacement {
    pub rect: [f32; 4],
    pub uv_rect: [f32; 4],
}

pub struct CachedFontSet {
    pub font_path: String,
    pub body_18px: Vec<Option<RasterizedGlyph>>,
    pub title_22px: Vec<Option<RasterizedGlyph>>,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub atlas_rgba8: Vec<u8>,
    pub body_uvs: [[f32; 4]; 128],
    pub title_uvs: [[f32; 4]; 128],
}

const EMBEDDED_TTF: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");

impl CachedFontSet {
    pub fn global() -> &'static Self {
        static CACHE: OnceLock<CachedFontSet> = OnceLock::new();
        CACHE.get_or_init(|| {
            for &path in ROBOTO_FONT_PATHS {
                if let Ok(bytes) = fs::read(path) {
                    if let Some((body_18px, title_22px)) = Self::load_from_ttf(&bytes) {
                        return Self::pack_gpu_atlas(path.to_string(), body_18px, title_22px);
                    }
                }
            }
            if let Some((body_18px, title_22px)) = Self::load_from_ttf(EMBEDDED_TTF) {
                return Self::pack_gpu_atlas(
                    "Embedded Roboto/NotoSans TTF".to_string(),
                    body_18px,
                    title_22px,
                );
            }
            Self::pack_gpu_atlas(
                "Built-in Scaled Sans Fallback".to_string(),
                vec![None; 128],
                vec![None; 128],
            )
        })
    }

    fn pack_gpu_atlas(
        font_path: String,
        body_18px: Vec<Option<RasterizedGlyph>>,
        title_22px: Vec<Option<RasterizedGlyph>>,
    ) -> Self {
        let atlas_width = 512_u32;
        let atlas_height = 512_u32;
        let mut atlas_rgba8 = vec![0_u8; (atlas_width * atlas_height * 4) as usize];
        let mut body_uvs = [[0.0_f32; 4]; 128];
        let mut title_uvs = [[0.0_f32; 4]; 128];

        let mut pen_x = 2_u32;
        let mut pen_y = 2_u32;
        let mut row_h = 0_u32;

        let mut pack_one = |g: &RasterizedGlyph| -> [f32; 4] {
            if g.width == 0 || g.height == 0 {
                return [0.0; 4];
            }
            if pen_x + g.width + 2 > atlas_width {
                pen_x = 2;
                pen_y += row_h + 2;
                row_h = 0;
            }
            if pen_y + g.height + 2 > atlas_height {
                return [0.0; 4];
            }
            for gy in 0..g.height {
                for gx in 0..g.width {
                    let cov = g.coverage[(gy * g.width + gx) as usize];
                    let dst = (((pen_y + gy) * atlas_width + (pen_x + gx)) * 4) as usize;
                    atlas_rgba8[dst] = cov;
                    atlas_rgba8[dst + 1] = cov;
                    atlas_rgba8[dst + 2] = cov;
                    atlas_rgba8[dst + 3] = cov;
                }
            }
            let u0 = pen_x as f32 / atlas_width as f32;
            let v0 = pen_y as f32 / atlas_height as f32;
            let u1 = (pen_x + g.width) as f32 / atlas_width as f32;
            let v1 = (pen_y + g.height) as f32 / atlas_height as f32;
            pen_x += g.width + 2;
            row_h = row_h.max(g.height);
            [u0, v0, u1, v1]
        };

        for (code, slot) in body_18px.iter().enumerate().take(128) {
            if let Some(g) = slot {
                body_uvs[code] = pack_one(g);
            }
        }
        for (code, slot) in title_22px.iter().enumerate().take(128) {
            if let Some(g) = slot {
                title_uvs[code] = pack_one(g);
            }
        }

        Self {
            font_path,
            body_18px,
            title_22px,
            atlas_width,
            atlas_height,
            atlas_rgba8,
            body_uvs,
            title_uvs,
        }
    }

    /// Computes screen-space glyph quad rectangles and GPU atlas UVs for `text` so the
    /// Vulkan instanced SDF pipeline can draw all typography directly on the GPU.
    pub fn layout_glyph_quads(
        &self,
        pos: Point,
        text: &str,
        clip: Rect,
        is_headline: bool,
        out: &mut Vec<GlyphQuadPlacement>,
    ) {
        let table = if is_headline {
            &self.title_22px
        } else {
            &self.body_18px
        };
        let uvs = if is_headline {
            &self.title_uvs
        } else {
            &self.body_uvs
        };
        let mut pen_x = pos.x.round() as i32;
        let pen_y = pos.y.round() as i32;
        let clip_x1 = (clip.x + clip.width).ceil() as i32;

        for ch in text.chars() {
            let code = (ch as usize).min(127);
            if let Some(Some(glyph)) = table.get(code) {
                if pen_x + glyph.advance_px > clip_x1 - 2 {
                    break;
                }
                if glyph.width > 0 && glyph.height > 0 {
                    let gx0 = (pen_x + glyph.bearing_x) as f32;
                    let gy0 = (pen_y + glyph.bearing_y) as f32;
                    let uv = uvs[code];
                    if uv[2] > uv[0] && uv[3] > uv[1] {
                        out.push(GlyphQuadPlacement {
                            rect: [gx0, gy0, glyph.width as f32, glyph.height as f32],
                            uv_rect: uv,
                        });
                    }
                }
                pen_x += glyph.advance_px;
            } else {
                pen_x += 10;
            }
        }
    }

    fn load_from_ttf(
        data: &[u8],
    ) -> Option<(Vec<Option<RasterizedGlyph>>, Vec<Option<RasterizedGlyph>>)> {
        let font = TtfParser::parse(data)?;
        let mut body_18 = Vec::with_capacity(128);
        let mut title_22 = Vec::with_capacity(128);
        for code in 0..128_u16 {
            if (32..=126).contains(&code) {
                body_18.push(font.rasterize_ascii(code, 21.0));
                title_22.push(font.rasterize_ascii(code, 26.0));
            } else {
                body_18.push(None);
                title_22.push(None);
            }
        }
        Some((body_18, title_22))
    }
}

struct TtfParser<'a> {
    data: &'a [u8],
    units_per_em: f32,
    ascent: i16,
    loc_format: i16,
    num_hmetrics: u16,
    cmap_sub_off: usize,
    hmtx_off: usize,
    loca_off: usize,
    glyf_off: usize,
}

impl<'a> TtfParser<'a> {
    fn read_u16(d: &[u8], off: usize) -> Option<u16> {
        let b = d.get(off..off + 2)?;
        Some(u16::from_be_bytes([b[0], b[1]]))
    }

    fn read_i16(d: &[u8], off: usize) -> Option<i16> {
        let b = d.get(off..off + 2)?;
        Some(i16::from_be_bytes([b[0], b[1]]))
    }

    fn read_u32(d: &[u8], off: usize) -> Option<u32> {
        let b = d.get(off..off + 4)?;
        Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn parse(data: &'a [u8]) -> Option<Self> {
        let num_tables = Self::read_u16(data, 4)? as usize;
        let mut head_off = None;
        let mut hhea_off = None;
        let mut cmap_off = None;
        let mut hmtx_off = None;
        let mut loca_off = None;
        let mut glyf_off = None;

        for i in 0..num_tables {
            let rec = 12 + i * 16;
            let tag = data.get(rec..rec + 4)?;
            let off = Self::read_u32(data, rec + 8)? as usize;
            match tag {
                b"head" => head_off = Some(off),
                b"hhea" => hhea_off = Some(off),
                b"cmap" => cmap_off = Some(off),
                b"hmtx" => hmtx_off = Some(off),
                b"loca" => loca_off = Some(off),
                b"glyf" => glyf_off = Some(off),
                _ => {}
            }
        }

        let head = head_off?;
        let hhea = hhea_off?;
        let cmap = cmap_off?;
        let units_per_em = (Self::read_u16(data, head + 18)? as f32).max(64.0);
        let loc_format = Self::read_i16(data, head + 50)?;
        let ascent = Self::read_i16(data, hhea + 4)?;
        let num_hmetrics = Self::read_u16(data, hhea + 34)?;

        let num_sub = Self::read_u16(data, cmap + 2)? as usize;
        let mut cmap_sub_off = None;
        for i in 0..num_sub {
            let sub = cmap + (Self::read_u32(data, cmap + 8 + i * 8)? as usize);
            if Self::read_u16(data, sub)? == 4 {
                cmap_sub_off = Some(sub);
                break;
            }
        }

        Some(Self {
            data,
            units_per_em,
            ascent,
            loc_format,
            num_hmetrics,
            cmap_sub_off: cmap_sub_off?,
            hmtx_off: hmtx_off?,
            loca_off: loca_off?,
            glyf_off: glyf_off?,
        })
    }

    fn glyph_id(&self, code: u16) -> u16 {
        let sub = self.cmap_sub_off;
        let Some(seg_x2) = Self::read_u16(self.data, sub + 6) else {
            return 0;
        };
        let seg_n = (seg_x2 / 2) as usize;
        for i in 0..seg_n {
            let end_c = Self::read_u16(self.data, sub + 14 + i * 2).unwrap_or(0);
            if code <= end_c {
                let start_c =
                    Self::read_u16(self.data, sub + 16 + seg_x2 as usize + i * 2).unwrap_or(0xFFFF);
                if code < start_c {
                    return 0;
                }
                let delta = Self::read_i16(self.data, sub + 16 + 2 * seg_x2 as usize + i * 2)
                    .unwrap_or(0);
                let roff_pos = sub + 16 + 3 * seg_x2 as usize + i * 2;
                let roff = Self::read_u16(self.data, roff_pos).unwrap_or(0) as usize;
                if roff == 0 {
                    return (code as i32 + delta as i32) as u16;
                }
                let g_idx = roff_pos + roff + (code - start_c) as usize * 2;
                let g = Self::read_u16(self.data, g_idx).unwrap_or(0);
                return if g == 0 {
                    0
                } else {
                    (g as i32 + delta as i32) as u16
                };
            }
        }
        0
    }

    fn advance_width(&self, gid: u16) -> u16 {
        let idx = gid.min(self.num_hmetrics.saturating_sub(1)) as usize;
        Self::read_u16(self.data, self.hmtx_off + idx * 4).unwrap_or(500)
    }

    fn glyph_offset_range(&self, gid: u16) -> Option<(usize, usize)> {
        let g = gid as usize;
        if self.loc_format == 0 {
            let a = Self::read_u16(self.data, self.loca_off + g * 2)? as usize * 2;
            let b = Self::read_u16(self.data, self.loca_off + (g + 1) * 2)? as usize * 2;
            Some((a, b))
        } else {
            let a = Self::read_u32(self.data, self.loca_off + g * 4)? as usize;
            let b = Self::read_u32(self.data, self.loca_off + (g + 1) * 4)? as usize;
            Some((a, b))
        }
    }

    fn rasterize_ascii(&self, code: u16, pixel_size: f32) -> Option<RasterizedGlyph> {
        let scale = pixel_size / self.units_per_em;
        let gid = self.glyph_id(code);
        let adv_px = ((self.advance_width(gid) as f32) * scale).round().max(4.0) as i32;
        let baseline_y = ((self.ascent as f32) * scale).round() as i32;

        let (g_start, g_end) = self.glyph_offset_range(gid)?;
        if g_start >= g_end {
            return Some(RasterizedGlyph {
                width: 0,
                height: 0,
                bearing_x: 0,
                bearing_y: baseline_y,
                advance_px: adv_px,
                coverage: Vec::new(),
            });
        }

        let goff = self.glyf_off + g_start;
        let num_contours = Self::read_i16(self.data, goff)?;
        if num_contours <= 0 {
            return Some(RasterizedGlyph {
                width: 0,
                height: 0,
                bearing_x: 0,
                bearing_y: baseline_y,
                advance_px: adv_px,
                coverage: Vec::new(),
            });
        }

        let x_min = Self::read_i16(self.data, goff + 2)? as f32 * scale;
        let y_min = Self::read_i16(self.data, goff + 4)? as f32 * scale;
        let x_max = Self::read_i16(self.data, goff + 6)? as f32 * scale;
        let y_max = Self::read_i16(self.data, goff + 8)? as f32 * scale;

        let n_cont = num_contours as usize;
        let mut end_pts = Vec::with_capacity(n_cont);
        for i in 0..n_cont {
            end_pts.push(Self::read_u16(self.data, goff + 10 + i * 2)? as usize);
        }
        let num_pts = *end_pts.last()? + 1;
        let instr_len = Self::read_u16(self.data, goff + 10 + n_cont * 2)? as usize;
        let mut cur = goff + 12 + n_cont * 2 + instr_len;

        let mut flags = Vec::with_capacity(num_pts);
        while flags.len() < num_pts {
            let f = *self.data.get(cur)?;
            cur += 1;
            flags.push(f);
            if (f & 0x08) != 0 {
                let repeat = *self.data.get(cur)? as usize;
                cur += 1;
                for _ in 0..repeat {
                    if flags.len() < num_pts {
                        flags.push(f);
                    }
                }
            }
        }

        let mut xs = Vec::with_capacity(num_pts);
        let mut x_acc = 0_i16;
        for &f in &flags {
            if (f & 0x02) != 0 {
                let dx = *self.data.get(cur)? as i16;
                cur += 1;
                x_acc += if (f & 0x10) != 0 { dx } else { -dx };
            } else if (f & 0x10) == 0 {
                let dx = Self::read_i16(self.data, cur)?;
                cur += 2;
                x_acc += dx;
            }
            xs.push(x_acc as f32 * scale);
        }

        let mut ys = Vec::with_capacity(num_pts);
        let mut y_acc = 0_i16;
        for &f in &flags {
            if (f & 0x04) != 0 {
                let dy = *self.data.get(cur)? as i16;
                cur += 1;
                y_acc += if (f & 0x20) != 0 { dy } else { -dy };
            } else if (f & 0x20) == 0 {
                let dy = Self::read_i16(self.data, cur)?;
                cur += 2;
                y_acc += dy;
            }
            ys.push(y_acc as f32 * scale);
        }

        let bx0 = x_min.floor() as i32;
        let bx1 = x_max.ceil() as i32;
        let by0 = (-y_max).floor() as i32;
        let by1 = (-y_min).ceil() as i32;
        let gw = (bx1 - bx0 + 1).clamp(1, 64) as u32;
        let gh = (by1 - by0 + 1).clamp(1, 64) as u32;

        // Flatten quadratic Bezier contours into line segments in bitmap coordinates
        let mut segments: Vec<(Point, Point)> = Vec::new();
        let mut start_idx = 0_usize;
        for &end_idx in &end_pts {
            if end_idx >= start_idx && end_idx < num_pts {
                let mut contour_pts: Vec<(Point, bool)> = Vec::new();
                for i in start_idx..=end_idx {
                    let pt = Point::new(xs[i] - bx0 as f32, -ys[i] - by0 as f32);
                    contour_pts.push((pt, (flags[i] & 0x01) != 0));
                }
                // Expand implied on-curve midpoints between consecutive off-curve points
                let mut expanded: Vec<(Point, bool)> = Vec::new();
                let n = contour_pts.len();
                for i in 0..n {
                    let curr = contour_pts[i];
                    let next = contour_pts[(i + 1) % n];
                    expanded.push(curr);
                    if !curr.1 && !next.1 {
                        expanded.push((
                            Point::new((curr.0.x + next.0.x) * 0.5, (curr.0.y + next.0.y) * 0.5),
                            true,
                        ));
                    }
                }
                // Ensure we start at an on-curve point
                if let Some(first_on) = expanded.iter().position(|p| p.1) {
                    expanded.rotate_left(first_on);
                }
                let en = expanded.len();
                let mut i = 0;
                while i < en {
                    let p0 = expanded[i].0;
                    let next = expanded[(i + 1) % en];
                    if next.1 {
                        segments.push((p0, next.0));
                        i += 1;
                    } else {
                        let ctrl = next.0;
                        let p2 = expanded[(i + 2) % en].0;
                        let mut prev = p0;
                        for step in 1..=4 {
                            let t = step as f32 * 0.25;
                            let mt = 1.0 - t;
                            let cur_pt = Point::new(
                                mt * mt * p0.x + 2.0 * mt * t * ctrl.x + t * t * p2.x,
                                mt * mt * p0.y + 2.0 * mt * t * ctrl.y + t * t * p2.y,
                            );
                            segments.push((prev, cur_pt));
                            prev = cur_pt;
                        }
                        i += 2;
                    }
                }
            }
            start_idx = end_idx + 1;
        }

        // 4x4 supersampled scanline winding rasterizer
        let mut coverage = vec![0_u8; (gw * gh) as usize];
        let mut x_hits = Vec::with_capacity(16);
        for py in 0..gh {
            let mut row_accum = vec![0_u16; gw as usize];
            for sy in 0..4 {
                let scan_y = py as f32 + (sy as f32 + 0.5) * 0.25;
                x_hits.clear();
                for &(p0, p1) in &segments {
                    if (p0.y <= scan_y && p1.y > scan_y) || (p1.y <= scan_y && p0.y > scan_y) {
                        let t = (scan_y - p0.y) / (p1.y - p0.y);
                        let ix = p0.x + t * (p1.x - p0.x);
                        let dir = if p1.y > p0.y { 1_i32 } else { -1_i32 };
                        x_hits.push((ix, dir));
                    }
                }
                x_hits.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                let mut winding = 0_i32;
                let mut span_start = 0.0_f32;
                for &(ix, dir) in &x_hits {
                    let prev_w = winding;
                    winding += dir;
                    if prev_w == 0 && winding != 0 {
                        span_start = ix;
                    } else if prev_w != 0 && winding == 0 {
                        let x0 = span_start.max(0.0);
                        let x1 = ix.min(gw as f32);
                        if x1 > x0 {
                            let px_start = x0.floor() as usize;
                            let px_end = (x1.ceil() as usize).min(gw as usize);
                            for px in px_start..px_end {
                                let left = (px as f32).max(x0);
                                let right = ((px + 1) as f32).min(x1);
                                if right > left {
                                    row_accum[px] += ((right - left) * 64.0) as u16;
                                }
                            }
                        }
                    }
                }
            }
            for px in 0..gw as usize {
                coverage[py as usize * gw as usize + px] = row_accum[px].min(255) as u8;
            }
        }

        Some(RasterizedGlyph {
            width: gw,
            height: gh,
            bearing_x: bx0,
            bearing_y: baseline_y + by0,
            advance_px: adv_px,
            coverage,
        })
    }
}

/// Renders anti-aliased Roboto / NotoSans text at 18px (body/buttons) or 22px (headlines when `is_headline` is true).
pub fn draw_text_sized_rgba(
    fb_rgba: &mut [u8],
    fb_w: u32,
    fb_h: u32,
    pos: Point,
    text: &str,
    color: Color,
    clip: Rect,
    is_headline: bool,
) {
    let font = CachedFontSet::global();
    let table = if is_headline {
        &font.title_22px
    } else {
        &font.body_18px
    };

    let cr = (color.r.clamp(0.0, 1.0) * 255.0) as u32;
    let cg = (color.g.clamp(0.0, 1.0) * 255.0) as u32;
    let cb = (color.b.clamp(0.0, 1.0) * 255.0) as u32;

    let mut pen_x = pos.x.round() as i32;
    let pen_y = pos.y.round() as i32;
    let clip_x0 = clip.x.floor() as i32;
    let clip_y0 = clip.y.floor() as i32;
    let clip_x1 = (clip.x + clip.width).ceil() as i32;
    let clip_y1 = (clip.y + clip.height).ceil() as i32;

    for ch in text.chars() {
        let code = (ch as usize).min(127);
        if let Some(Some(glyph)) = table.get(code) {
            if pen_x + glyph.advance_px > clip_x1 - 2 {
                break;
            }
            let gx0 = pen_x + glyph.bearing_x;
            let gy0 = pen_y + glyph.bearing_y;

            for gy in 0..glyph.height as i32 {
                let py = gy0 + gy;
                if py < 0 || py >= fb_h as i32 || py < clip_y0 || py >= clip_y1 {
                    continue;
                }
                let row_off = (py as u32 * fb_w) as usize;
                let g_row_off = (gy as u32 * glyph.width) as usize;
                for gx in 0..glyph.width as i32 {
                    let cov = glyph.coverage[g_row_off + gx as usize] as u32;
                    if cov == 0 {
                        continue;
                    }
                    let px = gx0 + gx;
                    if px >= 0 && px < fb_w as i32 && px >= clip_x0 && px < clip_x1 {
                        let idx = (row_off + px as usize) * 4;
                        if idx + 3 < fb_rgba.len() {
                            let inv = 255 - cov;
                            fb_rgba[idx] = ((fb_rgba[idx] as u32 * inv + cr * cov) / 255) as u8;
                            fb_rgba[idx + 1] =
                                ((fb_rgba[idx + 1] as u32 * inv + cg * cov) / 255) as u8;
                            fb_rgba[idx + 2] =
                                ((fb_rgba[idx + 2] as u32 * inv + cb * cov) / 255) as u8;
                            fb_rgba[idx + 3] = 255;
                        }
                    }
                }
            }
            pen_x += glyph.advance_px;
        } else {
            pen_x += 10;
        }
    }
}

/// Renders anti-aliased 18px Roboto / NotoSans text into an RGBA8 buffer (`fb_w x fb_h`) clipped to `clip`.
pub fn draw_text_rgba(
    fb_rgba: &mut [u8],
    fb_w: u32,
    fb_h: u32,
    pos: Point,
    text: &str,
    color: Color,
    clip: Rect,
) {
    draw_text_sized_rgba(fb_rgba, fb_w, fb_h, pos, text, color, clip, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truetype_roboto_font_rasterizer_loaded() {
        let font = CachedFontSet::global();
        let glyph_a = font.body_18px[b'A' as usize]
            .as_ref()
            .expect("Expected TrueType glyph 'A' to rasterize");
        assert!(glyph_a.width >= 8 && glyph_a.height >= 10);
        assert!(glyph_a.coverage.iter().any(|&c| c > 128));
    }

    #[test]
    fn test_multiline_wrapping_and_visible_culling() {
        let sample = "Iguanodon renders crisp GPU text across Vulkan surfaces.\nLine two has extra words to wrap.";
        let layout = MultilineTextLayout::shape(sample, 18.0, 1.35, 220.0);
        assert!(layout.lines.len() >= 3);
        let vis = layout.visible_lines(0.0, 40.0);
        assert!(!vis.is_empty());
        assert!(vis.len() <= layout.lines.len());
    }

    #[test]
    fn test_caret_hit_test_roundtrip() {
        let text = "Vulkan UI Toolkit";
        let layout = MultilineTextLayout::shape(text, 18.0, 1.3, 400.0);
        let pos = layout.caret_position(text, 7); // Right at 'U'
        let idx = layout.byte_index_at_point(text, pos);
        assert_eq!(idx, 7);
    }
}
