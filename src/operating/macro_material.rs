use crate::data_plane::event::{CanonicalEvent, EventFlags, EventKind};
use crate::data_plane::ids::{JournalSequence, SourceId};
use smallvec::SmallVec;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroMaterialKind {
    Observations,
    Positioning,
    Provenance,
}

impl MacroMaterialKind {
    pub const fn ledger_label(self) -> &'static str {
        match self {
            Self::Observations => "official macro observations",
            Self::Positioning => "official positioning observations",
            Self::Provenance => "official release and document provenance",
        }
    }
}

/// Compact description of one already-committed canonical macro batch.
///
/// Ledger stores only this operational receipt and the canonical range. It
/// never duplicates every observation into the cold journal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacroMaterialBatch {
    pub source: SourceId,
    pub kind: MacroMaterialKind,
    pub first_sequence: JournalSequence,
    pub last_sequence: JournalSequence,
    pub ts_received_ns: i64,
    pub event_count: u32,
    pub correction_count: u32,
}

impl MacroMaterialBatch {
    pub fn from_events(events: &[CanonicalEvent]) -> Result<Self, MacroMaterialError> {
        let mut batches = Self::partition(events)?;
        if batches.len() != 1 {
            return Err(MacroMaterialError::MixedKind);
        }
        Ok(batches.remove(0))
    }

    pub fn partition(events: &[CanonicalEvent]) -> Result<SmallVec<[Self; 3]>, MacroMaterialError> {
        let first = events.first().ok_or(MacroMaterialError::Empty)?;
        let source = SourceId(first.header.source_id);
        let mut batches = SmallVec::<[Self; 3]>::new();
        for event in events {
            if SourceId(event.header.source_id) != source {
                return Err(MacroMaterialError::MixedSource);
            }
            let kind = material_kind(event.kind()?)?;
            let sequence = event.journal_sequence();
            if sequence == JournalSequence::UNKNOWN {
                return Err(MacroMaterialError::MissingSequence);
            }
            let corrected =
                u32::from(EventFlags(event.header.flags).contains(EventFlags::CORRECTION));
            if let Some(batch) = batches.iter_mut().find(|batch| batch.kind == kind) {
                if sequence <= batch.last_sequence {
                    return Err(MacroMaterialError::NonIncreasingSequence);
                }
                batch.last_sequence = sequence;
                batch.ts_received_ns = batch.ts_received_ns.max(event.header.ts_received_ns);
                batch.event_count = batch.event_count.saturating_add(1);
                batch.correction_count = batch.correction_count.saturating_add(corrected);
            } else {
                batches.push(Self {
                    source,
                    kind,
                    first_sequence: sequence,
                    last_sequence: sequence,
                    ts_received_ns: event.header.ts_received_ns,
                    event_count: 1,
                    correction_count: corrected,
                });
            }
        }
        Ok(batches)
    }
}

fn material_kind(kind: EventKind) -> Result<MacroMaterialKind, MacroMaterialError> {
    match kind {
        EventKind::MacroObservation => Ok(MacroMaterialKind::Observations),
        EventKind::PositioningObservation => Ok(MacroMaterialKind::Positioning),
        EventKind::MacroRelease | EventKind::SourceDocument => Ok(MacroMaterialKind::Provenance),
        _ => Err(MacroMaterialError::UnsupportedKind(kind)),
    }
}

#[derive(Debug, Error)]
pub enum MacroMaterialError {
    #[error("canonical material batch is empty")]
    Empty,
    #[error("canonical material batch mixes sources")]
    MixedSource,
    #[error("canonical material batch mixes event families")]
    MixedKind,
    #[error("canonical material batch contains unsupported event kind {0:?}")]
    UnsupportedKind(EventKind),
    #[error("canonical material batch has no committed sequence")]
    MissingSequence,
    #[error("canonical material batch sequence is not increasing")]
    NonIncreasingSequence,
    #[error("canonical material batch exceeds supported bounds")]
    TooLarge,
    #[error(transparent)]
    Canonical(#[from] crate::data_plane::event::CanonicalError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::event::{CanonicalEvent, TimeQuality};
    use crate::data_plane::ids::{ReceiptId, SeriesId, StreamId};

    fn event(sequence: u64, source: u16, flags: u32) -> CanonicalEvent {
        let mut event = CanonicalEvent::macro_observation(
            SourceId(source),
            StreamId(1),
            SeriesId(1),
            ReceiptId(1),
            sequence,
            1,
            2,
            10,
            20,
            3.0,
            4,
            None,
            TimeQuality::ObservedLive,
        )
        .unwrap();
        event.header.journal_sequence = sequence;
        event.header.flags |= flags;
        event
    }

    #[test]
    fn batch_summary_keeps_range_source_and_corrections() {
        let batch = MacroMaterialBatch::from_events(&[
            event(7, 20, EventFlags::FINAL),
            event(8, 20, EventFlags::CORRECTION),
        ])
        .unwrap();
        assert_eq!(batch.first_sequence, JournalSequence(7));
        assert_eq!(batch.last_sequence, JournalSequence(8));
        assert_eq!(batch.source, SourceId(20));
        assert_eq!(batch.event_count, 2);
        assert_eq!(batch.correction_count, 1);
    }
}
