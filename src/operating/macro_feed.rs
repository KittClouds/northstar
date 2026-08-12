use crate::data_plane::ids::{SourceId, StreamId};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroFeedState {
    Awaiting,
    Current,
    NotConfigured,
    Rejected,
}

#[derive(Clone, Debug)]
pub struct MacroFeedSnapshot {
    pub source: SourceId,
    pub stream: StreamId,
    pub state: MacroFeedState,
    pub detail: Arc<str>,
    pub next_retry_ns: i64,
}
