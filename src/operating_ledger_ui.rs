use super::{OperatingApp, OperatorComposerAction};
use crate::data_plane::ids::{instruments, InstrumentId};
use crate::ledger::{
    ActorKind, CorrelationId, LedgerEntryId, LedgerEventKind, LedgerEventStatus, SourceEventKey,
    TradeCaseId,
};
use crate::operating::{
    Availability, LedgerCommandPort, OperatorEntryKind, OperatorLedgerCommand, OperatorMutation,
};
use crate::ui_theme::*;
use gpui::{div, prelude::*, px, rgb, Context, FontWeight, IntoElement, SharedString, Window};
use gpui_component::input::Input;
use gpui_component::scroll::ScrollableElement;
use std::sync::Arc;

impl OperatingApp {
    pub(super) fn render_ledger_page(
        &self,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ledger = &self.snapshot.ledger;
        let decisions = ledger
            .rows
            .iter()
            .filter(|row| row.kind == LedgerEventKind::Decision)
            .count() as u64;
        let routed = ledger
            .rows
            .iter()
            .filter(|row| row.kind == LedgerEventKind::Order)
            .count() as u64;
        let blocked = ledger
            .rows
            .iter()
            .filter(|row| row.status == LedgerEventStatus::Rejected)
            .count() as u64;
        let fills = ledger
            .rows
            .iter()
            .filter(|row| row.kind == LedgerEventKind::Fill)
            .count() as u64;
        let unknown = ledger
            .rows
            .iter()
            .filter(|row| row.kind.requires_trade_case() && row.case_id == TradeCaseId::UNKNOWN)
            .count() as u64;
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
                    .child(
                        div()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Causal Ledger"),
                            )
                            .child(div().mt_1().text_sm().text_color(rgb(TEXT_MUTED)).child(
                                "One causal history for machine evidence and operator memory.",
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .text_xs()
                                    .text_color(rgb(ACCENT))
                                    .child("IMMUTABLE / SCHEMA V2"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(availability_color(ledger.availability)))
                                    .child(ledger.health.to_string()),
                            ),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(if compact { 3 } else { 6 })
                    .gap_2()
                    .child(metric("DECISIONS", decisions, LAVENDER))
                    .child(metric("ROUTED", routed, ACCENT))
                    .child(metric("BLOCKED", blocked, WARNING))
                    .child(metric("FILLS", fills, ACCENT))
                    .child(metric(
                        "UNKNOWN",
                        unknown,
                        if unknown == 0 { ACCENT } else { DANGER },
                    ))
                    .child(metric(
                        "INCIDENTS",
                        ledger.incident_count,
                        if ledger.incident_count == 0 {
                            TEXT_MUTED
                        } else {
                            DANGER
                        },
                    )),
            )
            .child(ledger_tabs())
            .child(
                div()
                    .h_0()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap_3()
                    .child(self.render_timeline(cx))
                    .child(self.render_case_inspector(cx)),
            )
    }

    fn render_timeline(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div()
            .flex_1()
            .min_h_0()
            .overflow_y_scrollbar()
            .p_2()
            .flex()
            .flex_col()
            .gap_1();
        let mut visible = 0usize;
        for row in self.snapshot.ledger.rows.iter() {
            visible += 1;
            let entry_id = row.entry_id;
            let case_id = row.case_id;
            let selected = self.selected_ledger_entry == Some(entry_id);
            body = body.child(
                timeline_row(row, selected)
                    .id(("ledger-entry", entry_id.get()))
                    .cursor_pointer()
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if app.selected_ledger_entry != Some(entry_id) {
                            app.selected_ledger_entry = Some(entry_id);
                            if case_id != TradeCaseId::UNKNOWN {
                                app.selected_ledger_case = Some(case_id);
                            }
                            cx.notify();
                        }
                    })),
            );
        }
        if visible == 0 {
            body = body.child(empty_copy(
                "No committed events",
                "The timeline changes only after the durable append completes.",
            ));
        }
        surface(
            "LIFECYCLE RECEIPTS",
            "newest committed evidence first",
            body,
        )
        .h_full()
        .flex_1()
        .min_w_0()
    }

    fn render_case_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_ledger_case.and_then(|case_id| {
            self.snapshot
                .ledger
                .cases
                .iter()
                .find(|case| case.case_id == case_id)
        });
        let case_label = selected.map_or_else(
            || "NEW MANUAL CASE".to_string(),
            |case| format!("CASE {:04}", case.case_id.get()),
        );
        let selected_row = self.selected_ledger_entry.and_then(|entry_id| {
            self.snapshot
                .ledger
                .rows
                .iter()
                .find(|row| row.entry_id == entry_id)
        });
        let target_is_operator =
            selected_row.is_some_and(|row| row.actor_kind == ActorKind::Operator);
        let mutation_ready = self.ledger_composer_action == OperatorComposerAction::Append
            || (self.selected_ledger_case.is_some() && target_is_operator);
        let submit_enabled =
            self.snapshot.ledger.availability == Availability::Ready && mutation_ready;
        let input = Input::new(&self.ledger_note_input);
        let status = self
            .ledger_status
            .clone()
            .unwrap_or_else(|| "Drafts stay local until Publish is pressed.".into());
        let mut kind_picker = div().flex().items_center().gap_1();
        for kind in OperatorEntryKind::ALL {
            let active = kind == self.ledger_entry_kind;
            kind_picker = kind_picker.child(
                div()
                    .id(("ledger-entry-kind", u32::from(kind.key_byte())))
                    .flex_1()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(if active { 0x426d61 } else { BORDER_DIM }))
                    .bg(rgb(if active { SURFACE_ACTIVE } else { SURFACE_ALT }))
                    .text_xs()
                    .text_center()
                    .text_color(rgb(if active { ACCENT } else { TEXT_MUTED }))
                    .cursor_pointer()
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if app.ledger_entry_kind != kind {
                            app.ledger_entry_kind = kind;
                            cx.notify();
                        }
                    }))
                    .child(kind.label()),
            );
        }
        let mut action_picker = div().flex().items_center().gap_1();
        for action in OperatorComposerAction::ALL {
            let active = action == self.ledger_composer_action;
            let available = action == OperatorComposerAction::Append || target_is_operator;
            action_picker = action_picker.child(
                div()
                    .id(("ledger-action", action as u32))
                    .flex_1()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(if active { 0x426d61 } else { BORDER_DIM }))
                    .bg(rgb(if active { SURFACE_ACTIVE } else { SURFACE_ALT }))
                    .text_xs()
                    .text_center()
                    .text_color(rgb(if active {
                        ACCENT
                    } else if available {
                        TEXT_MUTED
                    } else {
                        TEXT_DIM
                    }))
                    .cursor_pointer()
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if action != OperatorComposerAction::Append {
                            let can_target = app.selected_ledger_entry.is_some_and(|entry_id| {
                                app.snapshot.ledger.rows.iter().any(|row| {
                                    row.entry_id == entry_id
                                        && row.actor_kind == ActorKind::Operator
                                })
                            });
                            if !can_target {
                                app.ledger_status =
                                    Some("Select an operator-authored entry first".into());
                                cx.notify();
                                return;
                            }
                        }
                        if app.ledger_composer_action != action {
                            app.ledger_composer_action = action;
                            cx.notify();
                        }
                    }))
                    .child(action.label()),
            );
        }
        let target = selected_row.map_or_else(
            || "No entry selected".to_string(),
            |row| {
                format!(
                    "Target ENTRY {:06} / {} / {}",
                    row.entry_id.get(),
                    row.kind.label(),
                    if row.actor_kind == ActorKind::Operator {
                        "operator"
                    } else {
                        "machine immutable"
                    }
                )
            },
        );
        let publish_label = match self.ledger_composer_action {
            OperatorComposerAction::Append => {
                format!("Append {}", self.ledger_entry_kind.label())
            }
            OperatorComposerAction::Amend => "Append amendment".to_string(),
            OperatorComposerAction::Redact => "Append redaction".to_string(),
        };
        surface(
            "CASE INSPECTOR",
            "append-only operator memory",
            div()
                .p_3()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(TEXT))
                                .child(case_label),
                        )
                        .child(
                            div()
                                .id("ledger-new-case")
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(BORDER))
                                .text_xs()
                                .text_color(rgb(TEXT_MUTED))
                                .cursor_pointer()
                                .on_click(cx.listener(|app, _, _, cx| {
                                    app.selected_ledger_case = None;
                                    app.selected_ledger_entry = None;
                                    app.ledger_entry_kind = OperatorEntryKind::Plan;
                                    app.ledger_composer_action = OperatorComposerAction::Append;
                                    app.ledger_status = Some("New case selected".into());
                                    cx.notify();
                                }))
                                .child("New case"),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(TEXT_DIM))
                        .child("ENTRY TYPE"),
                )
                .child(kind_picker)
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(TEXT_DIM))
                        .child("ACTION"),
                )
                .child(action_picker)
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(if target_is_operator { TEXT_MUTED } else { TEXT_DIM }))
                        .child(target),
                )
                .child(
                    div()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(BORDER_DIM))
                        .bg(rgb(CANVAS))
                        .child(input),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(TEXT_MUTED))
                        .child(status),
                )
                .child(
                    div()
                        .id("ledger-publish-note")
                        .w_full()
                        .py_2()
                        .rounded_md()
                        .bg(rgb(if submit_enabled { ACCENT } else { BORDER_DIM }))
                        .text_color(rgb(if submit_enabled { CANVAS } else { TEXT_DIM }))
                        .font_weight(FontWeight::BOLD)
                        .text_center()
                        .cursor_pointer()
                        .on_click(cx.listener(|app, _, window, cx| {
                            app.publish_ledger_entry(window, cx);
                        }))
                        .child(publish_label),
                )
                .child(
                    div()
                        .mt_2()
                        .pt_3()
                        .border_t_1()
                        .border_color(rgb(BORDER_DIM))
                        .text_xs()
                        .text_color(rgb(TEXT_DIM))
                        .child("Machine evidence cannot be edited. Operator amendments and redactions append linked receipts; history remains intact."),
                ),
        )
        .w(px(388.0))
        .flex_shrink_0()
    }

    fn publish_ledger_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let body = self.ledger_note_input.read(cx).value().trim().to_string();
        if body.is_empty() {
            self.ledger_status = Some("Write an operator entry before publishing".into());
            cx.notify();
            return;
        }
        if self.ledger_entry_kind.requires_existing_case() && self.selected_ledger_case.is_none() {
            self.ledger_status = Some("Interventions and reviews require an existing case".into());
            cx.notify();
            return;
        }
        let parent = self.selected_ledger_entry.unwrap_or(LedgerEntryId::UNKNOWN);
        let mutation = match self.ledger_composer_action {
            OperatorComposerAction::Append => OperatorMutation::Append,
            OperatorComposerAction::Amend if parent != LedgerEntryId::UNKNOWN => {
                OperatorMutation::Amend(parent)
            }
            OperatorComposerAction::Redact if parent != LedgerEntryId::UNKNOWN => {
                OperatorMutation::Redact(parent)
            }
            OperatorComposerAction::Amend | OperatorComposerAction::Redact => {
                self.ledger_status = Some("Select an operator entry to mutate".into());
                cx.notify();
                return;
            }
        };
        let submitted_ns = now_ns();
        self.ledger_submit_nonce = self.ledger_submit_nonce.saturating_add(1);
        let command_key = command_key(
            submitted_ns,
            self.ledger_submit_nonce,
            self.ledger_entry_kind,
            mutation,
            body.as_bytes(),
        );
        let case_id = self.selected_ledger_case.unwrap_or(TradeCaseId::UNKNOWN);
        let instrument_id = self
            .selected_ledger_case
            .and_then(|case_id| {
                self.snapshot
                    .ledger
                    .cases
                    .iter()
                    .find(|case| case.case_id == case_id)
                    .map(|case| case.instrument_id)
            })
            .or(self.selected_instrument)
            .unwrap_or(InstrumentId::UNKNOWN);
        match self.runtime.submit_operator_event(OperatorLedgerCommand {
            command_key,
            case_id,
            correlation_id: CorrelationId::UNKNOWN,
            instrument_id,
            submitted_ns,
            entry_kind: self.ledger_entry_kind,
            mutation,
            body: Arc::from(body),
        }) {
            Ok(()) => {
                self.ledger_note_input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                self.ledger_status = Some(SharedString::from(format!(
                    "Queued {} for durable commit",
                    mutation.label()
                )));
                self.ledger_composer_action = OperatorComposerAction::Append;
            }
            Err(error) => self.ledger_status = Some(SharedString::from(error.to_string())),
        }
        cx.notify();
    }
}

fn ledger_tabs() -> gpui::Div {
    let mut tabs = div()
        .flex()
        .items_center()
        .gap_5()
        .border_b_1()
        .border_color(rgb(BORDER_DIM));
    for (index, label) in [
        "Lifecycle",
        "Decisions",
        "Orders",
        "Fills",
        "Positions",
        "Reconciliation",
        "Incidents",
    ]
    .into_iter()
    .enumerate()
    {
        tabs = tabs.child(
            div()
                .px_2()
                .py_2()
                .border_b_2()
                .border_color(rgb(if index == 0 { ACCENT } else { CANVAS }))
                .text_sm()
                .text_color(rgb(if index == 0 { TEXT } else { TEXT_MUTED }))
                .child(label),
        );
    }
    tabs.child(
        div()
            .ml_auto()
            .text_xs()
            .text_color(rgb(TEXT_DIM))
            .child("J/K navigate / Enter inspect / receipt IDs searchable"),
    )
}

fn surface(title: &'static str, subtitle: &'static str, body: impl IntoElement) -> gpui::Div {
    div()
        .h_full()
        .min_h_0()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(48.0))
                .px_3()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(rgb(BORDER_DIM))
                .child(div().text_sm().font_weight(FontWeight::BOLD).child(title))
                .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(subtitle)),
        )
        .child(body)
}

fn metric(label: &'static str, value: u64, color: u32) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER_DIM))
        .bg(rgb(SURFACE))
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

fn timeline_row(row: &crate::operating::LedgerRowSnapshot, selected: bool) -> gpui::Div {
    let color = status_color(row.status);
    div()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(if selected { 0x426d61 } else { BORDER_DIM }))
        .bg(rgb(if selected {
            SURFACE_ACTIVE
        } else {
            SURFACE_ALT
        }))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .bg(rgb(SURFACE_ACTIVE))
                                .text_xs()
                                .text_color(rgb(color))
                                .child(row.kind.label()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(TEXT_DIM))
                                .child(format!("ENTRY {:06}", row.entry_id.get())),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(TEXT_DIM))
                        .child(format_utc(row.ts_event_ns)),
                ),
        )
        .child(
            div()
                .mt_2()
                .text_sm()
                .text_color(rgb(TEXT))
                .child(row.preview.to_string()),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .items_center()
                .justify_between()
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(instrument_label(row.instrument_id))
                .child(row.status.label()),
        )
}

fn empty_copy(title: &'static str, detail: &'static str) -> impl IntoElement {
    div()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER_DIM))
        .child(div().font_weight(FontWeight::BOLD).child(title))
        .child(
            div()
                .mt_2()
                .text_sm()
                .text_color(rgb(TEXT_MUTED))
                .child(detail),
        )
}

const fn status_color(status: LedgerEventStatus) -> u32 {
    match status {
        LedgerEventStatus::Accepted | LedgerEventStatus::Observed => ACCENT,
        LedgerEventStatus::Pending | LedgerEventStatus::Superseded => WARNING,
        LedgerEventStatus::Rejected | LedgerEventStatus::Incident => DANGER,
        LedgerEventStatus::Redacted => TEXT_DIM,
    }
}

const fn availability_color(availability: Availability) -> u32 {
    match availability {
        Availability::Ready | Availability::Live => ACCENT,
        Availability::Replaying | Availability::Delayed => LAVENDER,
        Availability::AwaitingFirstReceipt | Availability::Stale => WARNING,
        Availability::NotConfigured | Availability::Deferred => TEXT_DIM,
        Availability::NotEntitled | Availability::Disconnected | Availability::Invalid => DANGER,
    }
}

const fn instrument_label(instrument: InstrumentId) -> &'static str {
    match instrument {
        instruments::US100 => "US100",
        instruments::US500 => "US500",
        instruments::US30 => "US30",
        instruments::DE40 => "DE40",
        instruments::UK100 => "UK100",
        instruments::JP225 => "JP225",
        _ => "UNASSIGNED",
    }
}

fn command_key(
    timestamp_ns: i64,
    nonce: u64,
    kind: OperatorEntryKind,
    mutation: OperatorMutation,
    body: &[u8],
) -> SourceEventKey {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-operator-entry-v2");
    hasher.update(&timestamp_ns.to_le_bytes());
    hasher.update(&nonce.to_le_bytes());
    hasher.update(&[kind.key_byte(), mutation.key_byte()]);
    hasher.update(&mutation.parent().get().to_le_bytes());
    hasher.update(body);
    let hash = hasher.finalize();
    let mut key = [0u8; 16];
    key.copy_from_slice(&hash.as_bytes()[..16]);
    let value = u128::from_le_bytes(key);
    SourceEventKey(if value == 0 { 1 } else { value })
}

fn now_ns() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

fn format_utc(timestamp_ns: i64) -> String {
    let seconds = timestamp_ns.div_euclid(1_000_000_000);
    let day = seconds.div_euclid(86_400);
    let second_of_day = seconds.rem_euclid(86_400);
    let (year, month, date) = civil_from_days(day);
    let hour = second_of_day / 3_600;
    let minute = second_of_day % 3_600 / 60;
    let second = second_of_day % 60;
    format!("{year:04}-{month:02}-{date:02} {hour:02}:{minute:02}:{second:02} UTC")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_formatting_is_stable_without_locale_state() {
        assert_eq!(format_utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(
            format_utc(1_788_976_800_000_000_000),
            "2026-09-09 18:00:00 UTC"
        );
    }
}
