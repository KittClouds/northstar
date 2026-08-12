use crate::data_plane::bars::Timeframe;
use crate::data_plane::ids::JournalSequence;
use crate::operating::{Availability, OperatingMode};
use crate::ui_theme::*;
use gpui::{div, prelude::*, px, rgb, FontWeight, IntoElement, SharedString};

pub(crate) fn readiness_row(
    label: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    state: Availability,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER_DIM))
        .bg(rgb(SURFACE_ALT))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .text_xs()
                .child(label.into())
                .child(div().text_color(rgb(TEXT_DIM)).child(detail.into())),
        )
        .child(availability_badge(state))
}

pub(crate) fn pipeline_card(
    label: &'static str,
    state_label: &'static str,
    detail: String,
    state: Availability,
) -> impl IntoElement {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .px_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(label),
                )
                .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(detail)),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(availability_color(state)))
                .child(state_label),
        )
}

pub(crate) fn status_pill(
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

pub(crate) fn mode_badge(mode: OperatingMode, compact: bool) -> impl IntoElement {
    let (label, color) = match mode {
        OperatingMode::OfflineReplay => {
            (if compact { "REPLAY" } else { "OFFLINE REPLAY" }, LAVENDER)
        }
        OperatingMode::LiveData => (
            if compact {
                "READ ONLY"
            } else {
                "LIVE DATA / READ ONLY"
            },
            ACCENT,
        ),
        OperatingMode::Paper => ("PAPER", WARNING),
        OperatingMode::Live => ("LIVE", DANGER),
    };
    div()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(color))
        .bg(rgb(0x121615))
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(color))
        .child(label)
}

pub(crate) fn availability_badge(state: Availability) -> impl IntoElement {
    let color = availability_color(state);
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER_DIM))
        .bg(rgb(0x121615))
        .text_xs()
        .text_color(rgb(color))
        .child(div().size_1p5().rounded_full().bg(rgb(color)))
        .child(availability_label(state))
}

pub(crate) fn empty_state(
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    state: Availability,
) -> impl IntoElement {
    div()
        .w_full()
        .flex_1()
        .min_h(px(120.0))
        .min_w_0()
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
                .mt_1()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.into()),
        )
        .child(
            div()
                .w_full()
                .max_w(px(520.0))
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(detail.into()),
        )
}

pub(crate) fn panel(
    label: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    body: impl IntoElement,
) -> gpui::Div {
    div()
        .min_h_0()
        .min_w_0()
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
                .child(label.into())
                .child(div().text_color(rgb(TEXT_DIM)).child(detail.into())),
        )
        .child(body)
}

pub(crate) const fn timeframe_label(timeframe: Timeframe) -> &'static str {
    match timeframe {
        Timeframe::M4 => "M4",
        Timeframe::M20 => "M20",
        Timeframe::H2 => "H2",
        Timeframe::H4 => "H4",
    }
}

pub(crate) fn brand_mark() -> impl IntoElement {
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
        .child("N")
}

pub(crate) const fn availability_label(state: Availability) -> &'static str {
    match state {
        Availability::Live => "live",
        Availability::Delayed => "delayed",
        Availability::Stale => "stale",
        Availability::Replaying => "replaying",
        Availability::AwaitingFirstReceipt => "awaiting first receipt",
        Availability::NotConfigured => "not configured",
        Availability::NotEntitled => "not entitled",
        Availability::Disconnected => "disconnected",
        Availability::Invalid => "invalid",
        Availability::Deferred => "deferred",
        Availability::Ready => "ready",
    }
}

pub(crate) const fn availability_short_label(state: Availability) -> &'static str {
    match state {
        Availability::AwaitingFirstReceipt => "waiting",
        Availability::NotConfigured => "off",
        Availability::Ready => "ready",
        other => availability_label(other),
    }
}

pub(crate) const fn availability_color(state: Availability) -> u32 {
    match state {
        Availability::Live | Availability::Ready => ACCENT,
        Availability::Delayed | Availability::Replaying => LAVENDER,
        Availability::Stale | Availability::AwaitingFirstReceipt => WARNING,
        Availability::NotConfigured | Availability::Deferred => TEXT_DIM,
        Availability::NotEntitled | Availability::Disconnected | Availability::Invalid => DANGER,
    }
}

pub(crate) fn format_sequence(sequence: JournalSequence) -> String {
    if sequence == JournalSequence::UNKNOWN {
        "no committed sequence".into()
    } else {
        format!("sequence {}", sequence.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_states_are_explicit_and_never_green() {
        assert_ne!(availability_color(Availability::NotConfigured), ACCENT);
        assert_ne!(availability_color(Availability::Deferred), ACCENT);
        assert_eq!(
            availability_label(Availability::NotEntitled),
            "not entitled"
        );
    }
}
