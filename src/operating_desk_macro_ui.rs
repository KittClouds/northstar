use super::*;
use crate::data_plane::ids::{instruments, macro_series, SeriesId};
use crate::data_plane::providers::macro_catalog::MacroUnit;
use crate::operating::MacroSeriesSnapshot;

impl OperatingApp {
    pub(super) fn render_desk_macro_context(&self, compact: bool) -> impl IntoElement {
        let selected = self.selected_instrument;
        let (region, facts): (&str, [(&str, Option<SeriesId>); 3]) = match selected {
            Some(instruments::US100 | instruments::US500 | instruments::US30) => (
                "UNITED STATES",
                [
                    ("INFLATION", Some(macro_series::US_CPI_ALL_ITEMS_NSA)),
                    ("GROWTH", Some(macro_series::US_REAL_GDP_QOQ_ANNUALIZED)),
                    ("RATES", Some(macro_series::US_TREASURY_10Y)),
                ],
            ),
            Some(instruments::DE40) => (
                "EURO AREA / GERMANY",
                [
                    ("INFLATION", Some(macro_series::DE_HICP_ALL_ITEMS_YOY)),
                    ("GROWTH", Some(macro_series::EA_REAL_GDP_QOQ)),
                    ("RATES", Some(macro_series::ECB_DEPOSIT_FACILITY_RATE)),
                ],
            ),
            Some(instruments::UK100) => (
                "UNITED KINGDOM",
                [
                    ("INFLATION", Some(macro_series::UK_CPI_ALL_ITEMS_YOY)),
                    ("GROWTH", Some(macro_series::UK_REAL_GDP_QOQ)),
                    ("RATES", Some(macro_series::BOE_BANK_RATE)),
                ],
            ),
            Some(instruments::JP225) => (
                "JAPAN",
                [
                    ("RATES", Some(macro_series::JP_OVERNIGHT_CALL_RATE)),
                    (
                        "TANKAN",
                        Some(macro_series::JP_TANKAN_LARGE_MANUFACTURING_CONDITIONS),
                    ),
                    ("E-STAT CORE", None),
                ],
            ),
            _ => (
                "NO INDEX SELECTED",
                [("INFLATION", None), ("GROWTH", None), ("RATES", None)],
            ),
        };
        let macro_snapshot = &self.snapshot.macro_office;
        let region_detail = selected
            .and_then(|id| {
                self.snapshot
                    .desk
                    .instruments
                    .iter()
                    .find(|index| index.instrument.id == id)
            })
            .map_or("select an index".to_owned(), |index| {
                format!(
                    "{} / replay-safe {}",
                    index.instrument.display_name,
                    format_sequence(macro_snapshot.last_sequence)
                )
            });
        let mut strip = div()
            .h(px(if compact { 150.0 } else { 82.0 }))
            .flex_shrink_0()
            .grid()
            .grid_cols(if compact { 2 } else { 4 })
            .gap_3()
            .child(desk_macro_card(
                "MACRO CONTEXT",
                region.to_owned(),
                region_detail,
                macro_snapshot.availability,
            ));
        for (title, series_id) in facts {
            let series = series_id.and_then(|id| {
                macro_snapshot
                    .series
                    .iter()
                    .find(|series| series.series_id == id)
            });
            strip = strip.child(desk_series_card(title, series));
        }
        strip
    }
}

fn desk_series_card(title: &'static str, series: Option<&MacroSeriesSnapshot>) -> gpui::Div {
    match series {
        Some(series) => {
            let (value, availability) = series.latest.map_or_else(
                || {
                    (
                        "awaiting receipt".to_owned(),
                        Availability::AwaitingFirstReceipt,
                    )
                },
                |point| {
                    (
                        desk_macro_value(point.value, series.unit),
                        Availability::Live,
                    )
                },
            );
            desk_macro_card(
                title,
                value,
                format!(
                    "{} / {} / {}",
                    series.provider_code,
                    series.measure.label(),
                    format_sequence(
                        series
                            .latest
                            .map_or(JournalSequence::UNKNOWN, |p| p.sequence)
                    )
                ),
                availability,
            )
        }
        None => desk_macro_card(
            title,
            "not configured".into(),
            "credential or verified table contract required".into(),
            Availability::NotConfigured,
        ),
    }
}

fn desk_macro_card(
    title: &'static str,
    value: String,
    detail: String,
    availability: Availability,
) -> gpui::Div {
    div()
        .min_w_0()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER_DIM))
        .bg(rgb(SURFACE))
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .min_w_0()
                .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(title))
                .child(
                    div()
                        .mt_1()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child(value),
                )
                .child(
                    div()
                        .mt_1()
                        .text_xs()
                        .text_color(rgb(TEXT_MUTED))
                        .child(detail),
                ),
        )
        .child(
            div()
                .size_1p5()
                .rounded_full()
                .flex_shrink_0()
                .bg(rgb(availability_color(availability))),
        )
}

fn desk_macro_value(value: f64, unit: MacroUnit) -> String {
    match unit {
        MacroUnit::Index => format!("{value:.2}"),
        MacroUnit::Percent => format!("{value:.2}%"),
        MacroUnit::PercentagePoints => format!("{value:.1} pts"),
        MacroUnit::Thousands => format!("{value:.0}k"),
        MacroUnit::DollarsPerHour => format!("${value:.2}/h"),
        MacroUnit::MillionsOfDollars => format!("${value:.0}M"),
    }
}
