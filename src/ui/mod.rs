mod fund;
mod ledger;
mod macro_page;
mod mission;
mod systems;

use crate::chart_engine::{ChartViewport, InvalidationLevel};
use crate::contracts::{DashboardSnapshot, IndexKey};
use crate::office::RegionKey;
use crate::runtime::{MarketDataPort, PrototypeRuntime};
use crate::ui_theme::*;
use gpui::{
    div, prelude::*, px, rgb, Context, FontWeight, IntoElement, Pixels, Point, Render,
    SharedString, Window,
};
use gpui_animated_gradient_text::{GradientPalette, GradientText};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Page {
    Desk,
    Macro,
    Fund,
    Systems,
    Ledger,
}

impl Page {
    const ALL: [Self; 5] = [
        Self::Desk,
        Self::Macro,
        Self::Fund,
        Self::Systems,
        Self::Ledger,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Desk => "Desk",
            Self::Macro => "Macro",
            Self::Fund => "Fund",
            Self::Systems => "Systems",
            Self::Ledger => "Ledger",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum SystemsView {
    #[default]
    Operations,
    StrategyRuntime,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum LedgerView {
    #[default]
    Lifecycle,
    Decisions,
    Orders,
    Fills,
    Positions,
    Reconciliation,
    Incidents,
}

pub struct NorthstarApp {
    pub(super) runtime: Arc<PrototypeRuntime>,
    pub(super) snapshot: Arc<DashboardSnapshot>,
    pub(super) selected_index: IndexKey,
    pub(super) page: Page,
    pub(super) timeframe: usize,
    pub(super) overlays_visible: bool,
    pub(super) arming_review_open: bool,
    pub(super) selected_strategy: usize,
    pub(super) selected_region: RegionKey,
    pub(super) selected_macro_release: usize,
    pub(super) systems_view: SystemsView,
    pub(super) ledger_view: LedgerView,
    pub(super) selected_receipt: usize,
    pub(super) chart_viewport: ChartViewport,
    pub(super) chart_cursor: Option<Point<Pixels>>,
    pub(super) chart_drag_position: Option<Point<Pixels>>,
    pub(super) chart_damage: InvalidationLevel,
}

impl NorthstarApp {
    pub fn new(runtime: Arc<PrototypeRuntime>, _cx: &mut Context<Self>) -> Self {
        let snapshot = runtime.dashboard_snapshot();
        Self {
            runtime,
            snapshot,
            selected_index: IndexKey::Us100,
            page: Page::Desk,
            timeframe: 1,
            overlays_visible: true,
            arming_review_open: false,
            selected_strategy: 0,
            selected_region: RegionKey::UnitedStates,
            selected_macro_release: 0,
            systems_view: SystemsView::Operations,
            ledger_view: LedgerView::Lifecycle,
            selected_receipt: 0,
            chart_viewport: ChartViewport::default(),
            chart_cursor: None,
            chart_drag_position: None,
            chart_damage: InvalidationLevel::Layout,
        }
    }

    fn render_topbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = GradientPalette::from_hex([0x57dfbb, 0x79a9ff, 0xd779ff])
            .unwrap_or_else(|_| GradientPalette::phoenix());
        let mut nav = div().h_full().flex().items_center().gap_1();
        for page in Page::ALL {
            let selected = self.page == page;
            nav = nav.child(
                div()
                    .id(("primary-page", page as usize))
                    .h_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .border_b_2()
                    .border_color(rgb(if selected { ACCENT } else { SURFACE }))
                    .bg(rgb(if selected { 0x1b2321 } else { SURFACE }))
                    .text_sm()
                    .text_color(rgb(if selected { TEXT } else { TEXT_MUTED }))
                    .cursor_pointer()
                    .hover(|item| item.bg(rgb(0x202624)).text_color(rgb(TEXT)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.page != page {
                            this.page = page;
                            cx.notify();
                        }
                    }))
                    .child(page.label()),
            );
        }

        let health = &self.snapshot.health;
        div()
            .h(px(HEADER_HEIGHT))
            .w_full()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .border_b_1()
            .border_color(rgb(BORDER))
            .bg(rgb(SURFACE))
            .child(
                div()
                    .min_w(px(208.0))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size(px(24.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x426d61))
                            .bg(rgb(SURFACE_ACTIVE))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(ACCENT))
                            .child("N"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child(GradientText::new("NORTHSTAR / INDEX", palette)),
                    ),
            )
            .child(nav)
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(health_pill(
                        "Massive",
                        format!("{} ms", health.massive_latency_ms),
                        ACCENT,
                    ))
                    .child(health_pill(
                        "TradeLocker",
                        if health.tradelocker_synced {
                            "synced"
                        } else {
                            "blocked"
                        },
                        if health.tradelocker_synced {
                            ACCENT
                        } else {
                            DANGER
                        },
                    ))
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x655033))
                            .bg(rgb(0x332719))
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(WARNING))
                            .child("SIM / GUARDED"),
                    ),
            )
    }

    pub(super) fn selected(&self) -> &crate::contracts::IndexSnapshot {
        self.runtime.index_snapshot(self.selected_index)
    }
}

impl Render for NorthstarApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = f32::from(window.viewport_size().width) < 1_460.0;
        let page = match self.page {
            Page::Desk => self.render_mission_control(compact, cx).into_any_element(),
            Page::Macro => self.render_macro_office(compact, cx).into_any_element(),
            Page::Fund => self.render_fund_board(compact, cx).into_any_element(),
            Page::Systems => self.render_systems(compact, cx).into_any_element(),
            Page::Ledger => self.render_lifecycle_ledger(compact, cx).into_any_element(),
        };
        div()
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(CANVAS))
            .text_color(rgb(TEXT))
            .child(self.render_topbar(cx))
            .child(page)
            .when(self.arming_review_open, |root| {
                root.child(self.render_arming_review(cx))
            })
    }
}

pub(super) fn panel_header(
    label: impl Into<SharedString>,
    detail: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .h_10()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_b_1()
        .border_color(rgb(BORDER_DIM))
        .bg(rgb(SURFACE))
        .text_xs()
        .text_color(rgb(0xaab6b2))
        .child(label.into())
        .child(div().text_color(rgb(TEXT_DIM)).child(detail.into()))
}

pub(super) fn health_pill(
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    color: u32,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x121615))
        .text_xs()
        .text_color(rgb(TEXT_MUTED))
        .child(div().size_1p5().rounded_full().bg(rgb(color)))
        .child(label.into())
        .child(div().text_color(rgb(TEXT)).child(value.into()))
}

pub(super) fn status_color(status: crate::contracts::GateStatus) -> u32 {
    use crate::contracts::GateStatus;
    match status {
        GateStatus::Pass => ACCENT,
        GateStatus::Observe => LAVENDER,
        GateStatus::Wait => WARNING,
        GateStatus::Block => DANGER,
    }
}

pub(super) fn status_label(status: crate::contracts::GateStatus) -> &'static str {
    use crate::contracts::GateStatus;
    match status {
        GateStatus::Pass => "PASS",
        GateStatus::Observe => "OBSERVE",
        GateStatus::Wait => "WAIT",
        GateStatus::Block => "BLOCK",
    }
}
