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

//! Flexbox layout solver, scroll viewport clamping, proportional scrollbar geometry,
//! and O(1) lazy-list virtualization (`LazyListLayout`) for Iguanodon.

use iguanodon_core::{NodeId, Point, Rect};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Auto,
    Px(f32),
    Fill(f32), // flex-grow weight
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutStyle {
    pub direction: FlexDirection,
    pub width: Dimension,
    pub height: Dimension,
    pub padding: f32,
    pub gap: f32,
    pub intrinsic_width: f32,
    pub intrinsic_height: f32,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            direction: FlexDirection::Column,
            width: Dimension::Auto,
            height: Dimension::Auto,
            padding: 0.0,
            gap: 0.0,
            intrinsic_width: 100.0,
            intrinsic_height: 36.0,
        }
    }
}

/// Android / Material 3 minimum interactive touch/click target size (`48.0` dp/px).
pub const MIN_TOUCH_TARGET_DP: f32 = 48.0;

/// Proportional scrollbar geometry for `ScrollableText` and `LazyColumn`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarMetrics {
    pub track_rect: Rect,
    pub thumb_rect: Rect,
    pub interactive_hit_rect: Rect,
    pub max_scroll_y: f32,
    pub clamped_scroll_y: f32,
    pub is_scrollable: bool,
}

impl ScrollbarMetrics {
    pub const DEFAULT_WIDTH: f32 = 12.0;
    pub const MIN_THUMB_HEIGHT: f32 = MIN_TOUCH_TARGET_DP;

    pub fn compute(viewport: Rect, content_height: f32, scroll_y: f32) -> Self {
        let max_scroll_y = (content_height - viewport.height).max(0.0);
        let clamped_scroll_y = scroll_y.clamp(0.0, max_scroll_y);
        let track_rect = Rect::new(
            viewport.x + viewport.width - Self::DEFAULT_WIDTH - 2.0,
            viewport.y + 2.0,
            Self::DEFAULT_WIDTH,
            (viewport.height - 4.0).max(0.0),
        );
        let interactive_hit_rect = Rect::new(
            (viewport.x + viewport.width - MIN_TOUCH_TARGET_DP).max(viewport.x),
            viewport.y,
            MIN_TOUCH_TARGET_DP.min(viewport.width),
            viewport.height.max(MIN_TOUCH_TARGET_DP),
        );

        if max_scroll_y <= 0.0 || content_height <= 0.0 {
            return Self {
                track_rect,
                thumb_rect: track_rect,
                interactive_hit_rect,
                max_scroll_y: 0.0,
                clamped_scroll_y: 0.0,
                is_scrollable: false,
            };
        }

        let visible_ratio = (viewport.height / content_height).clamp(0.02, 1.0);
        let thumb_h = (track_rect.height * visible_ratio)
            .max(Self::MIN_THUMB_HEIGHT)
            .min(track_rect.height);
        let available_track = (track_rect.height - thumb_h).max(0.0);
        let scroll_progress = clamped_scroll_y / max_scroll_y;
        let thumb_y = track_rect.y + available_track * scroll_progress;

        Self {
            track_rect,
            thumb_rect: Rect::new(track_rect.x, thumb_y, track_rect.width, thumb_h),
            interactive_hit_rect,
            max_scroll_y,
            clamped_scroll_y,
            is_scrollable: true,
        }
    }

    /// Converts a pointer Y coordinate on the scrollbar track into a content `scroll_y` offset.
    pub fn scroll_y_from_pointer(&self, pointer_y: f32) -> f32 {
        if !self.is_scrollable {
            return 0.0;
        }
        let usable = (self.track_rect.height - self.thumb_rect.height).max(1.0);
        let rel_y = (pointer_y - self.track_rect.y - self.thumb_rect.height * 0.5).clamp(0.0, usable);
        (rel_y / usable) * self.max_scroll_y
    }
}

/// O(1) Virtualized Lazy List calculator.
/// Given `total_items` (e.g., 10,000), only materializes items intersecting `viewport` plus `overscan`.
#[derive(Debug, Clone, PartialEq)]
pub struct LazyListLayout {
    pub total_items: usize,
    pub item_height: f32,
    pub item_gap: f32,
    pub overscan: usize,
    pub visible_range: Range<usize>,
    pub total_content_height: f32,
    pub scrollbar: ScrollbarMetrics,
}

impl LazyListLayout {
    pub fn compute(
        viewport: Rect,
        total_items: usize,
        item_height: f32,
        item_gap: f32,
        scroll_y: f32,
        overscan: usize,
    ) -> Self {
        let stride = (item_height + item_gap).max(1.0);
        let total_content_height = if total_items == 0 {
            0.0
        } else {
            total_items as f32 * stride - item_gap
        };
        let scrollbar = ScrollbarMetrics::compute(viewport, total_content_height, scroll_y);
        let clamped_scroll = scrollbar.clamped_scroll_y;

        if total_items == 0 || viewport.height <= 0.0 {
            return Self {
                total_items,
                item_height,
                item_gap,
                overscan,
                visible_range: 0..0,
                total_content_height,
                scrollbar,
            };
        }

        let first_visible = (clamped_scroll / stride).floor().max(0.0) as usize;
        let last_visible = ((clamped_scroll + viewport.height) / stride).ceil().max(0.0) as usize;

        let start = first_visible.saturating_sub(overscan).min(total_items);
        let end = (last_visible + overscan).min(total_items);

        Self {
            total_items,
            item_height,
            item_gap,
            overscan,
            visible_range: start..end,
            total_content_height,
            scrollbar,
        }
    }

    pub fn materialized_count(&self) -> usize {
        self.visible_range.len()
    }

    pub fn item_rect(&self, viewport: Rect, item_index: usize) -> Rect {
        let stride = (self.item_height + self.item_gap).max(1.0);
        let y = viewport.y + (item_index as f32 * stride) - self.scrollbar.clamped_scroll_y;
        let width = (viewport.width - ScrollbarMetrics::DEFAULT_WIDTH - 6.0).max(0.0);
        Rect::new(viewport.x, y, width, self.item_height)
    }
}

/// Retained layout tree solver producing physical `Rect` bounds per `NodeId`.
#[derive(Debug, Default)]
pub struct LayoutEngine {
    pub styles: HashMap<NodeId, LayoutStyle>,
    pub children: HashMap<NodeId, Vec<NodeId>>,
    pub computed_bounds: HashMap<NodeId, Rect>,
}

impl LayoutEngine {
    pub fn set_style(&mut self, node: NodeId, style: LayoutStyle) {
        self.styles.insert(node, style);
    }

    pub fn set_children(&mut self, parent: NodeId, kids: Vec<NodeId>) {
        self.children.insert(parent, kids);
    }

    pub fn compute_layout(&mut self, root: NodeId, viewport: Rect) {
        self.computed_bounds.clear();
        self.layout_node(root, viewport);
    }

    pub fn bounds(&self, node: NodeId) -> Option<Rect> {
        self.computed_bounds.get(&node).copied()
    }

    /// Hit-tests the layout tree in reverse paint order (top-most child first).
    pub fn hit_test(&self, root: NodeId, pt: Point) -> Option<NodeId> {
        let rect = self.computed_bounds.get(&root)?;
        if !rect.contains(pt) {
            return None;
        }
        if let Some(kids) = self.children.get(&root) {
            for &child in kids.iter().rev() {
                if let Some(hit) = self.hit_test(child, pt) {
                    return Some(hit);
                }
            }
        }
        Some(root)
    }

    fn layout_node(&mut self, node: NodeId, available: Rect) {
        let style = self.styles.get(&node).copied().unwrap_or_default();
        let width = match style.width {
            Dimension::Px(w) => w.min(available.width),
            Dimension::Fill(_) => available.width,
            Dimension::Auto => style.intrinsic_width.min(available.width),
        };
        let height = match style.height {
            Dimension::Px(h) => h.min(available.height),
            Dimension::Fill(_) => available.height,
            Dimension::Auto => style.intrinsic_height.min(available.height),
        };

        let self_rect = Rect::new(available.x, available.y, width, height);
        self.computed_bounds.insert(node, self_rect);

        let kids = match self.children.get(&node).cloned() {
            Some(k) if !k.is_empty() => k,
            _ => return,
        };

        let inner = Rect::new(
            self_rect.x + style.padding,
            self_rect.y + style.padding,
            (self_rect.width - style.padding * 2.0).max(0.0),
            (self_rect.height - style.padding * 2.0).max(0.0),
        );

        let total_gaps = style.gap * (kids.len().saturating_sub(1) as f32);
        match style.direction {
            FlexDirection::Column => {
                let mut fixed_h = 0.0_f32;
                let mut total_flex = 0.0_f32;
                for &child in &kids {
                    let cs = self.styles.get(&child).copied().unwrap_or_default();
                    match cs.height {
                        Dimension::Px(h) => fixed_h += h,
                        Dimension::Auto => fixed_h += cs.intrinsic_height,
                        Dimension::Fill(weight) => total_flex += weight.max(0.1),
                    }
                }
                let remaining_h = (inner.height - fixed_h - total_gaps).max(0.0);
                let mut cursor_y = inner.y;
                for child in kids {
                    let cs = self.styles.get(&child).copied().unwrap_or_default();
                    let ch = match cs.height {
                        Dimension::Px(h) => h,
                        Dimension::Auto => cs.intrinsic_height,
                        Dimension::Fill(w) => {
                            if total_flex > 0.0 {
                                remaining_h * (w.max(0.1) / total_flex)
                            } else {
                                0.0
                            }
                        }
                    };
                    let child_avail = Rect::new(inner.x, cursor_y, inner.width, ch);
                    self.layout_node(child, child_avail);
                    cursor_y += ch + style.gap;
                }
            }
            FlexDirection::Row => {
                let mut fixed_w = 0.0_f32;
                let mut total_flex = 0.0_f32;
                for &child in &kids {
                    let cs = self.styles.get(&child).copied().unwrap_or_default();
                    match cs.width {
                        Dimension::Px(w) => fixed_w += w,
                        Dimension::Auto => fixed_w += cs.intrinsic_width,
                        Dimension::Fill(weight) => total_flex += weight.max(0.1),
                    }
                }
                let remaining_w = (inner.width - fixed_w - total_gaps).max(0.0);
                let mut cursor_x = inner.x;
                for child in kids {
                    let cs = self.styles.get(&child).copied().unwrap_or_default();
                    let cw = match cs.width {
                        Dimension::Px(w) => w,
                        Dimension::Auto => cs.intrinsic_width,
                        Dimension::Fill(w) => {
                            if total_flex > 0.0 {
                                remaining_w * (w.max(0.1) / total_flex)
                            } else {
                                0.0
                            }
                        }
                    };
                    let child_avail = Rect::new(cursor_x, inner.y, cw, inner.height);
                    self.layout_node(child, child_avail);
                    cursor_x += cw + style.gap;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_list_virtualization_10k_items() {
        let viewport = Rect::new(0.0, 0.0, 320.0, 360.0);
        // 10,000 items, each 32px tall with 4px gap (stride = 36px)
        let lazy = LazyListLayout::compute(viewport, 10_000, 32.0, 4.0, 3600.0, 2);

        // At scroll_y = 3600.0, first visible is item 100 (100 * 36 = 3600)
        // Viewport holds 10 items (100..110), plus 2 overscan before & after -> 98..112 (14 items)
        assert_eq!(lazy.visible_range, 98..112);
        assert_eq!(lazy.materialized_count(), 14);
        assert!(lazy.scrollbar.is_scrollable);
        assert!(lazy.scrollbar.thumb_rect.height >= ScrollbarMetrics::MIN_THUMB_HEIGHT);
    }

    #[test]
    fn test_scrollbar_thumb_drag_mapping() {
        let viewport = Rect::new(0.0, 0.0, 300.0, 200.0);
        let sb = ScrollbarMetrics::compute(viewport, 2000.0, 0.0);
        assert!(sb.is_scrollable);

        // Dragging to bottom of track should reach max_scroll_y (1800.0)
        let bottom_scroll = sb.scroll_y_from_pointer(sb.track_rect.y + sb.track_rect.height);
        assert!((bottom_scroll - 1800.0).abs() < 1.0);
    }

    #[test]
    fn test_flexbox_row_and_column_layout() {
        let mut engine = LayoutEngine::default();
        let root = NodeId::from_raw(1);
        let left = NodeId::from_raw(2);
        let right = NodeId::from_raw(3);

        engine.set_style(
            root,
            LayoutStyle {
                direction: FlexDirection::Row,
                width: Dimension::Fill(1.0),
                height: Dimension::Fill(1.0),
                padding: 10.0,
                gap: 20.0,
                ..Default::default()
            },
        );
        engine.set_style(
            left,
            LayoutStyle {
                width: Dimension::Px(200.0),
                height: Dimension::Fill(1.0),
                ..Default::default()
            },
        );
        engine.set_style(
            right,
            LayoutStyle {
                width: Dimension::Fill(1.0),
                height: Dimension::Fill(1.0),
                ..Default::default()
            },
        );
        engine.set_children(root, vec![left, right]);

        engine.compute_layout(root, Rect::new(0.0, 0.0, 1000.0, 600.0));
        let l_rect = engine.bounds(left).unwrap();
        let r_rect = engine.bounds(right).unwrap();
        assert_eq!(l_rect, Rect::new(10.0, 10.0, 200.0, 580.0));
        assert_eq!(r_rect, Rect::new(230.0, 10.0, 760.0, 580.0));
    }
}
