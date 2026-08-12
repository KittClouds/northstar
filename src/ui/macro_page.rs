use super::{
    panel_header, status_color, NorthstarApp, ACCENT, BORDER, BORDER_DIM, DANGER, LAVENDER,
    SURFACE_ACTIVE, SURFACE_ALT, TEXT, TEXT_DIM, TEXT_MUTED, WARNING,
};
use crate::contracts::IndexKey;
use crate::office::{MacroFamily, MacroRegionSnapshot, RegionKey, ReleaseStatus};
use gpui::{div, prelude::*, px, rgb, Context, FontWeight, IntoElement};

impl NorthstarApp {
    pub(super) fn render_macro_office(
        &self,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let macro_office = &self.snapshot.macro_office;
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
                                    .child("Global Macro"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Official observations, revisions, positioning, and release provenance."),
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
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!(
                                "SNAPSHOT {} / CATALOG V{} / {}",
                                macro_office.snapshot_id,
                                macro_office.catalog_version,
                                macro_office.as_of
                            )),
                    ),
            )
            .child(self.render_macro_situation_strip())
            .child(self.render_region_cards(compact, cx))
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
                            .child(self.render_context_matrix())
                            .child(self.render_positioning_panel()),
                    )
                    .child(
                        div()
                            .col_span(if compact { 1 } else { 5 })
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(self.render_release_table(cx))
                            .child(self.render_release_inspector())
                            .when(!compact, |column| {
                                column.child(self.render_source_documents())
                            }),
                    ),
            )
    }

    fn render_macro_situation_strip(&self) -> impl IntoElement {
        let data = &self.snapshot.macro_office;
        div()
            .grid()
            .grid_cols(5)
            .gap_2()
            .child(macro_metric(
                "NEXT OPERATING RELEASE",
                data.next_release,
                "calendar v42",
                WARNING,
            ))
            .child(macro_metric(
                "PUBLICATION",
                "atomic / current",
                format!("sequence {}", self.snapshot.operations.publication_sequence),
                ACCENT,
            ))
            .child(macro_metric(
                "LATE RELEASES",
                data.late_releases.to_string(),
                "Japan industrial output",
                if data.late_releases == 0 {
                    ACCENT
                } else {
                    WARNING
                },
            ))
            .child(macro_metric(
                "STALE FAMILIES",
                data.stale_families.to_string(),
                "missing is never zero",
                if data.stale_families == 0 {
                    ACCENT
                } else {
                    DANGER
                },
            ))
            .child(macro_metric(
                "MODE",
                "replay-safe fixture",
                "no scoring / no model",
                LAVENDER,
            ))
    }

    fn render_region_cards(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let mut cards = div()
            .mt_3()
            .grid()
            .grid_cols(if compact { 2 } else { 4 })
            .gap_2();
        for key in RegionKey::ALL {
            let region = self.snapshot.macro_office.region(key);
            let selected = self.selected_region == key;
            cards = cards.child(
                div()
                    .id(("macro-region", key as usize))
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(if selected { ACCENT } else { BORDER }))
                    .bg(rgb(if selected {
                        SURFACE_ACTIVE
                    } else {
                        SURFACE_ALT
                    }))
                    .cursor_pointer()
                    .hover(|item| item.bg(rgb(0x202724)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.selected_region != key {
                            this.selected_region = key;
                            cx.notify();
                        }
                    }))
                    .child(region_card_header(region, selected))
                    .child(
                        div()
                            .mt_2()
                            .grid()
                            .grid_cols(2)
                            .gap_x_3()
                            .gap_y_1()
                            .text_xs()
                            .child(region_value("Growth", region.growth))
                            .child(region_value("Inflation", region.inflation))
                            .child(region_value("Policy", region.policy_rate))
                            .child(region_value("Next", region.next_release)),
                    ),
            );
        }
        cards
    }

    fn render_context_matrix(&self) -> impl IntoElement {
        let mut table = div().flex_1().min_h_0();
        let mut header = div()
            .grid()
            .grid_cols(7)
            .items_center()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(BORDER_DIM))
            .text_xs()
            .text_color(rgb(TEXT_DIM))
            .child("INDEX");
        for family in MacroFamily::ALL {
            header = header.child(family.label().to_uppercase());
        }
        table = table.child(header);

        for index in IndexKey::ALL {
            let mut row = div()
                .grid()
                .grid_cols(7)
                .items_center()
                .px_3()
                .py_1()
                .border_b_1()
                .border_color(rgb(BORDER_DIM))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(if self.selected_index == index {
                            ACCENT
                        } else {
                            TEXT
                        }))
                        .child(index.symbol()),
                );
            for family in MacroFamily::ALL {
                let cell = self.snapshot.macro_office.context_cell(index, family);
                row = row.child(
                    div()
                        .pl_2()
                        .border_l_2()
                        .border_color(rgb(status_color(cell.status)))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(status_color(cell.status)))
                                .child(cell.state),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(TEXT_MUTED))
                                .child(cell.value),
                        ),
                );
            }
            table = table.child(row);
        }

        div()
            .flex_1()
            .min_h(px(270.0))
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header(
                "INDEX CONTEXT",
                "facts + transparent transforms",
            ))
            .child(table)
    }

    fn render_positioning_panel(&self) -> impl IntoElement {
        let positioning = self
            .snapshot
            .macro_office
            .positioning(self.selected_index)
            .expect("fixture positioning for every index");
        let available = positioning.percentile > 0;
        div()
            .h(px(112.0))
            .flex_shrink_0()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header(
                format!("POSITIONING / {}", self.selected_index.symbol()),
                format!(
                    "{} / {}d old",
                    positioning.contract, positioning.report_age_days
                ),
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(5)
                    .gap_3()
                    .p_2()
                    .child(position_value(
                        "Asset managers",
                        signed_count(positioning.asset_manager_net),
                    ))
                    .child(position_value(
                        "Leveraged",
                        signed_count(positioning.leveraged_money_net),
                    ))
                    .child(position_value(
                        "Dealers",
                        signed_count(positioning.dealer_net),
                    ))
                    .child(position_value(
                        "Weekly delta",
                        signed_count(positioning.weekly_delta),
                    ))
                    .child(
                        div()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child("Historical range"),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(if available { ACCENT } else { WARNING }))
                                    .child(if available {
                                        format!("{}th percentile", positioning.percentile)
                                    } else {
                                        "unavailable".to_string()
                                    }),
                            )
                            .child(div().mt_2().h_1p5().rounded_full().bg(rgb(0x29322f)).when(
                                available,
                                |bar| {
                                    bar.child(
                                        div()
                                            .h_full()
                                            .w(gpui::relative(
                                                positioning.percentile as f32 / 100.0,
                                            ))
                                            .rounded_full()
                                            .bg(rgb(LAVENDER)),
                                    )
                                },
                            )),
                    ),
            )
    }

    fn render_release_table(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows = div().flex_1().min_h_0();
        for (slot, release) in self.snapshot.macro_office.releases.iter().enumerate() {
            let selected = self.selected_macro_release == slot;
            rows = rows.child(
                div()
                    .id(("macro-release", slot))
                    .grid()
                    .grid_cols(8)
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .bg(rgb(if selected { SURFACE_ACTIVE } else { 0x141817 }))
                    .cursor_pointer()
                    .hover(|item| item.bg(rgb(0x202724)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.selected_macro_release != slot {
                            this.selected_macro_release = slot;
                            cx.notify();
                        }
                    }))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(release.time_label),
                    )
                    .child(div().text_xs().child(region_short(release.region)))
                    .child(div().col_span(2).text_sm().child(release.name))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child(release.actual),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(release.previous),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(release.revised),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(release_status_color(release.status)))
                            .child(release.change),
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
            .child(panel_header("RELEASE TAPE", "actual / previous / revised"))
            .child(rows)
    }

    fn render_release_inspector(&self) -> impl IntoElement {
        let release = &self.snapshot.macro_office.releases[self.selected_macro_release];
        div()
            .flex_shrink_0()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(SURFACE_ALT))
            .child(panel_header(
                format!("RELEASE #{}", release.release_id),
                release.freshness,
            ))
            .child(
                div()
                    .p_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().font_weight(FontWeight::BOLD).child(release.name))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(release_status_color(release.status)))
                                    .child(release_status_label(release.status)),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!(
                                "{} / {} / {}",
                                release.source,
                                release.provenance,
                                release.region.label()
                            )),
                    ),
            )
    }

    fn render_source_documents(&self) -> impl IntoElement {
        let mut rows = div();
        for document in self.snapshot.macro_office.documents.iter().take(3) {
            rows = rows.child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER_DIM))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .text_xs()
                            .child(
                                div()
                                    .text_color(rgb(if document.official {
                                        ACCENT
                                    } else {
                                        LAVENDER
                                    }))
                                    .child(document.institution),
                            )
                            .child(div().text_color(rgb(TEXT_DIM)).child(document.published)),
                    )
                    .child(div().mt_1().text_sm().child(document.title)),
            );
        }
        div()
            .flex_shrink_0()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .bg(rgb(0x141817))
            .child(panel_header("OFFICIAL DOCUMENTS", "source receipts"))
            .child(rows)
    }
}

fn macro_metric(
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

fn region_card_header(region: &MacroRegionSnapshot, selected: bool) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(if selected { ACCENT } else { TEXT }))
                .child(region.region.label()),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(TEXT_DIM))
                .child(region.freshness),
        )
}

fn region_value(label: &'static str, value: &'static str) -> impl IntoElement {
    div()
        .child(div().text_color(rgb(TEXT_DIM)).child(label))
        .child(div().mt_0p5().text_color(rgb(TEXT_MUTED)).child(value))
}

fn position_value(label: &'static str, value: String) -> impl IntoElement {
    div()
        .child(div().text_xs().text_color(rgb(TEXT_MUTED)).child(label))
        .child(div().mt_1().font_weight(FontWeight::BOLD).child(value))
}

fn signed_count(value: i32) -> String {
    if value == 0 {
        "unavailable".to_string()
    } else {
        format!("{value:+}")
    }
}

fn region_short(region: RegionKey) -> &'static str {
    match region {
        RegionKey::UnitedStates => "US",
        RegionKey::EuroArea => "EU",
        RegionKey::UnitedKingdom => "UK",
        RegionKey::Japan => "JP",
    }
}

fn release_status_color(status: ReleaseStatus) -> u32 {
    match status {
        ReleaseStatus::Scheduled => WARNING,
        ReleaseStatus::Received => ACCENT,
        ReleaseStatus::Revised => LAVENDER,
        ReleaseStatus::Late => DANGER,
    }
}

fn release_status_label(status: ReleaseStatus) -> &'static str {
    match status {
        ReleaseStatus::Scheduled => "SCHEDULED",
        ReleaseStatus::Received => "RECEIVED",
        ReleaseStatus::Revised => "REVISED",
        ReleaseStatus::Late => "LATE",
    }
}
