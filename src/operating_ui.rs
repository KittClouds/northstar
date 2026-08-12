use crate::chart_engine::ChartViewport;
use crate::data_plane::bars::Timeframe;
use crate::data_plane::ids::{InstrumentId, JournalSequence};
use crate::operating::{
    Availability, DomainMask, NorthstarRuntime, OfficeSnapshot, OfficeSnapshotPort,
    OperatorEntryKind, ReferenceRole,
};
use crate::operating_chart::{canonical_index_chart, canonical_slot_count};
use crate::operating_ui_status::*;
use crate::ui_theme::*;
use async_channel::TrySendError as AsyncTrySendError;
use crossbeam_channel::{bounded, Sender};
use gpui::{
    div, prelude::*, px, rgb, Context, CursorStyle, Entity, FontWeight, IntoElement,
    MouseMoveEvent, Pixels, Point, Render, ScrollWheelEvent, SharedString, Task, Window,
};
use gpui_animated_gradient_text::{GradientPalette, GradientText};
use gpui_component::input::InputState;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

#[path = "operating_desk_macro_ui.rs"]
mod desk_macro_ui;
#[path = "operating_fund_systems_ui.rs"]
mod fund_systems_ui;
#[path = "operating_ledger_ui.rs"]
mod ledger_ui;
#[path = "operating_macro_board_ui.rs"]
mod macro_board_ui;
#[path = "operating_macro_ui.rs"]
mod macro_ui;
#[path = "operating_page.rs"]
mod page;
#[path = "operating_ui_render.rs"]
mod render;
use page::OperatingPage;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OperatorComposerAction {
    Append,
    Amend,
    Redact,
}

impl OperatorComposerAction {
    const ALL: [Self; 3] = [Self::Append, Self::Amend, Self::Redact];

    const fn label(self) -> &'static str {
        match self {
            Self::Append => "Append",
            Self::Amend => "Amend",
            Self::Redact => "Redact",
        }
    }
}

struct UiPending {
    changed: AtomicU8,
    generation: AtomicU64,
}

pub struct OperatingApp {
    runtime: Arc<NorthstarRuntime>,
    snapshot: Arc<OfficeSnapshot>,
    page: OperatingPage,
    selected_instrument: Option<InstrumentId>,
    chart_timeframe: Timeframe,
    chart_viewport: ChartViewport,
    chart_cursor: Option<Point<Pixels>>,
    chart_drag_position: Option<Point<Pixels>>,
    ledger_note_input: Entity<InputState>,
    selected_ledger_case: Option<crate::ledger::TradeCaseId>,
    selected_ledger_entry: Option<crate::ledger::LedgerEntryId>,
    ledger_entry_kind: OperatorEntryKind,
    ledger_composer_action: OperatorComposerAction,
    ledger_submit_nonce: u64,
    ledger_status: Option<SharedString>,
    last_changed: DomainMask,
    ui_updates: u64,
    stop_sender: Option<Sender<()>>,
    snapshot_thread: Option<JoinHandle<()>>,
    snapshot_task: Option<Task<()>>,
}

impl OperatingApp {
    pub fn new(
        runtime: Arc<NorthstarRuntime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let snapshot = runtime.current();
        let selected_instrument = snapshot
            .desk
            .instruments
            .first()
            .map(|index| index.instrument.id);
        let selected_ledger_case = snapshot.ledger.cases.first().map(|case| case.case_id);
        let mut app = Self {
            runtime,
            snapshot,
            page: OperatingPage::Desk,
            selected_instrument,
            chart_timeframe: Timeframe::M4,
            chart_viewport: ChartViewport::default(),
            chart_cursor: None,
            chart_drag_position: None,
            ledger_note_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Write a plan, observation, intervention, or review note...")
            }),
            selected_ledger_case,
            selected_ledger_entry: None,
            ledger_entry_kind: OperatorEntryKind::Plan,
            ledger_composer_action: OperatorComposerAction::Append,
            ledger_submit_nonce: 0,
            ledger_status: None,
            last_changed: DomainMask::ALL,
            ui_updates: 0,
            stop_sender: None,
            snapshot_thread: None,
            snapshot_task: None,
        };
        app.start_snapshot_loop(cx);
        app
    }

    fn start_snapshot_loop(&mut self, cx: &mut Context<Self>) {
        let subscription = self.runtime.subscribe();
        let (stop_sender, stop_receiver) = bounded(1);
        let (ui_sender, ui_receiver) = async_channel::bounded(1);
        let pending = Arc::new(UiPending {
            changed: AtomicU8::new(0),
            generation: AtomicU64::new(self.snapshot.generation.0),
        });
        let thread_pending = Arc::clone(&pending);
        let snapshot_thread = std::thread::Builder::new()
            .name("northstar-snapshot-wake".into())
            .spawn(move || {
                while let Some(notice) = subscription.wait_or_cancel(&stop_receiver) {
                    thread_pending
                        .changed
                        .fetch_or(notice.changed.bits(), Ordering::AcqRel);
                    thread_pending
                        .generation
                        .store(notice.generation.0, Ordering::Release);
                    match ui_sender.try_send(()) {
                        Ok(()) | Err(AsyncTrySendError::Full(())) => {}
                        Err(AsyncTrySendError::Closed(())) => break,
                    }
                }
            })
            .expect("spawn Northstar snapshot wake thread");

        let runtime = Arc::clone(&self.runtime);
        self.snapshot_task = Some(cx.spawn(async move |app, async_cx| {
            while ui_receiver.recv().await.is_ok() {
                let changed =
                    DomainMask::from_bits_truncate(pending.changed.swap(0, Ordering::AcqRel));
                let advertised_generation = pending.generation.load(Ordering::Acquire);
                if changed.is_empty() {
                    continue;
                }
                if app
                    .update(async_cx, |app, cx| {
                        let next = runtime.current();
                        if next.generation.0 < advertised_generation
                            || next.generation <= app.snapshot.generation
                        {
                            return;
                        }
                        app.snapshot = next;
                        app.last_changed = changed;
                        app.ui_updates = app.ui_updates.saturating_add(1);
                        app.repair_selection();
                        if app.page.wants(changed) {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
        self.stop_sender = Some(stop_sender);
        self.snapshot_thread = Some(snapshot_thread);
    }

    fn repair_selection(&mut self) {
        let previous = self.selected_instrument;
        let selected_exists = self.selected_instrument.is_some_and(|selected| {
            self.snapshot
                .desk
                .instruments
                .iter()
                .any(|index| index.instrument.id == selected)
        });
        if !selected_exists {
            self.selected_instrument = self
                .snapshot
                .desk
                .instruments
                .first()
                .map(|index| index.instrument.id);
        }
        if self.selected_instrument != previous {
            self.reset_chart_view();
        }
        let ledger_case_exists = self.selected_ledger_case.is_some_and(|selected| {
            self.snapshot
                .ledger
                .cases
                .iter()
                .any(|case| case.case_id == selected)
        });
        if !ledger_case_exists {
            self.selected_ledger_case = self.snapshot.ledger.cases.first().map(|case| case.case_id);
        }
        let ledger_entry_exists = self.selected_ledger_entry.is_some_and(|selected| {
            self.snapshot.ledger.rows.iter().any(|row| {
                row.entry_id == selected
                    && self
                        .selected_ledger_case
                        .is_none_or(|case_id| row.case_id == case_id)
            })
        });
        if !ledger_entry_exists {
            self.selected_ledger_entry = self.selected_ledger_case.and_then(|case_id| {
                self.snapshot
                    .ledger
                    .rows
                    .iter()
                    .find(|row| row.case_id == case_id)
                    .map(|row| row.entry_id)
            });
        }
        if self.last_changed.contains(DomainMask::LEDGER)
            && self.snapshot.ledger.availability == Availability::Ready
        {
            self.ledger_status = Some("Durably committed".into());
        }
    }

    fn reset_chart_view(&mut self) {
        self.chart_viewport = ChartViewport::default();
        self.chart_cursor = None;
        self.chart_drag_position = None;
    }

    fn selected(&self) -> Option<&crate::operating::IndexMarketSnapshot> {
        let selected = self.selected_instrument?;
        self.snapshot
            .desk
            .instruments
            .iter()
            .find(|index| index.instrument.id == selected)
            .map(AsRef::as_ref)
    }

    fn selected_arc(&self) -> Option<Arc<crate::operating::IndexMarketSnapshot>> {
        let selected = self.selected_instrument?;
        self.snapshot
            .desk
            .instruments
            .iter()
            .find(|index| index.instrument.id == selected)
            .map(Arc::clone)
    }

    fn render_topbar(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = GradientPalette::from_hex([0x57dfbb, 0x79a9ff, 0xd779ff])
            .unwrap_or_else(|_| GradientPalette::phoenix());
        let mut navigation = div().h_full().flex().items_center().gap_1();
        for page in OperatingPage::ALL {
            let selected = page == self.page;
            navigation = navigation.child(
                div()
                    .id(("operating-page", page as usize))
                    .h_full()
                    .px(if compact { px(8.0) } else { px(12.0) })
                    .flex()
                    .items_center()
                    .border_b_2()
                    .border_color(rgb(if selected { ACCENT } else { SURFACE }))
                    .bg(rgb(if selected { 0x1b2321 } else { SURFACE }))
                    .text_sm()
                    .text_color(rgb(if selected { TEXT } else { TEXT_MUTED }))
                    .cursor_pointer()
                    .hover(|item| item.bg(rgb(0x202624)).text_color(rgb(TEXT)))
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if app.page != page {
                            app.page = page;
                            cx.notify();
                        }
                    }))
                    .child(page.label()),
            );
        }

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
                    .min_w(px(if compact { 206.0 } else { 226.0 }))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(brand_mark())
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child(GradientText::new("NORTHSTAR / INDEX", palette)),
                    ),
            )
            .child(navigation)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(compact, |status| status.ml_3())
                    .when(!compact, |status| status.ml_auto())
                    .child(status_pill(
                        "Massive",
                        if compact {
                            availability_short_label(self.snapshot.desk.availability)
                        } else {
                            availability_label(self.snapshot.desk.availability)
                        },
                        availability_color(self.snapshot.desk.availability),
                    ))
                    .child(status_pill(
                        "TradeLocker",
                        if compact {
                            availability_short_label(self.snapshot.venue.availability)
                        } else {
                            availability_label(self.snapshot.venue.availability)
                        },
                        availability_color(self.snapshot.venue.availability),
                    ))
                    .child(mode_badge(self.snapshot.mode, compact)),
            )
    }

    fn render_desk(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_0()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child(self.render_pipeline_strip(compact))
            .child(self.render_desk_macro_context(compact))
            .child(
                div()
                    .h_0()
                    .min_h_0()
                    .flex_1()
                    .flex()
                    .gap_3()
                    .child(self.render_universe(cx))
                    .child(self.render_market_canvas(cx))
                    .when(!compact, |row| row.child(self.render_venue_truth())),
            )
    }

    fn render_universe(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div().flex_1().min_h_0().flex().flex_col().gap_2().p_3();
        if self.snapshot.desk.instruments.is_empty() {
            body = body.child(empty_state(
                "No canonical instrument catalog",
                "Publish the licensed six-index catalog before market values are accepted.",
                Availability::NotConfigured,
            ));
        } else {
            for index in self.snapshot.desk.instruments.iter() {
                let id = index.instrument.id;
                let selected = self.selected_instrument == Some(id);
                let value = index
                    .reference
                    .value_f64()
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "—".into());
                body = body.child(
                    div()
                        .id(("operating-index", id.get()))
                        .p_3()
                        .rounded_lg()
                        .border_1()
                        .border_color(rgb(if selected { 0x426d61 } else { BORDER_DIM }))
                        .bg(rgb(if selected {
                            SURFACE_ACTIVE
                        } else {
                            SURFACE_ALT
                        }))
                        .cursor_pointer()
                        .hover(|item| item.border_color(rgb(0x426d61)))
                        .on_click(cx.listener(move |app, _, _, cx| {
                            if app.selected_instrument != Some(id) {
                                app.selected_instrument = Some(id);
                                app.reset_chart_view();
                                cx.notify();
                            }
                        }))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child(SharedString::new(
                                                    index.instrument.display_name.clone(),
                                                )),
                                        )
                                        .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(
                                            SharedString::new(format!(
                                                "{}{}",
                                                index.instrument.reference_symbol,
                                                if index.instrument.reference_role
                                                    == ReferenceRole::ContextProxy
                                                {
                                                    " · CONTEXT PROXY"
                                                } else {
                                                    ""
                                                }
                                            )),
                                        )),
                                )
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(
                                            if index.reference.meta.availability.has_value() {
                                                TEXT
                                            } else {
                                                TEXT_DIM
                                            },
                                        ))
                                        .child(value),
                                ),
                        )
                        .child(
                            div()
                                .mt_2()
                                .child(availability_badge(index.reference.meta.availability)),
                        ),
                );
            }
        }
        panel("INDEX UNIVERSE", "reference authority / Massive", body)
            .w(px(292.0))
            .flex_shrink_0()
    }

    fn render_timeframe_tabs(&self, cx: &mut Context<Self>) -> gpui::Div {
        let mut tabs = div().flex().items_center().gap_1();
        for timeframe in Timeframe::ALL {
            let selected = timeframe == self.chart_timeframe;
            tabs = tabs.child(
                div()
                    .id(("canonical-timeframe", timeframe as u32))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(if selected { 0x426d61 } else { BORDER_DIM }))
                    .bg(rgb(if selected {
                        SURFACE_ACTIVE
                    } else {
                        SURFACE_ALT
                    }))
                    .text_xs()
                    .text_color(rgb(if selected { ACCENT } else { TEXT_MUTED }))
                    .cursor_pointer()
                    .on_click(cx.listener(move |app, _, _, cx| {
                        if app.chart_timeframe != timeframe {
                            app.chart_timeframe = timeframe;
                            app.reset_chart_view();
                            cx.notify();
                        }
                    }))
                    .child(timeframe_label(timeframe)),
            );
        }
        tabs
    }

    fn render_market_canvas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if let Some(index) = self.selected_arc() {
            let timeframe = self.chart_timeframe;
            let series = index.series(timeframe);
            let value = index
                .reference
                .value_f64()
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "AWAITING VALUE".into());
            div()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .p_4()
                        .flex()
                        .items_start()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::BOLD)
                                        .child(SharedString::new(
                                            index.instrument.display_name.clone(),
                                        )),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(TEXT_DIM))
                                        .child(format!(
                                            "seq {} / {} {} bars / series rev {}",
                                            index.reference.meta.journal_sequence.get(),
                                            timeframe_label(timeframe),
                                            series.len,
                                            series.revision
                                        )),
                                ),
                        )
                        .child(self.render_timeframe_tabs(cx))
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(ACCENT))
                                .child(value),
                        ),
                )
                .child(
                    div()
                        .m_3()
                        .mt_0()
                        .flex_1()
                        .rounded_lg()
                        .border_1()
                        .border_color(rgb(BORDER_DIM))
                        .bg(rgb(0x111514))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if series.len == 0 {
                            empty_state(
                                "Awaiting first completed canonical bar",
                                format!(
                                    "Live values may arrive before the session-anchored {} window closes.",
                                    timeframe_label(timeframe)
                                ),
                                index.reference.meta.availability,
                            )
                            .into_any_element()
                        } else {
                            div()
                                .id("canonical-native-chart")
                                .size_full()
                                .min_h_0()
                                .min_w_0()
                                .relative()
                                .cursor(CursorStyle::Crosshair)
                                .on_mouse_move(cx.listener(
                                    |app, event: &MouseMoveEvent, _, cx| {
                                        let cursor_changed = app.chart_cursor != Some(event.position);
                                        let mut redraw = cursor_changed;
                                        if event.dragging() {
                                            if let Some(previous) = app.chart_drag_position {
                                                let delta_x = f32::from(event.position.x - previous.x);
                                                if delta_x != 0.0 {
                                                    let slots = app.selected().map_or(0, |index| {
                                                        canonical_slot_count(
                                                            index,
                                                            app.chart_timeframe,
                                                        )
                                                    });
                                                    app.chart_viewport.pan_pixels(delta_x, slots);
                                                    redraw = true;
                                                }
                                            }
                                            app.chart_drag_position = Some(event.position);
                                        } else {
                                            app.chart_drag_position = None;
                                        }
                                        app.chart_cursor = Some(event.position);
                                        if redraw {
                                            cx.notify();
                                        }
                                    },
                                ))
                                .on_scroll_wheel(cx.listener(
                                    |app, event: &ScrollWheelEvent, window, cx| {
                                        let slots = app.selected().map_or(0, |index| {
                                            canonical_slot_count(index, app.chart_timeframe)
                                        });
                                        if slots == 0 {
                                            return;
                                        }
                                        let plot_width =
                                            (f32::from(window.viewport_size().width) - 360.0)
                                                .max(320.0);
                                        let anchor = ((f32::from(event.position.x) - 320.0)
                                            / plot_width)
                                            .clamp(0.0, 1.0);
                                        let delta_y =
                                            f32::from(event.delta.pixel_delta(px(18.0)).y);
                                        if delta_y.abs() < f32::EPSILON {
                                            return;
                                        }
                                        app.chart_viewport.zoom(
                                            if delta_y < 0.0 { 1.12 } else { 0.89 },
                                            anchor,
                                            plot_width,
                                            slots,
                                        );
                                        cx.notify();
                                    },
                                ))
                                .child(canonical_index_chart(
                                    Arc::clone(&index),
                                    timeframe,
                                    self.chart_viewport,
                                    self.chart_cursor,
                                ))
                                .into_any_element()
                        }),
                )
                .into_any_element()
        } else {
            empty_state(
                "Desk is waiting for configuration",
                "Massive entitlements, verified tickers, and a session calendar are required. No demo chart will be substituted.",
                self.snapshot.desk.availability,
            )
            .into_any_element()
        };
        panel(
            "MARKET CANVAS",
            format!(
                "office gen {} / desk gen {}",
                self.snapshot.generation.0, self.snapshot.desk.generation
            ),
            body,
        )
        .w_0()
        .flex_1()
        .min_w_0()
    }

    fn render_venue_truth(&self) -> impl IntoElement {
        let venue = &self.snapshot.venue;
        let venue_state = venue.availability;
        let quote = self.selected_instrument.and_then(|id| venue.quote(id));
        let summary = if let Some(account) = venue.account.as_deref() {
            div()
                .rounded_lg()
                .border_1()
                .border_color(rgb(BORDER_DIM))
                .bg(rgb(SURFACE_ALT))
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .child(account.name.to_string()),
                        )
                        .child(availability_badge(venue_state)),
                )
                .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                    "{} / account {}",
                    account.currency, account.acc_num
                )))
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(ACCENT))
                        .child(format_money_micros(
                            account.projected_balance_micros,
                            &account.currency,
                        )),
                )
                .into_any_element()
        } else {
            empty_state(
                "TradeLocker configuration pending",
                venue.health.to_string(),
                venue_state,
            )
            .into_any_element()
        };
        let body = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child(summary)
            .child(readiness_row(
                "Selected quote",
                quote.map_or_else(
                    || "awaiting coherent epoch".into(),
                    |quote| {
                        format!(
                            "{} / {}",
                            format_scaled(quote.bid_scaled, quote.price_scale),
                            format_scaled(quote.ask_scaled, quote.price_scale)
                        )
                    },
                ),
                venue_state,
            ))
            .child(readiness_row(
                "Instrument contracts",
                format!("{} / 6 admitted", venue.contracts.len()),
                if venue.contracts.len() == 6 {
                    venue_state
                } else {
                    Availability::AwaitingFirstReceipt
                },
            ))
            .child(readiness_row(
                "Reconciliation epoch",
                if venue.generation == 0 {
                    "awaiting first complete refresh".into()
                } else {
                    format!(
                        "generation {} / {} open positions",
                        venue.generation,
                        venue.positions.len()
                    )
                },
                venue_state,
            ))
            .child(readiness_row(
                "Unknown venue objects",
                format!(
                    "{} orders / {} positions",
                    venue.unknown_orders.len(),
                    venue.unknown_positions.len()
                ),
                if venue.unknown_orders.is_empty()
                    && venue.unknown_positions.is_empty()
                    && venue.generation > 0
                {
                    Availability::Ready
                } else if venue.generation == 0 {
                    Availability::AwaitingFirstReceipt
                } else {
                    Availability::Invalid
                },
            ));
        panel("VENUE TRUTH", "TradeLocker / read-only", body)
            .w(px(320.0))
            .flex_shrink_0()
    }

    fn render_pipeline_strip(&self, compact: bool) -> impl IntoElement {
        div()
            .h(px(if compact { 132.0 } else { 92.0 }))
            .flex_shrink_0()
            .grid()
            .grid_cols(if compact { 2 } else { 4 })
            .gap_3()
            .child(pipeline_card(
                "MARKET DATA",
                availability_label(self.snapshot.desk.availability),
                format_sequence(self.snapshot.desk.last_sequence),
                self.snapshot.desk.availability,
            ))
            .child(pipeline_card(
                "MACRO",
                availability_label(self.snapshot.macro_office.availability),
                format_sequence(self.snapshot.macro_office.last_sequence),
                self.snapshot.macro_office.availability,
            ))
            .child(pipeline_card(
                "FUND / VENUE",
                availability_label(self.snapshot.venue.availability),
                if self.snapshot.venue.generation == 0 {
                    "coherent epoch pending".into()
                } else {
                    format!("venue epoch {}", self.snapshot.venue.generation)
                },
                self.snapshot.venue.availability,
            ))
            .child(pipeline_card(
                "LEDGER",
                availability_label(self.snapshot.ledger.availability),
                format!("{} UI publications", self.ui_updates),
                self.snapshot.ledger.availability,
            ))
    }
}

fn format_money_micros(value: i64, currency: &str) -> String {
    let amount = value as f64 / 1_000_000.0;
    if currency.is_empty() {
        format!("{amount:+.2}")
    } else {
        format!("{amount:+.2} {currency}")
    }
}

fn format_quantity_micros(value: i64) -> String {
    let quantity = value as f64 / 1_000_000.0;
    format!("{quantity:+.2}")
}

fn format_scaled(value: i64, scale: i64) -> String {
    if scale <= 0 {
        return "—".into();
    }
    format!("{:.2}", value as f64 / scale as f64)
}
