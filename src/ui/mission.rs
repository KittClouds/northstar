use super::{
    panel_header, status_color, status_label, NorthstarApp, ACCENT, ACCENT_DIM, BORDER, BORDER_DIM,
    CANVAS, DANGER, LAVENDER, SURFACE, SURFACE_ACTIVE, SURFACE_ALT, TEXT, TEXT_DIM, TEXT_MUTED,
    WARNING,
};
use crate::chart::index_chart;
use crate::chart_engine::InvalidationLevel;
use crate::contracts::MarketSession;
use crate::runtime::TradingControlPort;
use gpui::{
    div, prelude::*, px, relative, rgb, Context, CursorStyle, FontWeight, IntoElement,
    MouseMoveEvent, ScrollWheelEvent, SharedString,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::Sizable;

impl NorthstarApp {
    pub(super) fn render_mission_control(
        &self,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .overflow_hidden()
            .when(!compact, |layout| layout.child(self.render_universe(cx)))
            .child(self.render_desk_center(cx))
            .child(self.render_risk_panel(compact, cx))
    }

    fn render_universe(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div().flex_1().min_h_0().overflow_hidden();
        for (slot, index) in self.snapshot.indices.iter().enumerate() {
            let selected = index.key == self.selected_index;
            let key = index.key;
            let change_color = if index.daily_change_pct >= 0.0 {
                ACCENT
            } else {
                DANGER
            };
            let session = match index.session {
                MarketSession::Open => format!("basis {:+.1}", index.basis_points),
                MarketSession::Closed => "closed".to_string(),
                MarketSession::PreMarket => "pre-market".to_string(),
            };
            list = list.child(
                div()
                    .id(("universe-index", slot))
                    .h(px(70.0))
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_l_2()
                    .border_color(rgb(if selected { ACCENT } else { BORDER_DIM }))
                    .bg(rgb(if selected { SURFACE_ACTIVE } else { 0x131716 }))
                    .cursor_pointer()
                    .hover(|item| item.bg(rgb(0x1a2320)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.selected_index != key {
                            this.selected_index = key;
                            cx.notify();
                        }
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(TEXT))
                                    .child(index.key.symbol()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(change_color))
                                    .child(format!("{:+.2}%", index.daily_change_pct)),
                            ),
                    )
                    .child(
                        div()
                            .mt_1()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format_price(index.reference_price))
                            .child(session),
                    ),
            );
        }
        div()
            .w(px(204.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(BORDER))
            .bg(rgb(0x131716))
            .child(panel_header("INDEX UNIVERSE", "6 bound"))
            .child(list)
    }

    fn render_desk_center(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected();
        div()
            .flex_1()
            .w_0()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(rgb(CANVAS))
            .child(
                div()
                    .h(px(64.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(0x151918))
                    .child(
                        div()
                            .min_w_0()
                            .child(div().text_lg().font_weight(FontWeight::BOLD).child(format!(
                                "{} / {}",
                                selected.key.symbol(),
                                selected.display_name
                            )))
                            .child(div().text_xs().text_color(rgb(TEXT_MUTED)).child(format!(
                                "Massive {} reference  /  TradeLocker {} execution CFD",
                                selected.massive_ticker, selected.tradelocker_symbol
                            ))),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .text_right()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(ACCENT))
                                    .child(format_price(selected.reference_price)),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_MUTED)).child(format!(
                                "exec {}  /  spread {:.1}",
                                format_price(selected.execution_price),
                                selected.spread_points
                            ))),
                    ),
            )
            .child(self.render_session_strip())
            .child(self.render_chart_tools(cx))
            .child(
                div()
                    .id("native-chart-canvas")
                    .h(px(306.0))
                    .flex_shrink_0()
                    .relative()
                    .mx_2()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .cursor(CursorStyle::Crosshair)
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        let cursor_changed = this.chart_cursor != Some(event.position);
                        let mut redraw = cursor_changed;
                        if event.dragging() {
                            if let Some(previous) = this.chart_drag_position {
                                let delta_x = f32::from(event.position.x - previous.x);
                                if delta_x != 0.0 {
                                    let bar_count = this.selected().bars.bars().len();
                                    this.chart_viewport.pan_pixels(delta_x, bar_count);
                                    this.chart_damage.merge(InvalidationLevel::Series);
                                    redraw = true;
                                }
                            }
                            this.chart_drag_position = Some(event.position);
                        } else {
                            this.chart_drag_position = None;
                            if cursor_changed {
                                this.chart_damage.merge(InvalidationLevel::Cursor);
                            }
                        }
                        this.chart_cursor = Some(event.position);
                        if redraw {
                            cx.notify();
                        }
                    }))
                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                        let window_width = f32::from(window.viewport_size().width);
                        let side_width = if window_width >= 1_460.0 {
                            480.0
                        } else {
                            236.0
                        };
                        let plot_width = (window_width - side_width).max(320.0);
                        let plot_left = if window_width >= 1_460.0 { 204.0 } else { 0.0 };
                        let anchor = (f32::from(event.position.x) - plot_left) / plot_width;
                        let delta_y = f32::from(event.delta.pixel_delta(px(18.0)).y);
                        if delta_y.abs() < f32::EPSILON {
                            return;
                        }
                        let factor = if delta_y < 0.0 { 1.12 } else { 0.89 };
                        let bar_count = this.selected().bars.bars().len();
                        this.chart_viewport
                            .zoom(factor, anchor, plot_width, bar_count);
                        this.chart_damage.merge(InvalidationLevel::Series);
                        cx.notify();
                    }))
                    .child(index_chart(
                        selected.bars.clone(),
                        self.chart_viewport,
                        self.chart_cursor,
                        self.overlays_visible,
                    ))
                    .child(
                        div()
                            .absolute()
                            .top(px(14.0))
                            .left(px(18.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("Regime / {}", selected.regime)),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_xs()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Reference / fair value / decision markers"),
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .right(px(10.0))
                            .top(px(116.0))
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(ACCENT))
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x0a1b15))
                            .child(format_price(selected.reference_price)),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom(px(10.0))
                            .left(px(18.0))
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x151b19))
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child("Wheel zoom  /  drag pan  /  crosshair inspect"),
                    ),
            )
            .child(self.render_decision_ledger(cx))
    }

    fn render_chart_tools(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tools = div()
            .h_10()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_1()
            .px_3()
            .text_xs()
            .text_color(rgb(TEXT_MUTED));
        for (slot, label) in ["1m", "5m", "15m", "1h"].into_iter().enumerate() {
            let selected = self.timeframe == slot;
            tools = tools.child(
                div()
                    .id(("timeframe", slot))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(if selected { BORDER } else { CANVAS }))
                    .bg(rgb(if selected { SURFACE_ALT } else { CANVAS }))
                    .text_color(rgb(if selected { TEXT } else { TEXT_MUTED }))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.timeframe != slot {
                            this.timeframe = slot;
                            cx.notify();
                        }
                    }))
                    .child(label),
            );
        }
        tools
            .child(
                div()
                    .id("toggle-overlays")
                    .ml_2()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(rgb(if self.overlays_visible {
                        ACCENT_DIM
                    } else {
                        CANVAS
                    }))
                    .text_color(rgb(if self.overlays_visible {
                        ACCENT
                    } else {
                        TEXT_MUTED
                    }))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.overlays_visible = !this.overlays_visible;
                        cx.notify();
                    }))
                    .child("Signals + risk bands"),
            )
            .child(div().ml_auto().text_color(rgb(TEXT_DIM)).child(format!(
                "native GPUI  /  {:.1}px bar",
                self.chart_viewport.bar_spacing
            )))
    }

    fn render_session_strip(&self) -> impl IntoElement {
        let macro_office = &self.snapshot.macro_office;
        let operations = &self.snapshot.operations;
        let selected = self.selected();
        div()
            .h(px(56.0))
            .flex_shrink_0()
            .grid()
            .grid_cols(5)
            .border_b_1()
            .border_color(rgb(BORDER_DIM))
            .bg(rgb(0x121615))
            .child(session_item(
                "NEXT RISK",
                macro_office.next_release,
                WARNING,
            ))
            .child(session_item("MACRO AS-OF", macro_office.as_of, LAVENDER))
            .child(session_item(
                "RISK CAPACITY",
                format!(
                    "{:.2}R remaining",
                    self.snapshot.fund_detail.remaining_daily_r
                ),
                ACCENT,
            ))
            .child(session_item(
                "PUBLICATION",
                format!(
                    "seq {} / {} ms",
                    operations.publication_sequence, operations.snapshot_age_ms
                ),
                ACCENT,
            ))
            .child(session_item(
                "SESSION PLAN",
                format!("{} focus / simulation", selected.key.symbol()),
                TEXT,
            ))
    }

    fn render_decision_ledger(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows = div().flex_1().min_h_0().overflow_hidden();
        for decision in self.snapshot.decisions.iter().take(3) {
            rows = rows.child(
                div()
                    .min_h(px(66.0))
                    .grid()
                    .grid_cols(6)
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(BORDER_DIM))
                    .text_sm()
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xb9c5c1))
                            .child(decision.time_label),
                    )
                    .child(
                        div()
                            .col_span(3)
                            .child(
                                div().child(format!("{} / {}", decision.system, decision.summary)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_DIM))
                                    .child(decision.evidence),
                            ),
                    )
                    .child(
                        div()
                            .text_color(rgb(status_color(decision.outcome)))
                            .child(status_label(decision.outcome)),
                    )
                    .child(
                        div()
                            .text_right()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!("#{:04}", decision.receipt_id)),
                    ),
            );
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .h_10()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_5()
                    .px_3()
                    .bg(rgb(SURFACE))
                    .text_sm()
                    .child(div().text_color(rgb(TEXT)).child("Decision ledger"))
                    .child(div().text_color(rgb(TEXT_MUTED)).child("Orders"))
                    .child(div().text_color(rgb(TEXT_MUTED)).child("Positions"))
                    .child(div().text_color(rgb(TEXT_MUTED)).child("Events")),
            )
            .child(rows)
    }

    fn render_risk_panel(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let risk = self.snapshot.fund.risk;
        let used = (risk.daily_used_r / risk.daily_limit_r).clamp(0.0, 1.0);
        div()
            .w(px(if compact { 236.0 } else { 276.0 }))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(BORDER))
            .bg(rgb(0x131716))
            .child(panel_header("RISK + EXECUTION", "account 02"))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .child(risk_row(
                        "Daily risk used",
                        format!("{:.2} / {:.2}R", risk.daily_used_r, risk.daily_limit_r),
                    ))
                    .child(
                        div()
                            .h_1p5()
                            .mt_2()
                            .mb_3()
                            .rounded_full()
                            .bg(rgb(0x28312e))
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(used))
                                    .rounded_full()
                                    .bg(rgb(ACCENT)),
                            ),
                    )
                    .child(risk_section(
                        "OPEN EXPOSURE",
                        [
                            ("Risk", format!("{:.2}R", risk.open_exposure_r)),
                            (
                                "Max loss if stopped",
                                format!("${:.0}", risk.max_loss_dollars),
                            ),
                        ],
                    ))
                    .child(risk_section(
                        "POSITION / US100 LONG",
                        [
                            ("Entry / stop", "20,097 / 20,072".to_string()),
                            ("Unrealized", "+$128".to_string()),
                        ],
                    ))
                    .child(
                        div()
                            .mt_3()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(0x3b4944))
                            .bg(rgb(0x171d1b))
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Live order gate is closed"),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_sm()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Simulation and shadow decisions only."),
                            )
                            .child(
                                Button::new("review-arming")
                                    .label("Review arming checklist")
                                    .small()
                                    .warning()
                                    .mt_3()
                                    .w_full()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.arming_review_open = true;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("pause-orders")
                                    .label("Pause new")
                                    .small()
                                    .ghost(),
                            )
                            .child(
                                Button::new("global-halt")
                                    .label("Global halt")
                                    .small()
                                    .danger(),
                            ),
                    ),
            )
    }

    pub(super) fn render_arming_review(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let gate = self.runtime.live_gate();
        let checks = [
            ("Massive feed fresh", gate.massive_fresh),
            ("TradeLocker SyncEnd received", gate.tradelocker_synced),
            ("Nautilus reconciliation complete", gate.nautilus_reconciled),
            ("Clock synchronized", gate.clock_synchronized),
            ("No unknown orders", gate.no_unknown_orders),
            (
                "Instrument contracts agree",
                gate.instrument_contracts_agree,
            ),
        ];
        let mut list = div().mt_3().flex().flex_col().gap_2();
        for (label, passed) in checks {
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(SURFACE_ALT))
                    .child(label)
                    .child(
                        div()
                            .text_color(rgb(if passed { ACCENT } else { DANGER }))
                            .child(if passed { "PASS" } else { "BLOCK" }),
                    ),
            );
        }
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgba(0x050807d9))
            .child(
                div()
                    .w(px(480.0))
                    .p_5()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SURFACE))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .child("Live arming review"),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child("Prototype mode cannot transmit orders. These gates preview the production contract."),
                    )
                    .child(list)
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("close-arming")
                                    .label("Close")
                                    .small()
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.arming_review_open = false;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("arming-disabled")
                                    .label(if gate.ready() {
                                        "Prototype only / live disabled"
                                    } else {
                                        "Blocked"
                                    })
                                    .small()
                                    .warning(),
                            ),
                    ),
            )
    }
}

fn session_item(
    label: &'static str,
    value: impl Into<SharedString>,
    color: u32,
) -> impl IntoElement {
    div()
        .min_w_0()
        .px_3()
        .py_2()
        .border_r_1()
        .border_color(rgb(BORDER_DIM))
        .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(label))
        .child(
            div()
                .mt_1()
                .truncate()
                .text_sm()
                .text_color(rgb(color))
                .child(value.into()),
        )
}

fn risk_row(label: &'static str, value: String) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .text_sm()
        .child(div().text_color(rgb(TEXT_MUTED)).child(label))
        .child(div().font_weight(FontWeight::BOLD).child(value))
}

fn risk_section<const N: usize>(
    title: &'static str,
    rows: [(&'static str, String); N],
) -> impl IntoElement {
    let mut section = div()
        .py_3()
        .border_t_1()
        .border_color(rgb(BORDER_DIM))
        .child(
            div()
                .mb_2()
                .text_xs()
                .text_color(rgb(TEXT_DIM))
                .child(title),
        );
    for (label, value) in rows {
        section = section.child(risk_row(label, value));
    }
    section
}

fn format_price(value: f32) -> SharedString {
    format!("{value:.1}").into()
}
