use super::{format_money_micros, format_quantity_micros, OperatingApp};
use crate::data_plane::providers::macro_catalog::MacroCategory;
use crate::ledger::{LedgerEventKind, LedgerEventStatus};
use crate::operating::{Availability, OperatingMode};
use crate::operating_ui_status::{
    availability_badge, availability_color, availability_label, format_sequence, panel,
};
use crate::ui_theme::*;
use gpui::{div, prelude::*, px, rgb, FontWeight, IntoElement};
use gpui_component::scroll::ScrollableElement;

impl OperatingApp {
    pub(super) fn render_fund_board(&self, compact: bool) -> impl IntoElement {
        let fund = &self.snapshot.fund;
        let account = self.snapshot.venue.account.as_deref();
        let venue_ready = fund.availability.has_value();
        let metric_state = if venue_ready {
            fund.availability
        } else {
            Availability::AwaitingFirstReceipt
        };

        div()
            .w_full()
            .h_0()
            .flex_1()
            .min_h_0()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .child(page_title(
                        "Fund Board",
                        "Daily capital, exposure, drawdown, limits, and attribution from one venue epoch.",
                    ))
                    .child(read_only_badge("TRADELOCKER / READ ONLY", metric_state)),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(if compact { 3 } else { 5 })
                    .gap_2()
                    .child(fund_metric(
                        "NET LIQUIDATION",
                        account.map_or_else(
                            || "—".into(),
                            |value| format_money_micros(value.projected_balance_micros, &value.currency),
                        ),
                        account.map_or_else(
                            || "awaiting account state".into(),
                            |value| format!("{} available", format_money_micros(value.available_funds_micros, &value.currency)),
                        ),
                        metric_state,
                    ))
                    .child(fund_metric(
                        "TODAY",
                        account.map_or_else(
                            || "—".into(),
                            |value| format_money_micros(value.today_net_micros, &value.currency),
                        ),
                        account.map_or_else(
                            || "realized / open".into(),
                            |value| format!("{} open / {} fees", format_money_micros(value.open_net_pnl_micros, &value.currency), format_money_micros(value.today_fees_micros, &value.currency)),
                        ),
                        metric_state,
                    ))
                    .child(fund_metric("DRAWDOWN", "—", "policy limit pending", metric_state))
                    .child(fund_metric("PORTFOLIO HEAT", "—", "no inferred exposure", metric_state))
                    .child(fund_metric("RISK CAPACITY", "—", "policy + equity required", metric_state)),
            )
            .child(
                div()
                    .h(px(if compact { 250.0 } else { 286.0 }))
                    .flex_shrink_0()
                    .grid()
                    .grid_cols(if compact { 1 } else { 3 })
                    .gap_3()
                    .child(
                        panel(
                            "EQUITY + DRAWDOWN",
                            "venue observations / no interpolation",
                            empty_fund_chart(metric_state),
                        )
                        .col_span(if compact { 1 } else { 2 }),
                    )
                    .child(panel(
                        "LIMITS",
                        "health is not authorization",
                        div()
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(limit_row("Daily stop", "— / —", 0.0, metric_state))
                            .child(limit_row("Portfolio heat", "— / —", 0.0, metric_state))
                            .child(limit_row("Correlation cap", "— / —", 0.0, metric_state))
                            .child(
                                div()
                                    .mt_1()
                                    .pt_3()
                                    .border_t_1()
                                    .border_color(rgb(BORDER_DIM))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_sm()
                                    .child(div().text_color(rgb(TEXT_MUTED)).child("Stale feeds"))
                                    .child(
                                        div()
                                            .text_color(rgb(availability_color(metric_state)))
                                            .child(if venue_ready { "0" } else { "—" }),
                                    ),
                            ),
                    )),
            )
            .child(
                div()
                    .h_0()
                    .flex_1()
                    .min_h_0()
                    .grid()
                    .grid_cols(if compact { 1 } else { 12 })
                    .gap_3()
                    .child(
                        panel(
                            "INDEX EXPOSURE",
                            "risk / notional / P&L",
                            self.render_index_exposure(metric_state),
                        )
                        .col_span(if compact { 1 } else { 4 }),
                    )
                    .child(
                        panel(
                            "DETERMINISTIC STRESS",
                            "explicit assumptions",
                            stress_book(metric_state),
                        )
                        .col_span(if compact { 1 } else { 4 }),
                    )
                    .child(
                        panel(
                            "DAILY ATTRIBUTION",
                            "realized / open / fees / R",
                            attribution_book(metric_state),
                        )
                        .col_span(if compact { 1 } else { 4 }),
                    ),
            )
    }

    fn render_index_exposure(&self, state: Availability) -> impl IntoElement {
        let mut rows = div().flex_1().min_h_0().overflow_y_scrollbar();
        if self.snapshot.desk.instruments.is_empty() {
            return rows.child(centered_copy(
                "Awaiting instrument catalog",
                "Exposure rows appear only for admitted Northstar instruments.",
                state,
            ));
        }
        for instrument in self.snapshot.desk.instruments.iter() {
            let (quantity, pnl) = self.snapshot.venue.exposure_for(instrument.instrument.id);
            let currency = self
                .snapshot
                .venue
                .account
                .as_ref()
                .map_or("", |account| account.currency.as_ref());
            rows = rows.child(
                div()
                    .grid()
                    .grid_cols(5)
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
                            .child(instrument.instrument.display_name.to_string()),
                    )
                    .child(div().col_span(2).text_color(rgb(TEXT_MUTED)).child(
                        if self.snapshot.venue.generation == 0 {
                            "awaiting venue".into()
                        } else {
                            format!("{} net", format_quantity_micros(quantity))
                        },
                    ))
                    .child(div().text_color(rgb(TEXT_DIM)).child(if quantity == 0 {
                        "flat".to_string()
                    } else {
                        "open".to_string()
                    }))
                    .child(div().text_color(rgb(availability_color(state))).child(
                        if self.snapshot.venue.generation == 0 {
                            "—".into()
                        } else {
                            format_money_micros(pnl, currency)
                        },
                    )),
            );
        }
        rows
    }

    pub(super) fn render_systems_control_room(&self, compact: bool) -> impl IntoElement {
        let desk = self.snapshot.desk.availability;
        let macro_state = self.snapshot.macro_office.availability;
        let ledger = self.snapshot.ledger.availability;
        let venue = self.snapshot.fund.availability;
        let journal_sequence = self
            .snapshot
            .desk
            .last_sequence
            .max(self.snapshot.macro_office.last_sequence)
            .max(self.snapshot.ledger.last_sequence);
        let mapped = self.snapshot.desk.instruments.len();

        div()
            .w_full()
            .h_0()
            .flex_1()
            .min_h_0()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .child(page_title(
                        "Systems Control Room",
                        "Data, publication, venue, machine, storage, and replay truth.",
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(read_only_badge("OPERATIONS", Availability::Ready))
                            .child(read_only_badge(
                                "STRATEGY RUNTIME / DEFERRED",
                                Availability::Deferred,
                            )),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(if compact { 3 } else { 5 })
                    .gap_2()
                    .child(system_metric(
                        "READINESS",
                        "guarded / read only".to_string(),
                        "arming disabled",
                        venue,
                    ))
                    .child(system_metric(
                        "PUBLICATION",
                        format_sequence(journal_sequence),
                        format!("office generation {}", self.snapshot.generation.0),
                        combined_state(desk, macro_state),
                    ))
                    .child(system_metric(
                        "CATALOG",
                        format!("{mapped} mapped"),
                        "Northstar instrument identities",
                        if mapped == 0 {
                            Availability::NotConfigured
                        } else {
                            Availability::Ready
                        },
                    ))
                    .child(system_metric(
                        "REPLAY",
                        replay_label(self.snapshot.mode),
                        "same snapshot publisher",
                        replay_state(self.snapshot.mode),
                    ))
                    .child(system_metric(
                        "LEDGER",
                        format!("{} entries", self.snapshot.ledger.entry_count),
                        "mmap hot journal + cold bodies",
                        ledger,
                    )),
            )
            .child(
                div()
                    .h_0()
                    .flex_1()
                    .min_h_0()
                    .grid()
                    .grid_cols(if compact { 1 } else { 12 })
                    .gap_3()
                    .child(
                        div()
                            .col_span(if compact { 1 } else { 7 })
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(panel(
                                "OFFICE PIPELINE",
                                "sources → truth → projectors → UI",
                                self.render_pipeline_map(),
                            ))
                            .child(panel(
                                "DATA COVERAGE",
                                "catalog / observations / receipts / replay",
                                self.render_coverage_matrix(),
                            )),
                    )
                    .child(
                        div()
                            .col_span(if compact { 1 } else { 5 })
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(panel(
                                "INCIDENT TIMELINE",
                                "every material transition is a receipt",
                                self.render_incident_timeline(),
                            ))
                            .child(panel(
                                "LIVE ELIGIBILITY TRUTH",
                                "health is not authorization",
                                eligibility_truth(desk, macro_state, venue, ledger, mapped),
                            )),
                    ),
            )
    }

    fn render_pipeline_map(&self) -> impl IntoElement {
        let macro_state = self.snapshot.macro_office.availability;
        let positioning_ready = self
            .snapshot
            .macro_office
            .positioning
            .iter()
            .any(|market| market.latest_report_ns != 0);
        div()
            .p_3()
            .grid()
            .grid_cols(4)
            .gap_2()
            .child(pipeline_column(
                "DATA",
                vec![
                    (
                        "Market observations",
                        format_sequence(self.snapshot.desk.last_sequence),
                        self.snapshot.desk.availability,
                    ),
                    (
                        "Official macro",
                        format!("{} series", self.snapshot.macro_office.series.len()),
                        macro_state,
                    ),
                    (
                        "CFTC TFF",
                        if positioning_ready {
                            "current".into()
                        } else {
                            "awaiting report".into()
                        },
                        if positioning_ready {
                            Availability::Ready
                        } else {
                            Availability::AwaitingFirstReceipt
                        },
                    ),
                ],
            ))
            .child(pipeline_column(
                "TRUTH",
                vec![
                    (
                        "Canonical journal",
                        format_sequence(self.snapshot.macro_office.last_sequence),
                        combined_state(self.snapshot.desk.availability, macro_state),
                    ),
                    (
                        "Snapshot publisher",
                        format!("gen {}", self.snapshot.generation.0),
                        Availability::Ready,
                    ),
                ],
            ))
            .child(pipeline_column(
                "VENUE",
                vec![
                    (
                        "TradeLocker",
                        if self.snapshot.venue.generation == 0 {
                            "coherent epoch pending".into()
                        } else {
                            format!(
                                "epoch {} / {} positions",
                                self.snapshot.venue.generation,
                                self.snapshot.venue.positions.len()
                            )
                        },
                        self.snapshot.venue.availability,
                    ),
                    (
                        "Instrument contracts",
                        format!("{} / 6", self.snapshot.venue.contracts.len()),
                        if self.snapshot.venue.contracts.len() == 6 {
                            self.snapshot.venue.availability
                        } else {
                            Availability::AwaitingFirstReceipt
                        },
                    ),
                ],
            ))
            .child(pipeline_column(
                "MACHINE",
                vec![
                    (
                        "Replay",
                        replay_label(self.snapshot.mode),
                        replay_state(self.snapshot.mode),
                    ),
                    (
                        "Ledger",
                        format!("{} entries", self.snapshot.ledger.entry_count),
                        self.snapshot.ledger.availability,
                    ),
                    (
                        "UI invalidation",
                        format!("{} updates", self.ui_updates),
                        Availability::Ready,
                    ),
                ],
            ))
    }

    fn render_coverage_matrix(&self) -> impl IntoElement {
        let snapshot = &self.snapshot.macro_office;
        let count = |category| {
            snapshot
                .series
                .iter()
                .filter(|series| series.category == category)
                .count()
        };
        div()
            .min_h_0()
            .overflow_y_scrollbar()
            .child(coverage_header())
            .child(coverage_row(
                "Inflation",
                MacroCategory::Inflation,
                count(MacroCategory::Inflation),
                snapshot,
            ))
            .child(coverage_row(
                "Labor",
                MacroCategory::Labor,
                count(MacroCategory::Labor),
                snapshot,
            ))
            .child(coverage_row(
                "Rates",
                MacroCategory::Rates,
                count(MacroCategory::Rates),
                snapshot,
            ))
            .child(coverage_row(
                "Growth",
                MacroCategory::Growth,
                count(MacroCategory::Growth),
                snapshot,
            ))
            .child(coverage_cells(
                "Venue contracts",
                format!("{} / 6", self.snapshot.venue.contracts.len()),
                if self.snapshot.venue.generation == 0 {
                    "coherent epoch pending".into()
                } else {
                    "config-derived / account-scoped".into()
                },
                "—".into(),
                self.snapshot.venue.availability,
            ))
    }

    fn render_incident_timeline(&self) -> impl IntoElement {
        let mut body = div().min_h_0().overflow_y_scrollbar();
        let mut count = 0usize;
        for row in self
            .snapshot
            .ledger
            .rows
            .iter()
            .filter(|row| {
                row.kind == LedgerEventKind::DataIncident
                    || row.status == LedgerEventStatus::Incident
            })
            .take(8)
        {
            count += 1;
            body = body.child(
                div()
                    .p_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_xs()
                            .text_color(rgb(DANGER))
                            .child(format!("INCIDENT / {}", row.kind.label()))
                            .child(format!("#{}", row.entry_id.get())),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(row.preview.to_string()),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!("canonical {}", row.canonical_last_sequence.get())),
                    ),
            );
        }
        if count == 0 {
            body = body.child(centered_copy(
                "No material incidents in the visible Ledger window",
                "Missing authorities remain explicit readiness states below.",
                self.snapshot.ledger.availability,
            ));
        }
        body
    }
}

fn page_title(title: &'static str, detail: &'static str) -> gpui::Div {
    div()
        .child(div().text_xl().font_weight(FontWeight::BOLD).child(title))
        .child(
            div()
                .mt_1()
                .text_sm()
                .text_color(rgb(TEXT_MUTED))
                .child(detail),
        )
}

fn read_only_badge(label: &'static str, state: Availability) -> gpui::Div {
    div()
        .px_3()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(availability_color(state)))
        .bg(rgb(0x171b1a))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(availability_color(state)))
        .child(label)
}

fn fund_metric(
    label: &'static str,
    value: impl Into<String>,
    detail: impl Into<String>,
    state: Availability,
) -> gpui::Div {
    metric_card(label, value.into(), detail.into(), state)
}

fn system_metric(
    label: &'static str,
    value: String,
    detail: impl Into<String>,
    state: Availability,
) -> gpui::Div {
    metric_card(label, value, detail.into(), state)
}

fn metric_card(
    label: &'static str,
    value: String,
    detail: String,
    state: Availability,
) -> gpui::Div {
    div()
        .min_w_0()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(label))
        .child(
            div()
                .mt_3()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(availability_color(state)))
                .child(value),
        )
        .child(
            div()
                .mt_2()
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(detail),
        )
}

fn empty_fund_chart(state: Availability) -> gpui::Div {
    div()
        .size_full()
        .min_h_0()
        .relative()
        .p_3()
        .child(
            div()
                .absolute()
                .top_3()
                .left_3()
                .right_3()
                .h(px(56.0))
                .flex()
                .flex_col()
                .justify_between()
                .children((0..5).map(|_| div().h_px().w_full().bg(rgb(BORDER_DIM)))),
        )
        .child(centered_copy(
            "Awaiting first coherent account epoch",
            "Equity and drawdown history will be plotted from venue receipts; zero is never substituted for unknown.",
            state,
        ))
}

fn limit_row(
    label: &'static str,
    value: &'static str,
    ratio: f32,
    state: Availability,
) -> gpui::Div {
    div()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .text_sm()
                .child(div().text_color(rgb(TEXT_MUTED)).child(label))
                .child(value),
        )
        .child(
            div().mt_2().h_1p5().rounded_full().bg(rgb(0x29322f)).child(
                div()
                    .h_full()
                    .w(gpui::relative(ratio.clamp(0.0, 1.0)))
                    .rounded_full()
                    .bg(rgb(availability_color(state))),
            ),
        )
}

fn stress_book(state: Availability) -> impl IntoElement {
    div()
        .min_h_0()
        .overflow_y_scrollbar()
        .child(stress_row("All stops", "defined stops + slippage", state))
        .child(stress_row(
            "US cluster shock",
            "simultaneous index move",
            state,
        ))
        .child(stress_row(
            "Venue disconnect",
            "hold positions / block new route",
            state,
        ))
}

fn stress_row(label: &'static str, assumption: &'static str, state: Availability) -> gpui::Div {
    div()
        .p_3()
        .border_b_1()
        .border_color(rgb(BORDER_DIM))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().font_weight(FontWeight::BOLD).child(label))
                .child(
                    div()
                        .text_color(rgb(availability_color(state)))
                        .child("— / —"),
                ),
        )
        .child(
            div()
                .mt_2()
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(assumption),
        )
}

fn attribution_book(state: Availability) -> gpui::Div {
    centered_copy(
        "No attributable venue activity",
        "Realized, open, fee, swap, and R columns remain absent until correlated TradeLocker receipts exist.",
        state,
    )
}

fn centered_copy(title: &'static str, detail: &'static str, state: Availability) -> gpui::Div {
    div()
        .size_full()
        .min_h(px(110.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .p_4()
        .text_center()
        .child(availability_badge(state))
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .max_w(px(520.0))
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(detail),
        )
}

fn pipeline_column(
    label: &'static str,
    rows: Vec<(&'static str, String, Availability)>,
) -> gpui::Div {
    let mut column = div()
        .min_w_0()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER_DIM))
        .bg(rgb(SURFACE_ALT))
        .child(
            div()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(rgb(BORDER_DIM))
                .text_xs()
                .text_color(rgb(TEXT_DIM))
                .child(label),
        );
    for (name, detail, state) in rows {
        column = column.child(
            div()
                .p_3()
                .border_b_1()
                .border_color(rgb(BORDER_DIM))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(name),
                        )
                        .child(
                            div()
                                .size_1p5()
                                .rounded_full()
                                .bg(rgb(availability_color(state))),
                        ),
                )
                .child(
                    div()
                        .mt_2()
                        .text_xs()
                        .text_color(rgb(TEXT_MUTED))
                        .child(detail),
                ),
        );
    }
    column
}

fn coverage_header() -> gpui::Div {
    div()
        .grid()
        .grid_cols(5)
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER_DIM))
        .text_xs()
        .text_color(rgb(TEXT_DIM))
        .child("FAMILY")
        .child("CATALOG")
        .child("CURRENT")
        .child("RECEIPTS")
        .child("REPLAY")
}

fn coverage_row(
    label: &'static str,
    category: MacroCategory,
    cataloged: usize,
    snapshot: &crate::operating::MacroSnapshot,
) -> gpui::Div {
    let current = snapshot
        .series
        .iter()
        .filter(|series| series.category == category && series.latest.is_some())
        .count();
    coverage_cells(
        label,
        cataloged.to_string(),
        format!("{current} current"),
        snapshot.receipt_count.to_string(),
        snapshot.availability,
    )
}

#[allow(dead_code)]
fn coverage_static_row(
    label: &'static str,
    catalog: &'static str,
    current: &'static str,
    state: Availability,
) -> gpui::Div {
    coverage_cells(label, catalog.into(), current.into(), "—".into(), state)
}

fn coverage_cells(
    label: &'static str,
    catalog: String,
    current: String,
    receipts: String,
    state: Availability,
) -> gpui::Div {
    div()
        .grid()
        .grid_cols(5)
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER_DIM))
        .text_sm()
        .child(div().font_weight(FontWeight::SEMIBOLD).child(label))
        .child(catalog)
        .child(current)
        .child(receipts)
        .child(
            div()
                .text_color(rgb(availability_color(state)))
                .child(availability_label(state)),
        )
}

fn eligibility_truth(
    desk: Availability,
    macro_state: Availability,
    venue: Availability,
    ledger: Availability,
    mapped: usize,
) -> gpui::Div {
    div()
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(truth_row(
            "Market observations",
            availability_label(desk),
            desk,
        ))
        .child(truth_row(
            "Official macro publication",
            availability_label(macro_state),
            macro_state,
        ))
        .child(truth_row(
            "TradeLocker coherent epoch",
            availability_label(venue),
            venue,
        ))
        .child(truth_row(
            "Unknown venue outcomes",
            "not yet observable",
            Availability::NotConfigured,
        ))
        .child(truth_row(
            "Instrument contracts",
            if mapped == 0 {
                "catalog absent"
            } else {
                "venue bindings pending"
            },
            Availability::NotConfigured,
        ))
        .child(truth_row(
            "Durable Ledger",
            availability_label(ledger),
            ledger,
        ))
        .child(
            div()
                .mt_2()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(0x655033))
                .bg(rgb(0x332719))
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(WARNING))
                .child("Read-only data eligibility only / execution remains disabled"),
        )
}

fn truth_row(label: &'static str, value: &'static str, state: Availability) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER_DIM))
        .text_sm()
        .child(div().text_color(rgb(TEXT_MUTED)).child(label))
        .child(
            div().flex().items_center().gap_2().child(value).child(
                div()
                    .size_1p5()
                    .rounded_full()
                    .bg(rgb(availability_color(state))),
            ),
        )
}

fn combined_state(left: Availability, right: Availability) -> Availability {
    if left.has_value() && right.has_value() {
        Availability::Ready
    } else if matches!(left, Availability::Invalid | Availability::Disconnected)
        || matches!(right, Availability::Invalid | Availability::Disconnected)
    {
        Availability::Invalid
    } else {
        Availability::AwaitingFirstReceipt
    }
}

fn replay_label(mode: OperatingMode) -> String {
    match mode {
        OperatingMode::OfflineReplay => "exact / offline".into(),
        OperatingMode::LiveData => "live journal".into(),
        OperatingMode::Paper => "paper / journaled".into(),
        OperatingMode::Live => "live / journaled".into(),
    }
}

fn replay_state(mode: OperatingMode) -> Availability {
    match mode {
        OperatingMode::OfflineReplay => Availability::Replaying,
        _ => Availability::Ready,
    }
}
