use super::macro_ui::{
    feed_snapshot, format_timestamp, format_value, missing_series_status, observation_table,
    provenance_panel, source_contract_panel,
};
use super::OperatingApp;
use crate::data_plane::event::ParticipantClass;
use crate::data_plane::ids::SourceId;
use crate::data_plane::provenance::ReleaseStatus;
use crate::data_plane::providers::bea::BEA_SOURCE;
use crate::data_plane::providers::bls::BLS_SOURCE;
use crate::data_plane::providers::bls_calendar::BLS_RELEASE_CALENDAR_STREAM;
use crate::data_plane::providers::boe::BOE_SOURCE;
use crate::data_plane::providers::boj::BOJ_SOURCE;
use crate::data_plane::providers::census::CENSUS_SOURCE;
use crate::data_plane::providers::ecb::ECB_SOURCE;
use crate::data_plane::providers::eurostat::EUROSTAT_SOURCE;
use crate::data_plane::providers::fred::FRED_SOURCE;
use crate::data_plane::providers::macro_catalog::MacroCategory;
use crate::data_plane::providers::ons::ONS_SOURCE;
use crate::operating::{Availability, MacroFeedState, MacroSeriesSnapshot, MacroSnapshot};
use crate::operating_ui_status::{availability_badge, availability_color, format_sequence};
use crate::ui_theme::*;
use gpui::{div, prelude::*, px, rgb, FontWeight, IntoElement};
use gpui_component::scroll::ScrollableElement;

const US_SOURCES: [SourceId; 4] = [BLS_SOURCE, FRED_SOURCE, BEA_SOURCE, CENSUS_SOURCE];
const EUROPE_SOURCES: [SourceId; 2] = [EUROSTAT_SOURCE, ECB_SOURCE];
const UK_SOURCES: [SourceId; 2] = [ONS_SOURCE, BOE_SOURCE];
const JAPAN_SOURCES: [SourceId; 1] = [BOJ_SOURCE];

impl OperatingApp {
    pub(super) fn render_macro_page(&self, compact: bool) -> impl IntoElement {
        let snapshot = &self.snapshot.macro_office;
        div()
            .w_full()
            .h_0()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(macro_header(snapshot))
            .child(situation_strip(snapshot, compact))
            .child(region_strip(snapshot, compact))
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
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(index_context(snapshot))
                            .child(observation_table(snapshot)),
                    )
                    .child(
                        div()
                            .col_span(if compact { 1 } else { 5 })
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(release_tape(snapshot))
                            .child(provenance_panel(snapshot))
                            .child(source_contract_panel(snapshot)),
                    ),
            )
    }
}

fn macro_header(snapshot: &MacroSnapshot) -> gpui::Div {
    div()
        .flex()
        .items_end()
        .justify_between()
        .gap_4()
        .child(
            div()
                .child(
                    div()
                        .text_xl()
                        .font_weight(FontWeight::BOLD)
                        .child("Global Macro"),
                )
                .child(div().mt_1().text_sm().text_color(rgb(TEXT_MUTED)).child(
                    "Official observations, revisions, positioning, and release provenance.",
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
                        .bg(rgb(0x121615))
                        .text_xs()
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!(
                            "GEN {} / {} / {}",
                            snapshot.generation,
                            format_sequence(snapshot.last_sequence),
                            if snapshot.as_of_ns == 0 {
                                "AWAITING CLOCK".to_string()
                            } else {
                                format_timestamp(snapshot.as_of_ns)
                            }
                        )),
                )
                .child(availability_badge(snapshot.availability)),
        )
}

fn situation_strip(snapshot: &MacroSnapshot, compact: bool) -> gpui::Div {
    let scheduled = snapshot
        .releases
        .iter()
        .filter(|release| release.status == ReleaseStatus::Scheduled)
        .min_by_key(|release| release.scheduled_ns);
    let current = snapshot
        .series
        .iter()
        .filter(|series| series.latest.is_some())
        .count();
    let revised = snapshot
        .series
        .iter()
        .filter_map(|series| series.latest)
        .filter(|point| point.corrected)
        .count();
    let calendar = feed_snapshot(snapshot, BLS_SOURCE, BLS_RELEASE_CALENDAR_STREAM);
    let (empty_release_value, empty_release_detail, empty_release_state) =
        match calendar.map(|feed| feed.state) {
            Some(MacroFeedState::Rejected) => (
                "calendar rejected".to_string(),
                calendar.map_or_else(|| "source rejected".into(), |feed| feed.detail.to_string()),
                Availability::Invalid,
            ),
            Some(MacroFeedState::NotConfigured) => (
                "calendar not configured".to_string(),
                "source activation required".to_string(),
                Availability::NotConfigured,
            ),
            Some(MacroFeedState::Current) => (
                "no upcoming release".to_string(),
                "official calendar current".to_string(),
                Availability::Ready,
            ),
            Some(MacroFeedState::Awaiting) | None => (
                "awaiting calendar".to_string(),
                "no canonical calendar receipt".to_string(),
                Availability::AwaitingFirstReceipt,
            ),
        };
    div()
        .grid()
        .grid_cols(if compact { 3 } else { 5 })
        .gap_2()
        .child(macro_metric(
            "NEXT OPERATING RELEASE",
            scheduled.map_or_else(
                || empty_release_value,
                |release| format!("release #{}", release.release_id.get()),
            ),
            scheduled.map_or_else(
                || empty_release_detail,
                |release| format_timestamp(release.scheduled_ns),
            ),
            if scheduled.is_some() {
                Availability::Ready
            } else {
                empty_release_state
            },
        ))
        .child(macro_metric(
            "PUBLICATION",
            format_sequence(snapshot.last_sequence),
            format!("{} canonical events", snapshot.event_count),
            snapshot.availability,
        ))
        .child(macro_metric(
            "CURRENT SERIES",
            format!("{current} / {}", snapshot.series.len()),
            "missing remains explicit".to_string(),
            snapshot.availability,
        ))
        .child(macro_metric(
            "REVISIONS",
            revised.to_string(),
            "latest corrected vintages".to_string(),
            if revised == 0 {
                Availability::Ready
            } else {
                Availability::Stale
            },
        ))
        .child(macro_metric(
            "MODE",
            "replay safe".to_string(),
            "observations only / no model".to_string(),
            Availability::Ready,
        ))
}

fn macro_metric(
    label: &'static str,
    value: String,
    detail: String,
    state: Availability,
) -> gpui::Div {
    div()
        .min_w_0()
        .overflow_hidden()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(label))
        .child(
            div()
                .mt_2()
                .text_base()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(availability_color(state)))
                .truncate()
                .child(value),
        )
        .child(
            div()
                .mt_2()
                .truncate()
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(detail),
        )
}

fn region_strip(snapshot: &MacroSnapshot, compact: bool) -> gpui::Div {
    div()
        .grid()
        .grid_cols(if compact { 2 } else { 4 })
        .gap_2()
        .child(region_card(
            "United States",
            "BLS / BEA / Census / FRED",
            &US_SOURCES,
            snapshot,
            true,
        ))
        .child(region_card(
            "Euro area / Germany",
            "Eurostat / ECB",
            &EUROPE_SOURCES,
            snapshot,
            false,
        ))
        .child(region_card(
            "United Kingdom",
            "ONS / Bank of England",
            &UK_SOURCES,
            snapshot,
            false,
        ))
        .child(region_card(
            "Japan",
            "Bank of Japan",
            &JAPAN_SOURCES,
            snapshot,
            false,
        ))
}

fn region_card(
    label: &'static str,
    source: &'static str,
    sources: &[SourceId],
    snapshot: &MacroSnapshot,
    selected: bool,
) -> gpui::Div {
    div()
        .min_w_0()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(if selected { ACCENT } else { BORDER }))
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
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(if selected { ACCENT } else { TEXT }))
                        .child(label),
                )
                .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(source)),
        )
        .child(
            div()
                .mt_3()
                .grid()
                .grid_cols(2)
                .gap_x_3()
                .gap_y_2()
                .child(region_value(
                    "Growth",
                    sources,
                    MacroCategory::Growth,
                    snapshot,
                ))
                .child(region_value(
                    "Inflation",
                    sources,
                    MacroCategory::Inflation,
                    snapshot,
                ))
                .child(region_value(
                    "Labor",
                    sources,
                    MacroCategory::Labor,
                    snapshot,
                ))
                .child(region_value(
                    "Policy / rates",
                    sources,
                    MacroCategory::Rates,
                    snapshot,
                )),
        )
}

fn region_value(
    label: &'static str,
    sources: &[SourceId],
    category: MacroCategory,
    snapshot: &MacroSnapshot,
) -> gpui::Div {
    let series = find_series(snapshot, sources, category);
    let (value, color) = series.and_then(|item| item.latest).map_or_else(
        || {
            let missing = missing_series_status(snapshot, series);
            (missing.label.to_ascii_lowercase(), missing.color)
        },
        |point| {
            (
                format_value(point.value, series.expect("series exists").unit),
                ACCENT,
            )
        },
    );
    div()
        .min_w_0()
        .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(label))
        .child(div().mt_1().text_xs().text_color(rgb(color)).child(value))
}

fn find_series<'a>(
    snapshot: &'a MacroSnapshot,
    sources: &[SourceId],
    category: MacroCategory,
) -> Option<&'a MacroSeriesSnapshot> {
    snapshot
        .series
        .iter()
        .filter(|series| sources.contains(&series.source) && series.category == category)
        .max_by_key(|series| {
            series
                .latest
                .map(|point| point.received_ns)
                .unwrap_or_default()
        })
}

fn index_context(snapshot: &MacroSnapshot) -> gpui::Div {
    let mut body = div().flex_1().min_h_0().overflow_y_scrollbar();
    body = body.child(
        div()
            .grid()
            .grid_cols(7)
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(BORDER_DIM))
            .text_xs()
            .text_color(rgb(TEXT_DIM))
            .child("INDEX")
            .child("GROWTH")
            .child("INFLATION")
            .child("LABOR")
            .child("RATES")
            .child("POSITIONING")
            .child("EVENT RISK"),
    );
    for (label, sources) in [
        ("US100", US_SOURCES.as_slice()),
        ("US500", US_SOURCES.as_slice()),
        ("US30", US_SOURCES.as_slice()),
        ("DE40", EUROPE_SOURCES.as_slice()),
        ("UK100", UK_SOURCES.as_slice()),
        ("JP225", JAPAN_SOURCES.as_slice()),
    ] {
        body = body.child(context_row(label, sources, snapshot));
    }
    surface("INDEX CONTEXT", "facts + transparent transforms", body).min_h(px(350.0))
}

fn context_row(label: &'static str, sources: &[SourceId], snapshot: &MacroSnapshot) -> gpui::Div {
    let positioning = snapshot
        .positioning
        .iter()
        .find(|market| market.label == label)
        .and_then(|market| market.participant(ParticipantClass::AssetManager))
        .and_then(|participant| participant.net_percentile)
        .map_or_else(
            || "unavailable".to_string(),
            |value| format!("{value:.0}th pct"),
        );
    let event_risk = snapshot
        .releases
        .iter()
        .filter(|release| release.status == ReleaseStatus::Scheduled)
        .count();
    div()
        .grid()
        .grid_cols(7)
        .gap_2()
        .items_center()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER_DIM))
        .child(div().font_weight(FontWeight::BOLD).child(label))
        .child(context_cell(sources, MacroCategory::Growth, snapshot))
        .child(context_cell(sources, MacroCategory::Inflation, snapshot))
        .child(context_cell(sources, MacroCategory::Labor, snapshot))
        .child(context_cell(sources, MacroCategory::Rates, snapshot))
        .child(div().text_xs().text_color(rgb(LAVENDER)).child(positioning))
        .child(
            div()
                .text_xs()
                .text_color(rgb(if event_risk == 0 { ACCENT } else { WARNING }))
                .child(if event_risk == 0 {
                    "clear".to_string()
                } else {
                    format!("{event_risk} scheduled")
                }),
        )
}

fn context_cell(
    sources: &[SourceId],
    category: MacroCategory,
    snapshot: &MacroSnapshot,
) -> gpui::Div {
    let series = find_series(snapshot, sources, category);
    let (value, detail, color) = series.and_then(|item| item.latest).map_or_else(
        || {
            let missing = missing_series_status(snapshot, series);
            (
                missing.label.to_ascii_lowercase(),
                missing.detail,
                missing.color,
            )
        },
        |point| {
            let item = series.expect("series exists");
            (
                format_value(point.value, item.unit),
                item.label.to_string(),
                ACCENT,
            )
        },
    );
    div()
        .min_w_0()
        .pl_2()
        .border_l_2()
        .border_color(rgb(color))
        .child(div().text_xs().text_color(rgb(color)).child(value))
        .child(
            div()
                .truncate()
                .text_xs()
                .text_color(rgb(TEXT_DIM))
                .child(detail),
        )
}

fn release_tape(snapshot: &MacroSnapshot) -> gpui::Div {
    let mut body = div().max_h(px(190.0)).overflow_y_scrollbar();
    if snapshot.releases.is_empty() {
        let message = feed_snapshot(snapshot, BLS_SOURCE, BLS_RELEASE_CALENDAR_STREAM).map_or(
            "Release calendar is not represented in this snapshot.".to_string(),
            |feed| match feed.state {
                MacroFeedState::Rejected => {
                    format!("Release calendar rejected: {}", feed.detail)
                }
                MacroFeedState::NotConfigured => {
                    "Release calendar source is not configured.".to_string()
                }
                MacroFeedState::Current => {
                    "Official calendar is current with no upcoming release.".to_string()
                }
                MacroFeedState::Awaiting => {
                    "Awaiting the first canonical release-calendar receipt.".to_string()
                }
            },
        );
        body = body.child(
            div()
                .p_4()
                .text_sm()
                .text_color(rgb(TEXT_MUTED))
                .child(message),
        );
    } else {
        for release in snapshot.releases.iter().rev() {
            let (status, color) = release_status(release.status);
            body = body.child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .text_xs()
                    .child(format!("#{:04}", release.release_id.get()))
                    .child(format_timestamp(release.scheduled_ns))
                    .child(format_sequence(release.sequence))
                    .child(div().text_color(rgb(color)).child(status)),
            );
        }
    }
    surface("RELEASE TAPE", "scheduled / actual / received", body)
}

fn release_status(status: ReleaseStatus) -> (&'static str, u32) {
    match status {
        ReleaseStatus::Scheduled => ("SCHEDULED", WARNING),
        ReleaseStatus::Published => ("PUBLISHED", ACCENT),
        ReleaseStatus::Rescheduled => ("RESCHEDULED", LAVENDER),
        ReleaseStatus::Cancelled => ("CANCELLED", DANGER),
    }
}

fn surface(label: &'static str, detail: &'static str, body: impl IntoElement) -> gpui::Div {
    div()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .overflow_hidden()
        .child(
            div()
                .h_10()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .border_b_1()
                .border_color(rgb(BORDER_DIM))
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(label)
                .child(div().text_color(rgb(TEXT_DIM)).child(detail)),
        )
        .child(body)
}
