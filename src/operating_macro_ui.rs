use crate::data_plane::event::{ParticipantClass, TimeQuality};
use crate::data_plane::ids::{SourceId, StreamId};
use crate::data_plane::providers::bea::{BEA_NIPA_STREAM, BEA_SOURCE};
use crate::data_plane::providers::bls::{BLS_SOURCE, BLS_TIMESERIES_STREAM};
use crate::data_plane::providers::boe::{BOE_RATES_STREAM, BOE_SOURCE};
use crate::data_plane::providers::boj::{BOJ_SOURCE, BOJ_TIMESERIES_STREAM};
use crate::data_plane::providers::census::{CENSUS_MARTS_STREAM, CENSUS_SOURCE};
use crate::data_plane::providers::cftc::{CFTC_SOURCE, CFTC_TFF_STREAM};
use crate::data_plane::providers::ecb::{ECB_POLICY_RATES_STREAM, ECB_SOURCE};
use crate::data_plane::providers::eurostat::{EUROSTAT_SOURCE, EUROSTAT_STATISTICS_STREAM};
use crate::data_plane::providers::fred::{FRED_OBSERVATIONS_STREAM, FRED_SOURCE};
use crate::data_plane::providers::macro_catalog::{MacroCategory, MacroUnit};
use crate::data_plane::providers::ons::{ONS_SOURCE, ONS_TIMESERIES_STREAM};
use crate::operating::{
    Availability, MacroFeedSnapshot, MacroFeedState, MacroSeriesSnapshot, MacroSnapshot,
};
use crate::operating_ui_status::{availability_color, format_sequence};
use crate::ui_theme::*;
use gpui::{div, prelude::*, rgb, FontWeight, IntoElement};
use gpui_component::scroll::ScrollableElement;

pub(super) fn observation_table(snapshot: &MacroSnapshot) -> gpui::Div {
    let mut body = div()
        .flex_1()
        .min_h_0()
        .overflow_y_scrollbar()
        .p_2()
        .flex()
        .flex_col()
        .gap_1();
    for series in snapshot.series.iter() {
        let (value, period, status, status_color) = series.latest.map_or_else(
            || {
                let missing = missing_series_status(snapshot, Some(series));
                ("—".into(), missing.detail, missing.label, missing.color)
            },
            |point| {
                (
                    format_value(point.value, series.unit),
                    format_period(point.period_start_ns),
                    if point.provisional { "PRELIM" } else { "FINAL" },
                    if point.provisional { WARNING } else { ACCENT },
                )
            },
        );
        body = body.child(
            div()
                .grid()
                .grid_cols(4)
                .gap_2()
                .px_2()
                .py_2()
                .rounded_md()
                .bg(rgb(SURFACE_ALT))
                .text_xs()
                .child(series.label)
                .child(div().text_color(rgb(TEXT_MUTED)).child(period))
                .child(div().text_color(rgb(TEXT)).child(value))
                .child(div().text_color(rgb(status_color)).child(status)),
        );
    }
    body = body.child(
        div()
            .mt_2()
            .px_2()
            .py_1()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT_MUTED))
            .child("CFTC TFF POSITIONING"),
    );
    for market in snapshot.positioning.iter() {
        body = body.child(positioning_row(market));
    }
    body = body
        .child(unsupported_positioning_row("DE40", "No CFTC TFF contract"))
        .child(unsupported_positioning_row("UK100", "No CFTC TFF contract"));
    macro_surface(
        "OBSERVATIONS + POSITIONING",
        "official values / no score",
        body,
    )
}

fn positioning_row(market: &crate::operating::PositioningMarketSnapshot) -> gpui::Div {
    let asset_managers = market.participant(ParticipantClass::AssetManager);
    let leveraged = market.participant(ParticipantClass::LeveragedFunds);
    let asset_net = asset_managers
        .and_then(|participant| participant.latest)
        .map_or_else(
            || "AM —".into(),
            |point| format!("AM {}", format_contracts(point.net())),
        );
    let leveraged_net = leveraged
        .and_then(|participant| participant.latest)
        .map_or_else(
            || "LF —".into(),
            |point| format!("LF {}", format_contracts(point.net())),
        );
    let percentile = asset_managers
        .and_then(|participant| participant.net_percentile)
        .map_or_else(|| "Pctl —".into(), |value| format!("Pctl {value:.0}"));
    let report = if market.latest_report_ns == 0 {
        "awaiting first report".into()
    } else {
        format_date(market.latest_report_ns)
    };
    div()
        .grid()
        .grid_cols(4)
        .gap_2()
        .px_2()
        .py_2()
        .rounded_md()
        .bg(rgb(SURFACE_ALT))
        .text_xs()
        .child(
            div()
                .child(market.label)
                .child(div().mt_1().text_color(rgb(TEXT_DIM)).child(report)),
        )
        .child(div().text_color(rgb(ACCENT)).child(asset_net))
        .child(div().text_color(rgb(LAVENDER)).child(leveraged_net))
        .child(div().text_color(rgb(TEXT_MUTED)).child(percentile))
}

fn unsupported_positioning_row(label: &'static str, detail: &'static str) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER_DIM))
        .text_xs()
        .child(label)
        .child(div().text_color(rgb(TEXT_DIM)).child(detail))
}

pub(super) fn provenance_panel(snapshot: &MacroSnapshot) -> gpui::Div {
    let latest = snapshot
        .series
        .iter()
        .filter_map(|series| series.latest)
        .max_by_key(|point| point.sequence);
    let body = div()
        .flex_1()
        .min_h_0()
        .overflow_y_scrollbar()
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(provenance_row("CANONICAL EVENTS", snapshot.event_count.to_string()))
        .child(provenance_row("RAW RECEIPTS", snapshot.receipt_count.to_string()))
        .child(provenance_row("JOURNAL", format_sequence(snapshot.last_sequence)))
        .child(provenance_row(
            "LAST RECEIVED",
            if snapshot.as_of_ns == 0 {
                "—".into()
            } else {
                format_timestamp(snapshot.as_of_ns)
            },
        ))
        .child(provenance_row(
            "TIME CONTRACT",
            latest.map_or_else(
                || "awaiting evidence".into(),
                |point| time_quality_label(point.time_quality).into(),
            ),
        ))
        .child(
            div()
                .mt_2()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(BORDER_DIM))
                .bg(rgb(CANVAS))
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child("Official U.S., European, and UK observations become effective only when Northstar receives them. Provider release dates and revision identities remain provenance evidence; they never backdate Northstar knowledge."),
        );
    macro_surface("PROVENANCE", "what Northstar knew and when", body)
}

pub(super) struct MissingSeriesStatus {
    pub label: &'static str,
    pub detail: String,
    pub color: u32,
}

pub(super) fn feed_snapshot(
    snapshot: &MacroSnapshot,
    source: SourceId,
    stream: StreamId,
) -> Option<&MacroFeedSnapshot> {
    snapshot
        .feeds
        .iter()
        .find(|feed| feed.source == source && feed.stream == stream)
}

pub(super) fn missing_series_status(
    snapshot: &MacroSnapshot,
    series: Option<&MacroSeriesSnapshot>,
) -> MissingSeriesStatus {
    let Some(series) = series else {
        return MissingSeriesStatus {
            label: "UNSUPPORTED",
            detail: "no canonical series in this catalog".into(),
            color: TEXT_DIM,
        };
    };
    match feed_snapshot(snapshot, series.source, series.stream).map(|feed| feed.state) {
        Some(MacroFeedState::NotConfigured) => MissingSeriesStatus {
            label: "NOT CONFIGURED",
            detail: "credential required".into(),
            color: WARNING,
        },
        Some(MacroFeedState::Rejected) => MissingSeriesStatus {
            label: "REJECTED",
            detail: feed_snapshot(snapshot, series.source, series.stream).map_or_else(
                || "canonical publication rejected".into(),
                |feed| feed.detail.to_string(),
            ),
            color: DANGER,
        },
        Some(MacroFeedState::Current) => MissingSeriesStatus {
            label: "NO VALUE",
            detail: "feed published without this observation".into(),
            color: WARNING,
        },
        Some(MacroFeedState::Awaiting) | None => MissingSeriesStatus {
            label: "AWAITING",
            detail: "awaiting first transport receipt".into(),
            color: TEXT_DIM,
        },
    }
}

fn feed_availability(snapshot: &MacroSnapshot, source: SourceId, stream: StreamId) -> Availability {
    match feed_snapshot(snapshot, source, stream).map(|feed| feed.state) {
        Some(MacroFeedState::Current) => Availability::Ready,
        Some(MacroFeedState::Rejected) => Availability::Invalid,
        Some(MacroFeedState::NotConfigured) => Availability::NotConfigured,
        Some(MacroFeedState::Awaiting) | None => Availability::AwaitingFirstReceipt,
    }
}

fn feed_contract_detail(
    snapshot: &MacroSnapshot,
    source: SourceId,
    stream: StreamId,
    contract: &'static str,
) -> String {
    feed_snapshot(snapshot, source, stream).map_or_else(
        || format!("{contract} / status unavailable"),
        |feed| format!("{contract} / {}", feed.detail),
    )
}

pub(super) fn source_contract_panel(snapshot: &MacroSnapshot) -> gpui::Div {
    let categories = snapshot.series.iter().fold(
        (0usize, 0usize, 0usize, 0usize),
        |(inflation, labor, rates, growth), series| match series.category {
            MacroCategory::Inflation => (inflation + 1, labor, rates, growth),
            MacroCategory::Labor => (inflation, labor + 1, rates, growth),
            MacroCategory::Rates => (inflation, labor, rates + 1, growth),
            MacroCategory::Growth => (inflation, labor, rates, growth + 1),
        },
    );
    let body = div()
        .flex_1()
        .min_h_0()
        .overflow_y_scrollbar()
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(contract_row(
            "BLS",
            feed_contract_detail(
                snapshot,
                BLS_SOURCE,
                BLS_TIMESERIES_STREAM,
                "U.S. inflation and labor",
            ),
            feed_availability(snapshot, BLS_SOURCE, BLS_TIMESERIES_STREAM),
        ))
        .child(contract_row(
            "CFTC TFF",
            feed_contract_detail(
                snapshot,
                CFTC_SOURCE,
                CFTC_TFF_STREAM,
                "4 of 6 Northstar indices covered",
            ),
            feed_availability(snapshot, CFTC_SOURCE, CFTC_TFF_STREAM),
        ))
        .child(contract_row(
            "EUROSTAT",
            feed_contract_detail(
                snapshot,
                EUROSTAT_SOURCE,
                EUROSTAT_STATISTICS_STREAM,
                "EA GDP + Germany inflation, labor, retail, industry",
            ),
            feed_availability(snapshot, EUROSTAT_SOURCE, EUROSTAT_STATISTICS_STREAM),
        ))
        .child(contract_row(
            "ECB",
            feed_contract_detail(
                snapshot,
                ECB_SOURCE,
                ECB_POLICY_RATES_STREAM,
                "Deposit facility + main refinancing rates",
            ),
            feed_availability(snapshot, ECB_SOURCE, ECB_POLICY_RATES_STREAM),
        ))
        .child(contract_row(
            "ONS",
            feed_contract_detail(
                snapshot,
                ONS_SOURCE,
                ONS_TIMESERIES_STREAM,
                "UK CPI, GDP, unemployment, and retail volume",
            ),
            feed_availability(snapshot, ONS_SOURCE, ONS_TIMESERIES_STREAM),
        ))
        .child(contract_row(
            "BANK OF ENGLAND",
            feed_contract_detail(
                snapshot,
                BOE_SOURCE,
                BOE_RATES_STREAM,
                "Official Bank Rate",
            ),
            feed_availability(snapshot, BOE_SOURCE, BOE_RATES_STREAM),
        ))
        .child(contract_row(
            "BANK OF JAPAN",
            feed_contract_detail(
                snapshot,
                BOJ_SOURCE,
                BOJ_TIMESERIES_STREAM,
                "Overnight call rate + Tankan business conditions",
            ),
            feed_availability(snapshot, BOJ_SOURCE, BOJ_TIMESERIES_STREAM),
        ))
        .child(contract_row(
            "E-STAT",
            "Application ID + exact table dimensions required",
            Availability::NotConfigured,
        ))
        .child(contract_row(
            "FRED",
            feed_contract_detail(
                snapshot,
                FRED_SOURCE,
                FRED_OBSERVATIONS_STREAM,
                "Fed funds + U.S. Treasury 2Y / 10Y",
            ),
            feed_availability(snapshot, FRED_SOURCE, FRED_OBSERVATIONS_STREAM),
        ))
        .child(contract_row(
            "BEA",
            feed_contract_detail(
                snapshot,
                BEA_SOURCE,
                BEA_NIPA_STREAM,
                "Real GDP + core PCE index",
            ),
            feed_availability(snapshot, BEA_SOURCE, BEA_NIPA_STREAM),
        ))
        .child(contract_row(
            "CENSUS MARTS",
            feed_contract_detail(
                snapshot,
                CENSUS_SOURCE,
                CENSUS_MARTS_STREAM,
                "Advance retail + food services sales",
            ),
            feed_availability(snapshot, CENSUS_SOURCE, CENSUS_MARTS_STREAM),
        ))
        .child(contract_row("LICENSE", "public / attribution", Availability::Ready))
        .child(contract_row("REFRESH", "daily / bounded supervisor", snapshot.availability))
        .child(contract_row(
            "PUBLICATION",
            "each source batch is all-or-none",
            Availability::Ready,
        ))
        .child(contract_row("INFLATION", format!("{} series", categories.0), snapshot.availability))
        .child(contract_row("LABOR", format!("{} series", categories.1), snapshot.availability))
        .child(contract_row("RATES", format!("{} series", categories.2), snapshot.availability))
        .child(contract_row("GROWTH", format!("{} series", categories.3), snapshot.availability))
        .child(
            div()
                .mt_2()
                .text_xs()
                .text_color(rgb(TEXT_DIM))
                .child("e-Stat remains credential-gated. Forecast consensus and crowd sentiment remain outside this contract."),
        );
    macro_surface("SOURCE CONTRACT", "official data / no scraping", body)
}

fn provenance_row(label: &'static str, value: String) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .py_1()
        .text_xs()
        .child(div().text_color(rgb(TEXT_DIM)).child(label))
        .child(div().text_color(rgb(TEXT)).child(value))
}

fn contract_row(label: &'static str, detail: impl Into<String>, state: Availability) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .p_2()
        .rounded_md()
        .bg(rgb(SURFACE_ALT))
        .text_xs()
        .child(
            div()
                .child(label)
                .child(div().mt_1().text_color(rgb(TEXT_DIM)).child(detail.into())),
        )
        .child(
            div()
                .size_1p5()
                .rounded_full()
                .bg(rgb(availability_color(state))),
        )
}

fn macro_surface(label: &'static str, detail: &'static str, body: impl IntoElement) -> gpui::Div {
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

pub(super) fn format_value(value: f64, unit: MacroUnit) -> String {
    match unit {
        MacroUnit::Index => format!("{value:.3}"),
        MacroUnit::Percent => format!("{value:.1}%"),
        MacroUnit::Thousands if value.abs() >= 10_000.0 => format!("{:.2}M", value / 1_000.0),
        MacroUnit::Thousands => format!("{value:.0}k"),
        MacroUnit::DollarsPerHour => format!("${value:.2}/h"),
        MacroUnit::MillionsOfDollars if value.abs() >= 1_000.0 => {
            format!("${:.1}B", value / 1_000.0)
        }
        MacroUnit::MillionsOfDollars => format!("${value:.0}M"),
        MacroUnit::PercentagePoints => format!("{value:.1} pts"),
    }
}

fn format_contracts(value: i64) -> String {
    let absolute = value.unsigned_abs();
    let sign = if value >= 0 { "+" } else { "-" };
    if absolute >= 1_000_000 {
        format!("{sign}{:.2}M", absolute as f64 / 1_000_000.0)
    } else if absolute >= 1_000 {
        format!("{sign}{:.1}k", absolute as f64 / 1_000.0)
    } else {
        format!("{sign}{absolute}")
    }
}

fn time_quality_label(quality: TimeQuality) -> &'static str {
    match quality {
        TimeQuality::ObservedLive => "observed live / receipt effective",
        TimeQuality::VerifiedPublication => "verified publication time",
        TimeQuality::EstimatedHistorical => "estimated historical time",
        TimeQuality::Unknown => "unknown time quality",
        TimeQuality::ProviderDelayed => "provider-delayed observation",
    }
}

fn format_period(timestamp_ns: i64) -> String {
    let (year, month, _) = civil_from_ns(timestamp_ns);
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let label = MONTHS
        .get(month.saturating_sub(1) as usize)
        .unwrap_or(&"???");
    format!("{label} {year}")
}

pub(super) fn format_timestamp(timestamp_ns: i64) -> String {
    let (year, month, day) = civil_from_ns(timestamp_ns);
    let seconds = timestamp_ns.div_euclid(1_000_000_000).rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        seconds / 3_600,
        (seconds % 3_600) / 60
    )
}

fn format_date(timestamp_ns: i64) -> String {
    let (year, month, day) = civil_from_ns(timestamp_ns);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_ns(timestamp_ns: i64) -> (i64, u8, u8) {
    let days = timestamp_ns.div_euclid(86_400_000_000_000);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u8, day as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_civil_format_is_stable_without_locale_state() {
        assert_eq!(civil_from_ns(0), (1970, 1, 1));
        assert_eq!(format_period(0), "Jan 1970");
        assert_eq!(format_timestamp(0), "1970-01-01 00:00 UTC");
    }
}
