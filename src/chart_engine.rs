//! Backend-neutral chart geometry and interaction state.
//!
//! The shape follows the strongest parts of Lumen Charts' canvas architecture:
//! logical coordinates are independent from paint calls, visible work is clipped,
//! cursor damage is cheaper than series damage, and zoom preserves its anchor.

use crate::contracts::Bar;

pub const MIN_BAR_SPACING: f32 = 3.0;
pub const MAX_BAR_SPACING: f32 = 24.0;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum InvalidationLevel {
    #[default]
    None = 0,
    Cursor = 1,
    Series = 2,
    Layout = 3,
}

impl InvalidationLevel {
    #[inline]
    pub fn merge(&mut self, other: Self) {
        *self = (*self).max(other);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChartViewport {
    pub bar_spacing: f32,
    pub scroll_offset: f32,
}

impl Default for ChartViewport {
    fn default() -> Self {
        Self {
            bar_spacing: 9.0,
            scroll_offset: 0.0,
        }
    }
}

impl ChartViewport {
    #[inline]
    pub fn first_visible_index(self, bar_count: usize, plot_width: f32) -> f32 {
        bar_count as f32 - plot_width / self.bar_spacing + self.scroll_offset
    }

    #[inline]
    pub fn logical_to_x(
        self,
        logical_index: f32,
        bar_count: usize,
        plot_left: f32,
        plot_width: f32,
    ) -> f32 {
        let first = self.first_visible_index(bar_count, plot_width);
        plot_left + (logical_index - first + 0.5) * self.bar_spacing
    }

    #[inline]
    pub fn x_to_logical(self, x: f32, bar_count: usize, plot_left: f32, plot_width: f32) -> f32 {
        let first = self.first_visible_index(bar_count, plot_width);
        first + (x - plot_left) / self.bar_spacing - 0.5
    }

    pub fn nearest_index(
        self,
        x: f32,
        bar_count: usize,
        plot_left: f32,
        plot_width: f32,
    ) -> Option<usize> {
        let index = self
            .x_to_logical(x, bar_count, plot_left, plot_width)
            .round() as i64;
        (index >= 0 && index < bar_count as i64).then_some(index as usize)
    }

    pub fn visible_range(self, bar_count: usize, plot_width: f32) -> std::ops::Range<usize> {
        let first = self
            .first_visible_index(bar_count, plot_width)
            .floor()
            .max(0.0) as usize;
        let last = (bar_count as f32 + self.scroll_offset)
            .ceil()
            .clamp(0.0, bar_count as f32) as usize;
        first.min(last)..last
    }

    pub fn zoom(&mut self, factor: f32, anchor_ratio: f32, plot_width: f32, bar_count: usize) {
        let old_spacing = self.bar_spacing;
        let new_spacing = (old_spacing * factor).clamp(MIN_BAR_SPACING, MAX_BAR_SPACING);
        if (new_spacing - old_spacing).abs() < f32::EPSILON {
            return;
        }

        let anchor_x = anchor_ratio.clamp(0.0, 1.0) * plot_width;
        let anchor_index =
            self.first_visible_index(bar_count, plot_width) + anchor_x / old_spacing - 0.5;
        self.bar_spacing = new_spacing;
        let new_first = anchor_index - anchor_x / new_spacing + 0.5;
        self.scroll_offset = new_first - bar_count as f32 + plot_width / new_spacing;
        self.clamp_scroll(bar_count);
    }

    pub fn pan_pixels(&mut self, delta_x: f32, bar_count: usize) {
        self.scroll_offset -= delta_x / self.bar_spacing;
        self.clamp_scroll(bar_count);
    }

    #[inline]
    fn clamp_scroll(&mut self, bar_count: usize) {
        self.scroll_offset = self.scroll_offset.clamp(-(bar_count as f32), 6.0);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PriceScale {
    low: f32,
    high: f32,
}

impl PriceScale {
    pub fn from_visible_bars(bars: &[Bar]) -> Option<Self> {
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for bar in bars {
            low = low.min(bar.low);
            high = high.max(bar.high);
        }
        if !low.is_finite() || !high.is_finite() {
            return None;
        }
        let padding = ((high - low) * 0.09).max(0.01);
        Some(Self {
            low: low - padding,
            high: high + padding,
        })
    }

    #[inline]
    pub fn price_to_y(self, price: f32, top: f32, height: f32) -> f32 {
        top + (self.high - price) / (self.high - self.low).max(f32::EPSILON) * height
    }

    #[inline]
    pub fn y_to_price(self, y: f32, top: f32, height: f32) -> f32 {
        self.high - ((y - top) / height.max(1.0)) * (self.high - self.low)
    }
}

#[inline]
pub fn snap_device_pixel(coordinate: f32, scale_factor: f32) -> f32 {
    if scale_factor <= 0.0 {
        return coordinate;
    }
    ((coordinate * scale_factor - 0.5).round() + 0.5) / scale_factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_coordinate_round_trip_is_stable() {
        let viewport = ChartViewport::default();
        let x = viewport.logical_to_x(42.0, 100, 10.0, 720.0);
        let logical = viewport.x_to_logical(x, 100, 10.0, 720.0);
        assert!((logical - 42.0).abs() < 0.001);
    }

    #[test]
    fn zoom_keeps_anchor_bar_stationary() {
        let mut viewport = ChartViewport::default();
        let before = viewport.x_to_logical(540.0, 200, 0.0, 720.0);
        viewport.zoom(1.4, 0.75, 720.0, 200);
        let after = viewport.x_to_logical(540.0, 200, 0.0, 720.0);
        assert!((before - after).abs() < 0.001);
    }

    #[test]
    fn cursor_damage_is_subsumed_by_series_damage() {
        let mut damage = InvalidationLevel::Cursor;
        damage.merge(InvalidationLevel::Series);
        damage.merge(InvalidationLevel::Cursor);
        assert_eq!(damage, InvalidationLevel::Series);
    }

    #[test]
    fn visible_range_is_clipped_to_dense_storage() {
        let mut viewport = ChartViewport::default();
        viewport.pan_pixels(320.0, 100);
        let visible = viewport.visible_range(100, 480.0);
        assert!(visible.start <= visible.end);
        assert!(visible.end <= 100);
    }

    #[test]
    fn pixel_snap_targets_physical_pixel_center() {
        assert!((snap_device_pixel(100.3, 2.0) - 100.25).abs() < 0.001);
    }
}
