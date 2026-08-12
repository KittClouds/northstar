use super::*;
use crate::operating::{MacroFeedSnapshot, MacroFeedState};

pub(super) const MACRO_FEEDS: [(SourceId, crate::data_plane::ids::StreamId); 11] = [
    (BLS_SOURCE, BLS_TIMESERIES_STREAM),
    (BLS_SOURCE, BLS_RELEASE_CALENDAR_STREAM),
    (CFTC_SOURCE, CFTC_TFF_STREAM),
    (EUROSTAT_SOURCE, EUROSTAT_STATISTICS_STREAM),
    (FRED_SOURCE, FRED_OBSERVATIONS_STREAM),
    (BEA_SOURCE, BEA_NIPA_STREAM),
    (CENSUS_SOURCE, CENSUS_MARTS_STREAM),
    (ECB_SOURCE, ECB_POLICY_RATES_STREAM),
    (ONS_SOURCE, ONS_TIMESERIES_STREAM),
    (BOE_SOURCE, BOE_RATES_STREAM),
    (BOJ_SOURCE, BOJ_TIMESERIES_STREAM),
];

#[derive(Clone, Debug)]
pub(super) struct FeedFailure {
    pub detail: Arc<str>,
    pub next_retry_ns: i64,
}

impl MacroPlane {
    pub fn set_feed_configured(
        &mut self,
        source: SourceId,
        stream: crate::data_plane::ids::StreamId,
        configured: bool,
    ) {
        if configured {
            self.unconfigured_feeds.remove(&(source, stream));
        } else {
            self.unconfigured_feeds.insert((source, stream));
            self.feed_failures.remove(&(source, stream));
        }
    }

    pub(in crate::operating) fn clear_feed_failure(
        &mut self,
        source: SourceId,
        stream: crate::data_plane::ids::StreamId,
    ) {
        self.feed_failures.remove(&(source, stream));
    }

    pub fn failure_snapshot(
        &mut self,
        source: SourceId,
        stream: crate::data_plane::ids::StreamId,
        message: impl Into<Arc<str>>,
        next_refresh_ns: i64,
    ) -> Arc<MacroSnapshot> {
        let message = message.into();
        self.feed_failures.insert(
            (source, stream),
            FeedFailure {
                detail: Arc::clone(&message),
                next_retry_ns: next_refresh_ns,
            },
        );
        self.published_snapshot(Availability::Stale, message, next_refresh_ns)
    }

    pub(in crate::operating) fn published_snapshot(
        &self,
        availability: Availability,
        health: impl Into<Arc<str>>,
        next_refresh_ns: i64,
    ) -> Arc<MacroSnapshot> {
        let mut snapshot = self
            .projector
            .snapshot(availability, health, next_refresh_ns);
        snapshot.receipt_count = self.raw_receipt_count;
        snapshot.feeds = MACRO_FEEDS
            .iter()
            .map(|&(source, stream)| {
                let key = (source, stream);
                let (state, detail, retry) = if self.unconfigured_feeds.contains(&key) {
                    (
                        MacroFeedState::NotConfigured,
                        Arc::from("credential not configured"),
                        0,
                    )
                } else if let Some(failure) = self.feed_failures.get(&key) {
                    (
                        MacroFeedState::Rejected,
                        Arc::clone(&failure.detail),
                        failure.next_retry_ns,
                    )
                } else if self.projector.has_feed(source, stream) {
                    (
                        MacroFeedState::Current,
                        Arc::from("canonical publication present"),
                        0,
                    )
                } else if self.last_receipts.contains_key(&key) {
                    (
                        MacroFeedState::Rejected,
                        Arc::from("raw receipt retained; canonical publication absent"),
                        0,
                    )
                } else {
                    (
                        MacroFeedState::Awaiting,
                        Arc::from("awaiting first transport receipt"),
                        0,
                    )
                };
                MacroFeedSnapshot {
                    source,
                    stream,
                    state,
                    detail,
                    next_retry_ns: retry,
                }
            })
            .collect();
        Arc::new(snapshot)
    }
}
