use crate::chart_engine::{snap_device_pixel, ChartViewport, PriceScale};
use crate::contracts::BarSeries;
use gpui::{
    canvas, fill, point, prelude::*, px, rgb, rgba, size, Bounds, Half, IntoElement, PathBuilder,
    Pixels, Point, Window,
};

const GRID: u32 = 0x26302d;
const GRID_BRIGHT: u32 = 0x34413d;
const UP: u32 = 0x57dfbb;
const DOWN: u32 = 0xe47069;
const FAIR_VALUE: u32 = 0xd3a557;
const REFERENCE: u32 = 0x4d897a;

pub fn index_chart(
    series: BarSeries,
    viewport: ChartViewport,
    cursor: Option<Point<Pixels>>,
    overlays_visible: bool,
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            paint_chart(bounds, window, &series, viewport, cursor, overlays_visible)
        },
    )
    .size_full()
    .min_w_0()
    .min_h_0()
}

pub fn equity_chart() -> impl IntoElement {
    const EQUITY: [f32; 30] = [
        100_000.0, 100_140.0, 100_225.0, 100_370.0, 100_510.0, 100_460.0, 100_640.0, 100_790.0,
        100_730.0, 100_905.0, 101_020.0, 100_940.0, 101_140.0, 101_260.0, 101_210.0, 101_340.0,
        101_470.0, 101_420.0, 101_590.0, 101_700.0, 101_645.0, 101_790.0, 101_735.0, 101_880.0,
        101_820.0, 101_970.0, 102_080.0, 101_980.0, 102_170.0, 101_842.0,
    ];
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let left = bounds.left() + px(20.0);
            let right = bounds.right() - px(20.0);
            let top = bounds.top() + px(18.0);
            let bottom = bounds.bottom() - px(18.0);
            paint_grid(left, right, top, bottom, window);
            let minimum = EQUITY.iter().copied().fold(f32::INFINITY, f32::min) - 120.0;
            let maximum = EQUITY.iter().copied().fold(f32::NEG_INFINITY, f32::max) + 120.0;
            let width = f32::from(right - left);
            let height = f32::from(bottom - top);
            let mut path = PathBuilder::stroke(px(2.5));
            for (index, value) in EQUITY.iter().copied().enumerate() {
                let x = left + px(width * index as f32 / (EQUITY.len() - 1) as f32);
                let y = top + px((maximum - value) / (maximum - minimum) * height);
                if index == 0 {
                    path.move_to(point(x, y));
                } else {
                    path.line_to(point(x, y));
                }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(UP));
            }
        },
    )
    .size_full()
}

fn paint_chart(
    bounds: Bounds<Pixels>,
    window: &mut Window,
    series: &BarSeries,
    viewport: ChartViewport,
    cursor: Option<Point<Pixels>>,
    overlays_visible: bool,
) {
    let bars = series.bars();
    if bars.len() < 2 {
        return;
    }

    let left = bounds.left() + px(18.0);
    let right = bounds.right() - px(18.0);
    let top = bounds.top() + px(16.0);
    let bottom = bounds.bottom() - px(20.0);
    let width = f32::from(right - left).max(1.0);
    let height = f32::from(bottom - top).max(1.0);

    let visible = viewport.visible_range(bars.len(), width);
    if visible.is_empty() {
        return;
    }

    paint_grid(left, right, top, bottom, window);

    let Some(price_scale) = PriceScale::from_visible_bars(&bars[visible.clone()]) else {
        return;
    };
    let top_value = f32::from(top);
    let left_value = f32::from(left);
    let y = |price: f32| px(price_scale.price_to_y(price, top_value, height));
    let x =
        |logical: usize| px(viewport.logical_to_x(logical as f32, bars.len(), left_value, width));
    let body_width = px((viewport.bar_spacing * 0.56).clamp(2.0, 13.0));

    if overlays_visible {
        let reference = bars.last().map_or(0.0, |bar| bar.close);
        let upper = y(reference * 1.0012);
        let lower = y(reference * 0.9988);
        window.paint_quad(fill(
            Bounds::new(point(left, upper), size(right - left, lower - upper)),
            rgba(0x57dfbb0d),
        ));
        let stop_top = y(reference * 0.9970);
        window.paint_quad(fill(
            Bounds::new(point(left, stop_top), size(right - left, bottom - stop_top)),
            rgba(0xe470690a),
        ));
    }

    let reference_y = y(bars.last().map_or(0.0, |bar| bar.close));
    window.paint_quad(fill(
        Bounds::new(
            point(
                left,
                px(snap_device_pixel(
                    f32::from(reference_y),
                    window.scale_factor(),
                )),
            ),
            size(right - left, px(1.0)),
        ),
        rgb(REFERENCE),
    ));

    let mut average_path = PathBuilder::stroke(px(1.5));
    let mut rolling_sum = 0.0f32;
    let mut rolling = [0.0f32; 8];
    let warmup_start = visible.start.saturating_sub(rolling.len() - 1);
    for (index, bar) in bars.iter().enumerate().take(visible.end).skip(warmup_start) {
        let slot = index % rolling.len();
        if index >= warmup_start + rolling.len() {
            rolling_sum -= rolling[slot];
        }
        rolling[slot] = bar.close;
        rolling_sum += bar.close;
        let count = (index - warmup_start + 1).min(rolling.len()) as f32;
        let average = rolling_sum / count;
        if index < visible.start {
            continue;
        }
        let position = point(x(index), y(average));
        if index == visible.start {
            average_path.move_to(position);
        } else {
            average_path.line_to(position);
        }
    }

    for (index, bar) in bars
        .iter()
        .enumerate()
        .take(visible.end)
        .skip(visible.start)
    {
        let candle_x = x(index);
        let color = if bar.close >= bar.open {
            rgb(UP)
        } else {
            rgb(DOWN)
        };
        window.paint_quad(fill(
            Bounds::new(
                point(candle_x, y(bar.high)),
                size(px(1.0), y(bar.low) - y(bar.high)),
            ),
            color,
        ));
        let open_y = y(bar.open);
        let close_y = y(bar.close);
        let body_top = open_y.min(close_y);
        let body_height = (open_y - close_y).abs().max(px(2.0));
        window.paint_quad(fill(
            Bounds::new(
                point(candle_x - body_width.half(), body_top),
                size(body_width, body_height),
            ),
            color,
        ));
    }

    if let Ok(path) = average_path.build() {
        window.paint_path(path, rgb(FAIR_VALUE));
    }

    if let Some(entry) = bars.get(bars.len().saturating_sub(15)) {
        let entry_x = x(bars.len().saturating_sub(15));
        let entry_y = y(entry.close);
        window.paint_quad(fill(
            Bounds::new(
                point(entry_x - px(4.0), entry_y - px(4.0)),
                size(px(8.0), px(8.0)),
            ),
            rgb(UP),
        ));
    }

    if let Some(cursor) = cursor.filter(|position| bounds.contains(position)) {
        let cursor_x = cursor.x.clamp(left, right);
        let cursor_y = cursor.y.clamp(top, bottom);
        let snapped_x = px(snap_device_pixel(
            f32::from(cursor_x),
            window.scale_factor(),
        ));
        let snapped_y = px(snap_device_pixel(
            f32::from(cursor_y),
            window.scale_factor(),
        ));
        window.paint_quad(fill(
            Bounds::new(point(snapped_x, top), size(px(1.0), bottom - top)),
            rgba(0xaab6b24d),
        ));
        window.paint_quad(fill(
            Bounds::new(point(left, snapped_y), size(right - left, px(1.0))),
            rgba(0xaab6b24d),
        ));

        if let Some(index) =
            viewport.nearest_index(f32::from(cursor_x), bars.len(), left_value, width)
        {
            let marker_x = x(index);
            let marker_y = y(bars[index].close);
            window.paint_quad(fill(
                Bounds::new(
                    point(marker_x - px(4.0), marker_y - px(4.0)),
                    size(px(8.0), px(8.0)),
                ),
                rgba(0xe2ebe8cc),
            ));
        }

        let _cursor_price = price_scale.y_to_price(f32::from(cursor_y), top_value, height);
    }
}

fn paint_grid(left: Pixels, right: Pixels, top: Pixels, bottom: Pixels, window: &mut Window) {
    let width = right - left;
    let height = bottom - top;
    for row in 0..=5 {
        let y = top + height * row as f32 / 5.0;
        let color = if row == 0 || row == 5 {
            rgb(GRID_BRIGHT)
        } else {
            rgb(GRID)
        };
        window.paint_quad(fill(
            Bounds::new(point(left, y), size(width, px(1.0))),
            color,
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

#[cfg(test)]
mod tests {
    use crate::contracts::{Bar, BarSeries};

    #[test]
    fn chart_series_has_room_for_indicator_warmup() {
        let bars = (0..16)
            .map(|index| Bar {
                timestamp_ns: index,
                open: 10.0,
                high: 12.0,
                low: 9.0,
                close: 11.0,
                volume: 1.0,
                flags: 0,
            })
            .collect();
        assert_eq!(BarSeries::new(bars).bars().len(), 16);
    }
}
