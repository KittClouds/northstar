use super::{
    panel_header, status_color, NorthstarApp, ACCENT, BORDER, BORDER_DIM, LAVENDER, SURFACE_ALT,
    TEXT, TEXT_DIM, TEXT_MUTED, WARNING,
};
use crate::chart::equity_chart;
use gpui::{div, prelude::*, px, relative, rgb, Context, FontWeight, IntoElement};

impl NorthstarApp {
    pub(super) fn render_fund_board(
        &self,
        compact: bool,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let fund = &self.snapshot.fund;
        let risk = fund.risk;
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .p_4()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .mb_3()
                    .child(
                        div()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Fund Board"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Daily capital, risk, and strategy health."),
                            ),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x665033))
                            .bg(rgb(0x332719))
                            .text_xs()
                            .text_color(rgb(0xf0b862))
                            .child("SIMULATION / DAY 34"),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(if compact { 2 } else { 5 })
                    .gap_3()
                    .child(metric_card(
                        "NET LIQUIDATION",
                        format!("${:.0}", fund.net_liquidation),
                        format!("+{:.2}% inception", fund.inception_pct),
                        ACCENT,
                    ))
                    .child(metric_card(
                        "TODAY",
                        format!("+${:.0}", fund.today_pnl),
                        format!(
                            "${:.0} realized / ${:.0} open",
                            self.snapshot.fund_detail.realized_today,
                            self.snapshot.fund_detail.unrealized_today
                        ),
                        ACCENT,
                    ))
                    .child(metric_card(
                        "DRAWDOWN",
                        format!("{:.2}%", risk.drawdown_pct),
                        format!("limit {:.2}%", risk.drawdown_limit_pct),
                        TEXT,
                    ))
                    .child(metric_card(
                        "PORTFOLIO HEAT",
                        format!("{:.2}R", risk.portfolio_heat_r),
                        format!("limit {:.2}R", risk.portfolio_limit_r),
                        WARNING,
                    ))
                    .child(metric_card(
                        "RISK CAPACITY",
                        format!("{:.2}R", self.snapshot.fund_detail.remaining_daily_r),
                        format!("gross {:.2}x", fund.gross_exposure),
                        LAVENDER,
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .h(px(if compact { 250.0 } else { 275.0 }))
                    .grid()
                    .grid_cols(if compact { 1 } else { 3 })
                    .gap_3()
                    .child(
                        div()
                            .col_span(if compact { 1 } else { 2 })
                            .min_w_0()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .overflow_hidden()
                            .bg(rgb(0x141817))
                            .child(panel_header("EQUITY + DRAWDOWN", "30 sessions"))
                            .child(div().flex_1().min_h_0().child(equity_chart())),
                    )
                    .child(
                        div()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .overflow_hidden()
                            .bg(rgb(0x141817))
                            .child(panel_header("LIMITS", "all green"))
                            .child(
                                div()
                                    .p_3()
                                    .child(limit_row(
                                        "Daily stop",
                                        risk.daily_used_r,
                                        risk.daily_limit_r,
                                        "R",
                                    ))
                                    .child(limit_row(
                                        "Portfolio heat",
                                        risk.portfolio_heat_r,
                                        risk.portfolio_limit_r,
                                        "R",
                                    ))
                                    .child(limit_row("Correlation cap", 0.44, 0.70, ""))
                                    .child(
                                        div()
                                            .mt_3()
                                            .pt_3()
                                            .border_t_1()
                                            .border_color(rgb(BORDER_DIM))
                                            .flex()
                                            .justify_between()
                                            .text_sm()
                                            .child(
                                                div()
                                                    .text_color(rgb(TEXT_MUTED))
                                                    .child("Stale feeds"),
                                            )
                                            .child(div().text_color(rgb(ACCENT)).child("0")),
                                    ),
                            ),
                    ),
            )
            .child(self.render_fund_details(compact))
    }

    fn render_fund_details(&self, compact: bool) -> impl IntoElement {
        div()
            .mt_3()
            .flex_1()
            .min_h_0()
            .grid()
            .grid_cols(if compact { 1 } else { 3 })
            .gap_3()
            .child(self.render_exposure_book())
            .child(self.render_stress_book())
            .child(self.render_attribution_book())
    }

    fn render_exposure_book(&self) -> impl IntoElement {
        let mut rows = div();
        for exposure in self.snapshot.fund_detail.exposures.iter() {
            rows = rows.child(
                div()
                    .grid()
                    .grid_cols(6)
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .text_sm()
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .child(exposure.index.symbol()),
                    )
                    .child(
                        div()
                            .col_span(2)
                            .text_color(rgb(TEXT_MUTED))
                            .child(exposure.cluster),
                    )
                    .child(format!("{:.2}R", exposure.risk_r))
                    .child(format!("${:.0}", exposure.notional_dollars))
                    .child(
                        div()
                            .text_color(rgb(if exposure.pnl_dollars >= 0.0 {
                                ACCENT
                            } else {
                                WARNING
                            }))
                            .child(format!("{:+.0}", exposure.pnl_dollars)),
                    ),
            );
        }
        div()
            .min_h_0()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header("INDEX EXPOSURE", "risk / notional / P&L"))
            .child(rows)
    }

    fn render_stress_book(&self) -> impl IntoElement {
        let mut rows = div();
        for scenario in self.snapshot.fund_detail.stress.iter() {
            rows = rows.child(
                div()
                    .px_3()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().font_weight(FontWeight::BOLD).child(scenario.name))
                            .child(div().text_color(rgb(status_color(scenario.status))).child(
                                format!("-${:.0} / {:.2}R", scenario.loss_dollars, scenario.loss_r),
                            )),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(scenario.assumption),
                    ),
            );
        }
        div()
            .min_h_0()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header("DETERMINISTIC STRESS", "explicit assumptions"))
            .child(rows)
    }

    fn render_attribution_book(&self) -> impl IntoElement {
        let mut rows = div();
        for attribution in self.snapshot.fund_detail.attribution.iter() {
            rows = rows.child(
                div()
                    .px_3()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .child(div().font_weight(FontWeight::BOLD).child(attribution.label))
                    .child(
                        div()
                            .mt_1()
                            .grid()
                            .grid_cols(4)
                            .gap_2()
                            .text_xs()
                            .child(attribution_value("Realized", attribution.realized_dollars))
                            .child(attribution_value("Open", attribution.unrealized_dollars))
                            .child(attribution_value("Fees", attribution.fees_dollars))
                            .child(
                                div()
                                    .child(div().text_color(rgb(TEXT_DIM)).child("Total"))
                                    .child(
                                        div()
                                            .mt_1()
                                            .text_color(rgb(ACCENT))
                                            .child(format!("{:+.2}R", attribution.total_r)),
                                    ),
                            ),
                    ),
            );
        }
        div()
            .min_h_0()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header(
                "DAILY ATTRIBUTION",
                "realized / open / fees / R",
            ))
            .child(rows)
    }
}

fn metric_card(
    label: &'static str,
    value: String,
    detail: String,
    detail_color: u32,
) -> impl IntoElement {
    div()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE_ALT))
        .child(div().text_xs().text_color(rgb(TEXT_MUTED)).child(label))
        .child(
            div()
                .mt_2()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .child(value),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(detail_color))
                .child(detail),
        )
}

fn limit_row(label: &'static str, used: f32, limit: f32, unit: &'static str) -> impl IntoElement {
    let ratio = (used / limit).clamp(0.0, 1.0);
    div()
        .mb_4()
        .child(
            div()
                .flex()
                .justify_between()
                .text_sm()
                .child(div().text_color(rgb(TEXT_MUTED)).child(label))
                .child(format!("{used:.2} / {limit:.2}{unit}")),
        )
        .child(
            div().mt_2().h_1p5().rounded_full().bg(rgb(0x29322f)).child(
                div()
                    .h_full()
                    .w(relative(ratio))
                    .rounded_full()
                    .bg(rgb(ACCENT)),
            ),
        )
}

fn attribution_value(label: &'static str, value: f32) -> impl IntoElement {
    div()
        .child(div().text_color(rgb(TEXT_DIM)).child(label))
        .child(
            div()
                .mt_1()
                .text_color(rgb(if value >= 0.0 { TEXT } else { WARNING }))
                .child(format!("{value:+.0}")),
        )
}
