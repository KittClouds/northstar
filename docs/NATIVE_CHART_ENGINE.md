# Native GPUI chart engine

Northstar owns its chart canvas. Lightweight Charts remains a product-behavior
reference, not a runtime dependency. Lumen Charts 2.0.2 was studied as a Rust
rendering reference because it cleanly separates trading-chart geometry from
Vello, wgpu, Canvas2D, and femtovg backends.

## Ideas retained from the Lumen study

- Domain transforms are independent from drawing. `ChartViewport` maps logical
  bar indices to pixels and back; `PriceScale` maps prices to pixels and back.
- Only the visible contiguous bar slice is scanned and painted.
- Zoom preserves the logical bar under the pointer anchor.
- Pan is stored as a compact bar offset, not a transformed copy of history.
- Coordinates are snapped to physical pixel centers for crisp one-pixel lines.
- Damage coalesces through `None -> Cursor -> Series -> Layout`.
- The crosshair is conceptually a cheap top layer over the slower series layer.

## GPUI-specific implementation

`chart_engine.rs` is backend-neutral math and interaction state. `chart.rs` is
the GPUI paint backend and owns quads and paths. `NorthstarApp` owns the viewport,
cursor, and drag state so the chart remains a normal host-controlled page rather
than a second windowing or GPU subsystem.

The production-facing path is `operating_chart.rs`. It reads
`BarSeriesSnapshot` chunks directly, binary-searches the visible bar range, and
paints canonical candles without collecting or copying full history. Horizontal
coordinates come from session-anchored timeframe slots rather than dense array
indices, so a missing M4/M20/H2/H4 window remains a visible gap. The fixture-free
operating root displays an empty authority state until canonical bars arrive;
`northstar-operating-replay` is the separately badged deterministic QA lab.

This deliberately avoids adding Lumen, Vello, or wgpu. GPUI already owns the
window, renderer, scale factor, event dispatch, and frame lifecycle. A second
renderer would add surface ownership and composition complexity without proving
a need at the six-index prototype scale.

## Current interactions

- Move: snapped crosshair and nearest-bar close marker.
- Wheel: anchored zoom clamped to 3-24 logical pixels per bar.
- Left-drag: horizontal history pan with bounded overscroll.
- Overlay toggle: fair-value/risk shading without changing provider data.
- Timeframe: M4/M20/H2/H4 switches between canonical series and resets the
  viewport without rebuilding source history.

## Next chart milestones

1. Cache bottom-layer display primitives by `(series_generation, viewport,
   bounds, scale_factor)` and rebuild only the cursor layer on mouse movement.
2. Add stable absolute-time ticks and measured price-axis labels.
3. Benchmark canonical visible-range planning, painting, and pointer-only
   invalidation separately.
4. Add optional typed panes only when a real operating datum requires one;
   indicators remain deferred.
5. Promote a renderer abstraction only if another GPUI-native backend is real.
