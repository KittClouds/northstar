use super::{
    panel_header, status_color, status_label, NorthstarApp, SystemsView, ACCENT, BORDER,
    BORDER_DIM, LAVENDER, SURFACE, SURFACE_ACTIVE, SURFACE_ALT, TEXT, TEXT_DIM, TEXT_MUTED,
    WARNING,
};
use crate::chart::index_chart;
use crate::chart_engine::ChartViewport;
use crate::contracts::GateStatus;
use crate::office::OperationGroup;
use gpui::{div, prelude::*, px, rgb, Context, FontWeight, IntoElement};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::Sizable;

impl NorthstarApp {
    pub(super) fn render_systems(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .p_4()
            .child(self.render_systems_header(cx))
            .child(match self.systems_view {
                SystemsView::Operations => self.render_operations(compact).into_any_element(),
                SystemsView::StrategyRuntime => {
                    self.render_strategy_runtime(compact, cx).into_any_element()
                }
            })
    }

    fn render_systems_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child("Systems Control Room"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child("Data, publication, engine, venue, machine, and replay truth."),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(self.systems_view_button(SystemsView::Operations, "Operations", cx))
                    .child(self.systems_view_button(
                        SystemsView::StrategyRuntime,
                        "Strategy runtime",
                        cx,
                    )),
            )
    }

    fn systems_view_button(
        &self,
        view: SystemsView,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.systems_view == view;
        div()
            .id(("systems-view", view as usize))
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(if selected { ACCENT } else { BORDER }))
            .bg(rgb(if selected {
                SURFACE_ACTIVE
            } else {
                SURFACE_ALT
            }))
            .text_sm()
            .text_color(rgb(if selected { TEXT } else { TEXT_MUTED }))
            .cursor_pointer()
            .hover(|item| item.bg(rgb(0x202724)).text_color(rgb(TEXT)))
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.systems_view != view {
                    this.systems_view = view;
                    cx.notify();
                }
            }))
            .child(label)
    }

    fn render_operations(&self, compact: bool) -> impl IntoElement {
        let operations = &self.snapshot.operations;
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .grid()
                    .grid_cols(5)
                    .gap_2()
                    .child(operation_metric(
                        "READINESS",
                        "eligible / guarded",
                        "arming remains manual",
                        ACCENT,
                    ))
                    .child(operation_metric(
                        "PUBLICATION",
                        format!("seq {}", operations.publication_sequence),
                        format!("{} ms old", operations.snapshot_age_ms),
                        ACCENT,
                    ))
                    .child(operation_metric(
                        "CATALOG",
                        format!("{}% mapped", operations.catalog_coverage_pct),
                        "DE40 TFF under review",
                        WARNING,
                    ))
                    .child(operation_metric(
                        "REPLAY",
                        if operations.replay_exact {
                            "exact"
                        } else {
                            "diverged"
                        },
                        "1,842 receipts / schema v2",
                        if operations.replay_exact {
                            ACCENT
                        } else {
                            WARNING
                        },
                    ))
                    .child(operation_metric(
                        "STORAGE",
                        format!("{} MB", operations.storage_megabytes),
                        "mmap generation 14",
                        LAVENDER,
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex_1()
                    .min_h_0()
                    .grid()
                    .grid_cols(if compact { 1 } else { 12 })
                    .gap_3()
                    .child(
                        div()
                            .col_span(if compact { 1 } else { 7 })
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(self.render_pipeline_map())
                            .child(self.render_coverage_matrix()),
                    )
                    .child(
                        div()
                            .col_span(if compact { 1 } else { 5 })
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(self.render_incident_timeline())
                            .child(self.render_system_truth_panel()),
                    ),
            )
    }

    fn render_pipeline_map(&self) -> impl IntoElement {
        let mut groups = div().grid().grid_cols(4).gap_2().p_3();
        for group in [
            OperationGroup::Data,
            OperationGroup::Engine,
            OperationGroup::Venue,
            OperationGroup::Machine,
        ] {
            let mut column = div()
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
                        .child(operation_group_label(group)),
                );
            for node in self
                .snapshot
                .operations
                .nodes
                .iter()
                .filter(|node| node.group == group)
            {
                column = column.child(
                    div()
                        .px_3()
                        .py_2()
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
                                        .font_weight(FontWeight::BOLD)
                                        .child(node.name),
                                )
                                .child(
                                    div()
                                        .size_2()
                                        .rounded_full()
                                        .bg(rgb(status_color(node.status))),
                                ),
                        )
                        .child(
                            div()
                                .mt_1()
                                .flex()
                                .justify_between()
                                .text_xs()
                                .text_color(rgb(TEXT_MUTED))
                                .child(node.state)
                                .child(node.metric),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_xs()
                                .text_color(rgb(TEXT_DIM))
                                .child(node.detail),
                        ),
                );
            }
            groups = groups.child(column);
        }
        div()
            .flex_1()
            .min_h(px(300.0))
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header(
                "OFFICE PIPELINE",
                "sources → truth → engines → UI",
            ))
            .child(groups)
    }

    fn render_coverage_matrix(&self) -> impl IntoElement {
        let mut rows = div();
        for coverage in self.snapshot.operations.coverage.iter() {
            rows = rows.child(
                div()
                    .grid()
                    .grid_cols(7)
                    .gap_2()
                    .items_center()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .text_sm()
                    .child(
                        div()
                            .col_span(2)
                            .font_weight(FontWeight::BOLD)
                            .child(coverage.family),
                    )
                    .child(coverage.cataloged)
                    .child(coverage.history)
                    .child(coverage.latest)
                    .child(div().text_color(rgb(TEXT_MUTED)).child(coverage.revisions))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(status_color(coverage.replay)))
                            .child(status_label(coverage.replay)),
                    ),
            );
        }
        div()
            .flex_shrink_0()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header(
                "DATA COVERAGE",
                "catalog / history / latest / revisions / replay",
            ))
            .child(rows)
    }

    fn render_incident_timeline(&self) -> impl IntoElement {
        let mut rows = div();
        for incident in self.snapshot.operations.incidents.iter() {
            rows =
                rows.child(
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
                                .text_xs()
                                .child(div().text_color(rgb(status_color(incident.status))).child(
                                    format!("{} / {}", incident.time_label, incident.system),
                                ))
                                .child(
                                    div()
                                        .text_color(rgb(TEXT_DIM))
                                        .child(format!("#{}", incident.incident_id)),
                                ),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .child(incident.summary),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_xs()
                                .text_color(rgb(TEXT_MUTED))
                                .child(incident.recovery),
                        ),
                );
        }
        div()
            .flex_1()
            .min_h(px(250.0))
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header(
                "INCIDENT TIMELINE",
                "every transition is a receipt",
            ))
            .child(rows)
    }

    fn render_system_truth_panel(&self) -> impl IntoElement {
        let health = &self.snapshot.health;
        div()
            .flex_shrink_0()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(SURFACE_ALT))
            .child(panel_header(
                "LIVE ELIGIBILITY TRUTH",
                "health is not authorization",
            ))
            .child(
                div()
                    .p_3()
                    .child(truth_row(
                        "Massive feed",
                        format!("{} ms", health.massive_latency_ms),
                        GateStatus::Pass,
                    ))
                    .child(truth_row(
                        "Nautilus reconciliation",
                        "epoch 9",
                        GateStatus::Pass,
                    ))
                    .child(truth_row(
                        "TradeLocker SyncEnd",
                        "received",
                        GateStatus::Pass,
                    ))
                    .child(truth_row(
                        "Unknown outcomes",
                        health.unknown_orders.to_string(),
                        GateStatus::Pass,
                    ))
                    .child(truth_row(
                        "Instrument contracts",
                        "6 / 6 agree",
                        GateStatus::Pass,
                    ))
                    .child(
                        div()
                            .mt_3()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x665033))
                            .bg(rgb(0x2a2117))
                            .text_sm()
                            .text_color(rgb(WARNING))
                            .child("Eligible to arm only / simulation remains active"),
                    ),
            )
    }

    fn render_strategy_runtime(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let strategy = &self.snapshot.strategies[self.selected_strategy];
        let index = self.runtime.index_snapshot(strategy.index);
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .mb_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child("Decision evidence, divergence, and promotion control."),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(0x121615))
                            .text_xs()
                            .child("revision 8f13a2"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .grid()
                    .grid_cols(if compact { 2 } else { 9 })
                    .gap_3()
                    .child(self.render_system_list(compact, cx))
                    .child(
                        div()
                            .col_span(if compact { 1 } else { 5 })
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .overflow_hidden()
                            .bg(rgb(0x141817))
                            .child(panel_header(
                                format!("DECISION TRACE / {}", strategy.name.to_uppercase()),
                                "last 60 minutes",
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .relative()
                                    .child(index_chart(
                                        index.bars.clone(),
                                        ChartViewport::default(),
                                        None,
                                        true,
                                    ))
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(16.0))
                                            .left(px(18.0))
                                            .text_xs()
                                            .text_color(rgb(TEXT_MUTED))
                                            .child(format!(
                                                "{} / confidence 0.62 / shadow agreement 98.7%",
                                                strategy.disposition
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .p_3()
                                    .border_t_1()
                                    .border_color(rgb(BORDER_DIM))
                                    .bg(rgb(SURFACE))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::BOLD)
                                            .child("Current decision / HOLD LONG"),
                                    )
                                    .child(
                                        div()
                                            .mt_1()
                                            .text_sm()
                                            .text_color(rgb(TEXT_MUTED))
                                            .child("Trend persistence and breadth are supportive; expansion is blocked above 0.75R portfolio heat."),
                                    ),
                            ),
                    )
                    .child(self.render_promotion_gates(compact)),
            )
    }

    fn render_system_list(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div().p_2();
        for (slot, strategy) in self.snapshot.strategies.iter().enumerate() {
            let selected = self.selected_strategy == slot;
            list = list.child(
                div()
                    .id(("strategy", slot))
                    .mb_2()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(if selected { 0x3aa789 } else { SURFACE_ALT }))
                    .bg(rgb(if selected {
                        SURFACE_ACTIVE
                    } else {
                        SURFACE_ALT
                    }))
                    .cursor_pointer()
                    .hover(|item| item.bg(rgb(0x202825)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.selected_strategy != slot {
                            this.selected_strategy = slot;
                            cx.notify();
                        }
                    }))
                    .child(div().font_weight(FontWeight::BOLD).child(strategy.name))
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(if selected { ACCENT } else { TEXT_MUTED }))
                            .child(format!(
                                "{} / {}",
                                strategy.index.symbol(),
                                strategy.disposition.to_lowercase()
                            )),
                    ),
            );
        }
        div()
            .col_span(if compact { 1 } else { 2 })
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header("SYSTEMS", "3 loaded"))
            .child(list)
    }

    fn render_promotion_gates(&self, _compact: bool) -> impl IntoElement {
        let strategy = &self.snapshot.strategies[self.selected_strategy];
        let pass_count = strategy
            .gates
            .iter()
            .filter(|gate| gate.status == crate::contracts::GateStatus::Pass)
            .count();
        let mut gates = div().p_3();
        for gate in &strategy.gates {
            gates = gates.child(
                div()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_sm()
                            .child(div().text_color(rgb(TEXT_MUTED)).child(gate.label))
                            .child(
                                div()
                                    .text_color(rgb(status_color(gate.status)))
                                    .child(status_label(gate.status)),
                            ),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child(gate.detail),
                    ),
            );
        }
        div()
            .col_span(2)
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header(
                "PROMOTION GATES",
                format!("{pass_count} / {}", strategy.gates.len()),
            ))
            .child(gates)
            .child(
                div()
                    .mx_3()
                    .mb_3()
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(0x4b4230))
                    .bg(rgb(0x211d16))
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(WARNING))
                            .child("Promotion unavailable"),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child("Insufficient live-like fills and session coverage."),
                    )
                    .child(
                        Button::new("open-evidence")
                            .label("Open evidence receipt")
                            .small()
                            .warning()
                            .mt_3()
                            .w_full(),
                    ),
            )
    }
}

fn operation_metric(
    label: &'static str,
    value: impl Into<gpui::SharedString>,
    detail: impl Into<gpui::SharedString>,
    color: u32,
) -> impl IntoElement {
    div()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE_ALT))
        .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(label))
        .child(
            div()
                .mt_1()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(color))
                .child(value.into()),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(detail.into()),
        )
}

fn operation_group_label(group: OperationGroup) -> &'static str {
    match group {
        OperationGroup::Data => "DATA",
        OperationGroup::Engine => "ENGINE",
        OperationGroup::Venue => "VENUE",
        OperationGroup::Machine => "MACHINE",
    }
}

fn truth_row(
    label: &'static str,
    value: impl Into<gpui::SharedString>,
    status: crate::contracts::GateStatus,
) -> impl IntoElement {
    div()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER_DIM))
        .flex()
        .items_center()
        .justify_between()
        .text_sm()
        .child(div().text_color(rgb(TEXT_MUTED)).child(label))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(value.into())
                .child(div().size_2().rounded_full().bg(rgb(status_color(status)))),
        )
}
