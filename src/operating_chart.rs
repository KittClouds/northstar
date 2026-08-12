use crate::chart_engine::{snap_device_pixel, ChartViewport};
use crate::data_plane::bars::{CanonicalBar, Timeframe};
use crate::operating::{BarSeriesSnapshot, IndexMarketSnapshot};
use crate::ui_theme::{ACCENT, DANGER};
use gpui::{
    canvas, fill, point, prelude::*, px, rgb, rgba, size, Bounds, Half, IntoElement, Pixels, Point,
    Window,
};
use std::ops::Range;
use std::sync::Arc;

const GRID: u32 = 0x26302d;
const GRID_BRIGHT: u32 = 0x34413d;
const REFERENCE: u32 = 0x4d897a;

pub(crate) fn canonical_index_chart(
    index: Arc<IndexMarketSnapshot>,
    timeframe: Timeframe,
    viewport: ChartViewport,
    cursor: Option<Point<Pixels>>,
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            paint_canonical_chart(bounds, window, &index, timeframe, viewport, cursor)
        },
    )
    .size_full()
    .min_w_0()
    .min_h_0()
}

pub(crate) fn canonical_slot_count(index: &IndexMarketSnapshot, timeframe: Timeframe) -> usize {
    CanonicalTimeline::new(index.series(timeframe), timeframe).map_or(0, |timeline| timeline.slots)
}

fn paint_canonical_chart(
    bounds: Bounds<Pixels>,
    window: &mut Window,
    index: &IndexMarketSnapshot,
    timeframe: Timeframe,
    viewport: ChartViewport,
    cursor: Option<Point<Pixels>>,
) {
    let series = index.series(timeframe);
    let Some(timeline) = CanonicalTimeline::new(series, timeframe) else {
        return;
    };
    let left = bounds.left() + px(18.0);
    let right = bounds.right() - px(18.0);
    let top = bounds.top() + px(16.0);
    let bottom = bounds.bottom() - px(20.0);
    let width = f32::from(right - left).max(1.0);
    let height = f32::from(bottom - top).max(1.0);
    let visible_slots = viewport.visible_range(timeline.slots, width);
    let visible = timeline.bar_range(series, visible_slots.clone());
    if visible.is_empty() {
        return;
    }

    let Some((low, high)) = price_extent(series, visible.clone()) else {
        return;
    };
    paint_grid(left, right, top, bottom, window);

    let span = (high - low).max(1.0);
    let top_value = f32::from(top);
    let left_value = f32::from(left);
    let y = |price: i64| px(top_value + ((high - price as f64) / span * height as f64) as f32);
    let x = |bar: &CanonicalBar| {
        px(viewport.logical_to_x(
            timeline.logical_slot(bar) as f32,
            timeline.slots,
            left_value,
            width,
        ))
    };
    let body_width = px((viewport.bar_spacing * 0.56).clamp(2.0, 13.0));

    for bar_index in visible {
        let Some(bar) = series.get(bar_index) else {
            continue;
        };
        let candle_x = x(bar);
        let color = rgb(if bar.close >= bar.open {
            ACCENT
        } else {
            DANGER
        });
        window.paint_quad(fill(
            Bounds::new(
                point(candle_x, y(bar.high)),
                size(px(1.0), y(bar.low) - y(bar.high)),
            ),
            color,
        ));
        let open_y = y(bar.open);
        let close_y = y(bar.close);
        window.paint_quad(fill(
            Bounds::new(
                point(candle_x - body_width.half(), open_y.min(close_y)),
                size(body_width, (open_y - close_y).abs().max(px(2.0))),
            ),
            color,
        ));
    }

    if let Some(last) = series.last() {
        let reference_y = px(snap_device_pixel(
            f32::from(y(last.close)),
            window.scale_factor(),
        ));
        window.paint_quad(fill(
            Bounds::new(point(left, reference_y), size(right - left, px(1.0))),
            rgb(REFERENCE),
        ));
    }

    if let Some(cursor) = cursor.filter(|position| bounds.contains(position)) {
        paint_crosshair(cursor, left, right, top, bottom, window);
        let logical = viewport
            .x_to_logical(f32::from(cursor.x), timeline.slots, left_value, width)
            .round() as i64;
        if logical >= 0 {
            if let Some(bar) = timeline.bar_at_slot(series, logical as usize) {
                let marker_x = x(bar);
                let marker_y = y(bar.close);
                window.paint_quad(fill(
                    Bounds::new(
                        point(marker_x - px(4.0), marker_y - px(4.0)),
                        size(px(8.0), px(8.0)),
                    ),
                    rgba(0xe2ebe8cc),
                ));
            }
        }
    }
}

fn price_extent(series: &BarSeriesSnapshot, range: Range<usize>) -> Option<(f64, f64)> {
    let mut low = i64::MAX;
    let mut high = i64::MIN;
    for index in range {
        let bar = series.get(index)?;
        low = low.min(bar.low);
        high = high.max(bar.high);
    }
    if low > high {
        return None;
    }
    let padding = ((high - low) as f64 * 0.09).max(1.0);
    Some((low as f64 - padding, high as f64 + padding))
}

fn paint_crosshair(
    cursor: Point<Pixels>,
    left: Pixels,
    right: Pixels,
    top: Pixels,
    bottom: Pixels,
    window: &mut Window,
) {
    let x = px(snap_device_pixel(
        f32::from(cursor.x.clamp(left, right)),
        window.scale_factor(),
    ));
    let y = px(snap_device_pixel(
        f32::from(cursor.y.clamp(top, bottom)),
        window.scale_factor(),
    ));
    window.paint_quad(fill(
        Bounds::new(point(x, top), size(px(1.0), bottom - top)),
        rgba(0xaab6b24d),
    ));
    window.paint_quad(fill(
        Bounds::new(point(left, y), size(right - left, px(1.0))),
        rgba(0xaab6b24d),
    ));
}

fn paint_grid(left: Pixels, right: Pixels, top: Pixels, bottom: Pixels, window: &mut Window) {
    let width = right - left;
    let height = bottom - top;
    for row in 0..=5 {
        let y = top + height * row as f32 / 5.0;
        window.paint_quad(fill(
            Bounds::new(point(left, y), size(width, px(1.0))),
            rgb(if row == 0 || row == 5 {
                GRID_BRIGHT
            } else {
                GRID
            }),
        ));
    }
    for column in 0..=6 {
        let x = left + width * column as f32 / 6.0;
        window.paint_quad(fill(
            Bounds::new(point(x, top), size(px(1.0), height)),
            rgb(GRID),
        ));
    }
}

#[derive(Clone, Copy, Debug)]
struct CanonicalTimeline {
    first_open_ns: i64,
    duration_ns: i64,
    slots: usize,
}

impl CanonicalTimeline {
    fn new(series: &BarSeriesSnapshot, timeframe: Timeframe) -> Option<Self> {
        if series.timeframe != timeframe {
            return None;
        }
        let first = series.get(0)?;
        let last = series.last()?;
        let duration_ns = timeframe.duration_ns();
        let span_ns = last.ts_open_ns.checked_sub(first.ts_open_ns)?;
        if duration_ns <= 0 || span_ns < 0 {
            return None;
        }
        let slots = usize::try_from(span_ns / duration_ns)
            .ok()?
            .checked_add(1)?;
        Some(Self {
            first_open_ns: first.ts_open_ns,
            duration_ns,
            slots,
        })
    }

    #[inline]
    fn logical_slot(self, bar: &CanonicalBar) -> usize {
        bar.ts_open_ns
            .checked_sub(self.first_open_ns)
            .and_then(|offset| usize::try_from(offset / self.duration_ns).ok())
            .unwrap_or(0)
    }

    fn bar_range(self, series: &BarSeriesSnapshot, slots: Range<usize>) -> Range<usize> {
        let start_ns = self.slot_time(slots.start);
        let end_ns = self.slot_time(slots.end);
        lower_bound(series, start_ns)..lower_bound(series, end_ns)
    }

    fn bar_at_slot(self, series: &BarSeriesSnapshot, slot: usize) -> Option<&CanonicalBar> {
        let timestamp = self.slot_time(slot);
        let index = lower_bound(series, timestamp);
        series.get(index).filter(|bar| bar.ts_open_ns == timestamp)
    }

    #[inline]
    fn slot_time(self, slot: usize) -> i64 {
        i64::try_from(slot)
            .ok()
            .and_then(|slot| slot.checked_mul(self.duration_ns))
            .and_then(|offset| self.first_open_ns.checked_add(offset))
            .unwrap_or(i64::MAX)
    }
}

fn lower_bound(series: &BarSeriesSnapshot, timestamp_ns: i64) -> usize {
    let mut left = 0usize;
    let mut right = series.len;
    while left < right {
        let middle = left + (right - left) / 2;
        let Some(bar) = series.get(middle) else {
            return left;
        };
        if bar.ts_open_ns < timestamp_ns {
            left = middle + 1;
        } else {
            right = middle;
        }
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::bars::MINUTE_NS;

    fn bar(slot: i64) -> CanonicalBar {
        CanonicalBar {
            ts_open_ns: slot * 4 * MINUTE_NS,
            ts_close_ns: (slot + 1) * 4 * MINUTE_NS,
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            first_sequence: slot as u64 + 1,
            last_sequence: slot as u64 + 1,
            instrument_id: 1,
            observation_count: 1,
            revision: 0,
            derivation_version: 1,
            timeframe: Timeframe::M4 as u16,
            flags: 0,
            reserved: 0,
        }
    }

    fn series() -> BarSeriesSnapshot {
        BarSeriesSnapshot {
            timeframe: Timeframe::M4,
            blocks: Arc::from([]),
            tail: Arc::from([bar(0), bar(1), bar(4)]),
            len: 3,
            revision: 1,
        }
    }

    #[test]
    fn logical_time_preserves_missing_bar_slots() {
        let series = series();
        let timeline = CanonicalTimeline::new(&series, Timeframe::M4).unwrap();
        assert_eq!(timeline.slots, 5);
        assert_eq!(timeline.logical_slot(series.get(1).unwrap()), 1);
        assert_eq!(timeline.logical_slot(series.get(2).unwrap()), 4);
        assert!(timeline.bar_at_slot(&series, 2).is_none());
        assert_eq!(
            timeline.bar_at_slot(&series, 4).unwrap().ts_open_ns,
            16 * MINUTE_NS
        );
    }
}
