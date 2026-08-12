#![cfg(feature = "desktop")]

use northstar_index_fund::chart_engine::{ChartViewport, PriceScale};
use northstar_index_fund::contracts::Bar;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[test]
#[ignore = "manual release-mode performance gate"]
fn dense_chart_prepare_stays_interactive() {
    let bars: Vec<_> = (0..2_048)
        .map(|index| {
            let base = 20_000.0 + (index as f32 * 0.013).sin() * 70.0;
            Bar {
                timestamp_ns: index,
                open: base - 2.0,
                high: base + 6.0,
                low: base - 7.0,
                close: base + 1.5,
                volume: 2_000.0 + index as f32,
                flags: 0,
            }
        })
        .collect();
    let mut viewport = ChartViewport::default();
    let started = Instant::now();
    let mut checksum = 0.0f32;

    for iteration in 0..20_000 {
        let anchor = (iteration % 100) as f32 / 100.0;
        viewport.zoom(1.0002, anchor, 1_024.0, bars.len());
        viewport.pan_pixels((iteration % 3) as f32 - 1.0, bars.len());
        let visible = viewport.visible_range(bars.len(), 1_024.0);
        let scale = PriceScale::from_visible_bars(&bars[visible]).expect("finite bars");
        checksum += scale.price_to_y(bars[iteration % bars.len()].close, 0.0, 640.0);
    }

    let elapsed = started.elapsed();
    black_box(checksum);
    eprintln!("chart_prepare_20k={elapsed:?}");
    assert!(
        elapsed < Duration::from_secs(1),
        "chart preparation exceeded the one-second smoke ceiling: {elapsed:?}"
    );
}
