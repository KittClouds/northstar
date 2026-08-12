use super::{
    panel_header, status_color, status_label, LedgerView, NorthstarApp, ACCENT, BORDER, BORDER_DIM,
    DANGER, LAVENDER, SURFACE_ACTIVE, SURFACE_ALT, TEXT, TEXT_DIM, TEXT_MUTED, WARNING,
};
use crate::office::{LifecycleReceipt, ReceiptKind};
use gpui::{div, prelude::*, rgb, Context, FontWeight, IntoElement};

impl LedgerView {
    const ALL: [Self; 7] = [
        Self::Lifecycle,
        Self::Decisions,
        Self::Orders,
        Self::Fills,
        Self::Positions,
        Self::Reconciliation,
        Self::Incidents,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Lifecycle => "Lifecycle",
            Self::Decisions => "Decisions",
            Self::Orders => "Orders",
            Self::Fills => "Fills",
            Self::Positions => "Positions",
            Self::Reconciliation => "Reconciliation",
            Self::Incidents => "Incidents",
        }
    }

    fn accepts(self, kind: ReceiptKind) -> bool {
        match self {
            Self::Lifecycle => true,
            Self::Decisions => matches!(kind, ReceiptKind::Decision | ReceiptKind::RiskEvaluation),
            Self::Orders => matches!(kind, ReceiptKind::CommandIntent | ReceiptKind::VenueOrder),
            Self::Fills => kind == ReceiptKind::Fill,
            Self::Positions => matches!(kind, ReceiptKind::Position | ReceiptKind::Exit),
            Self::Reconciliation => kind == ReceiptKind::Reconciliation,
            Self::Incidents => matches!(kind, ReceiptKind::Incident | ReceiptKind::OperatorAction),
        }
    }
}

impl NorthstarApp {
    pub(super) fn render_lifecycle_ledger(
        &self,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ledger = &self.snapshot.lifecycle;
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
                                    .child("Causal Ledger"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Observation to decision, risk, venue truth, position, and reconciliation."),
                            ),
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
                            .text_color(rgb(ACCENT))
                            .child("IMMUTABLE / SCHEMA V2"),
                    ),
            )
            .child(self.render_ledger_summary())
            .child(self.render_ledger_tabs(cx))
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
                            .col_span(if compact { 1 } else { 8 })
                            .min_w_0()
                            .min_h_0()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .overflow_hidden()
                            .bg(rgb(0x141817))
                            .child(panel_header(
                                format!("{} RECEIPTS", self.ledger_view.label().to_uppercase()),
                                format!("{} total / newest first", ledger.receipts.len()),
                            ))
                            .child(self.render_receipt_table(cx)),
                    )
                    .child(
                        div()
                            .col_span(if compact { 1 } else { 4 })
                            .min_w_0()
                            .min_h_0()
                            .child(self.render_receipt_inspector()),
                    ),
            )
    }

    fn render_ledger_summary(&self) -> impl IntoElement {
        let ledger = &self.snapshot.lifecycle;
        div()
            .grid()
            .grid_cols(6)
            .gap_2()
            .child(ledger_metric("DECISIONS", ledger.decisions, LAVENDER))
            .child(ledger_metric("ROUTED", ledger.routed, ACCENT))
            .child(ledger_metric("BLOCKED", ledger.blocked, WARNING))
            .child(ledger_metric("FILLS", ledger.fills, ACCENT))
            .child(ledger_metric(
                "UNKNOWN",
                ledger.unknown,
                if ledger.unknown == 0 { ACCENT } else { DANGER },
            ))
            .child(ledger_metric("INCIDENTS", ledger.incidents, WARNING))
    }

    fn render_ledger_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tabs = div()
            .mt_3()
            .flex()
            .items_center()
            .gap_1()
            .border_b_1()
            .border_color(rgb(BORDER));
        for view in LedgerView::ALL {
            let selected = self.ledger_view == view;
            tabs = tabs.child(
                div()
                    .id(("ledger-view", view as usize))
                    .px_3()
                    .py_2()
                    .border_b_2()
                    .border_color(rgb(if selected { ACCENT } else { BORDER_DIM }))
                    .text_sm()
                    .text_color(rgb(if selected { TEXT } else { TEXT_MUTED }))
                    .cursor_pointer()
                    .hover(|item| item.bg(rgb(0x202624)).text_color(rgb(TEXT)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.ledger_view != view {
                            this.ledger_view = view;
                            if let Some(slot) = this
                                .snapshot
                                .lifecycle
                                .receipts
                                .iter()
                                .position(|receipt| view.accepts(receipt.kind))
                            {
                                this.selected_receipt = slot;
                            }
                            cx.notify();
                        }
                    }))
                    .child(view.label()),
            );
        }
        tabs.child(
            div()
                .ml_auto()
                .pb_2()
                .text_xs()
                .text_color(rgb(TEXT_DIM))
                .child("J/K navigate / Enter inspect / receipt IDs searchable"),
        )
    }

    fn render_receipt_table(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut table = div().flex_1().min_h_0();
        table = table.child(
            div()
                .grid()
                .grid_cols(10)
                .gap_2()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(rgb(BORDER_DIM))
                .text_xs()
                .text_color(rgb(TEXT_DIM))
                .child("TIME")
                .child("INDEX")
                .child("TYPE")
                .child(div().col_span(2).child("ORIGIN"))
                .child(div().col_span(2).child("EVENT / DECISION"))
                .child("RISK")
                .child("STATUS")
                .child("ID"),
        );

        for (slot, receipt) in self.snapshot.lifecycle.receipts.iter().enumerate() {
            if !self.ledger_view.accepts(receipt.kind) {
                continue;
            }
            let selected = self.selected_receipt == slot;
            table = table.child(
                div()
                    .id(("receipt-row", slot))
                    .grid()
                    .grid_cols(10)
                    .gap_2()
                    .items_center()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .bg(rgb(if selected { SURFACE_ACTIVE } else { 0x141817 }))
                    .cursor_pointer()
                    .hover(|item| item.bg(rgb(0x202724)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.selected_receipt != slot {
                            this.selected_receipt = slot;
                            cx.notify();
                        }
                    }))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(receipt.time_label),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child(receipt.index.map_or("—", |index| index.symbol())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(receipt_kind_color(receipt.kind)))
                            .child(receipt.kind.label()),
                    )
                    .child(div().col_span(2).text_sm().child(receipt.origin))
                    .child(div().col_span(2).text_sm().child(receipt.summary))
                    .child(
                        div().text_sm().child(
                            receipt
                                .risk_r
                                .map_or("—".to_string(), |risk| format!("{risk:.2}R")),
                        ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(status_color(receipt.status)))
                            .child(status_label(receipt.status)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(if selected { ACCENT } else { TEXT_DIM }))
                            .child(format!("#{}", receipt.receipt_id)),
                    ),
            );
        }
        table
    }

    fn render_receipt_inspector(&self) -> impl IntoElement {
        let receipt = &self.snapshot.lifecycle.receipts[self.selected_receipt];
        let parent = receipt.parent_id.and_then(|parent_id| {
            self.snapshot
                .lifecycle
                .receipts
                .iter()
                .find(|candidate| candidate.receipt_id == parent_id)
        });
        let mut children = self
            .snapshot
            .lifecycle
            .receipts
            .iter()
            .filter(|candidate| candidate.parent_id == Some(receipt.receipt_id));
        let first_child = children.next();

        div()
            .h_full()
            .min_h_0()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(SURFACE_ALT))
            .child(panel_header(
                format!("RECEIPT #{}", receipt.receipt_id),
                receipt.kind.label(),
            ))
            .child(
                div()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::BOLD)
                                    .child(receipt.summary),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x202724))
                                    .text_xs()
                                    .text_color(rgb(status_color(receipt.status)))
                                    .child(status_label(receipt.status)),
                            ),
                    )
                    .child(inspector_section("EVIDENCE", receipt.evidence))
                    .child(inspector_section("VENUE TRUTH", receipt.venue_truth))
                    .child(inspector_section(
                        "LATENCY",
                        format!("{} µs observed-to-receipt", receipt.latency_us),
                    ))
                    .child(inspector_section("PROVENANCE", receipt.provenance))
                    .child(
                        div()
                            .mt_4()
                            .pt_3()
                            .border_t_1()
                            .border_color(rgb(BORDER_DIM))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_DIM))
                                    .child("CAUSAL NEIGHBORS"),
                            )
                            .child(causal_neighbor("Parent", parent))
                            .child(causal_neighbor("Child", first_child)),
                    ),
            )
    }
}

fn ledger_metric(label: &'static str, value: u16, color: u32) -> impl IntoElement {
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
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(color))
                .child(value.to_string()),
        )
}

fn receipt_kind_color(kind: ReceiptKind) -> u32 {
    match kind {
        ReceiptKind::Observation => LAVENDER,
        ReceiptKind::Decision | ReceiptKind::RiskEvaluation => WARNING,
        ReceiptKind::CommandIntent
        | ReceiptKind::VenueOrder
        | ReceiptKind::Fill
        | ReceiptKind::Position
        | ReceiptKind::Exit
        | ReceiptKind::Reconciliation => ACCENT,
        ReceiptKind::Incident => DANGER,
        ReceiptKind::OperatorAction => TEXT,
    }
}

fn inspector_section(
    label: &'static str,
    value: impl Into<gpui::SharedString>,
) -> impl IntoElement {
    div()
        .mt_4()
        .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(label))
        .child(
            div()
                .mt_1()
                .text_sm()
                .text_color(rgb(TEXT_MUTED))
                .child(value.into()),
        )
}

fn causal_neighbor(label: &'static str, receipt: Option<&LifecycleReceipt>) -> impl IntoElement {
    div()
        .mt_2()
        .p_2()
        .rounded_md()
        .bg(rgb(0x141817))
        .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(label))
        .child(
            div()
                .mt_1()
                .text_sm()
                .child(receipt.map_or("none".to_string(), |item| {
                    format!(
                        "#{} / {} / {}",
                        item.receipt_id,
                        item.kind.label(),
                        item.summary
                    )
                })),
        )
}
