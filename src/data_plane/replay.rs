use crate::data_plane::event::{CanonicalBatch, CanonicalEvent};
use crate::data_plane::ids::{BatchId, JournalSequence};
use crate::data_plane::journal::{CommitReceipt, JournalError, JournalWriter, MappedJournal};

#[derive(Clone, Copy, Debug)]
pub struct EventBatchRef<'a> {
    pub id: BatchId,
    pub first_sequence: JournalSequence,
    pub events: &'a [CanonicalEvent],
}

pub trait CanonicalConsumer {
    fn apply_batch(&mut self, batch: EventBatchRef<'_>);
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReplayConfig {
    /// Inclusive causal cutoff. Filtering preserves journal order and never
    /// re-sorts late observations by event time.
    pub as_of_effective_ns: Option<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayReport {
    pub source_batches: usize,
    pub dispatched_batches: usize,
    pub event_count: usize,
    pub first_sequence: Option<JournalSequence>,
    pub last_sequence: Option<JournalSequence>,
    pub input_hash: [u8; 32],
}

pub struct ReplayEngine<'a> {
    journal: &'a MappedJournal,
}

impl<'a> ReplayEngine<'a> {
    pub const fn new(journal: &'a MappedJournal) -> Self {
        Self { journal }
    }

    pub fn run<C: CanonicalConsumer>(
        &self,
        config: ReplayConfig,
        consumer: &mut C,
    ) -> ReplayReport {
        let mut hasher = blake3::Hasher::new();
        let mut report = ReplayReport {
            source_batches: 0,
            dispatched_batches: 0,
            event_count: 0,
            first_sequence: None,
            last_sequence: None,
            input_hash: [0; 32],
        };

        for source_batch in self.journal.batches() {
            report.source_batches += 1;
            dispatch_eligible_runs(
                source_batch.id,
                source_batch.events,
                config.as_of_effective_ns,
                consumer,
                &mut hasher,
                &mut report,
            );
        }
        report.input_hash = *hasher.finalize().as_bytes();
        report
    }
}

fn dispatch_eligible_runs<C: CanonicalConsumer>(
    batch_id: BatchId,
    events: &[CanonicalEvent],
    cutoff: Option<i64>,
    consumer: &mut C,
    hasher: &mut blake3::Hasher,
    report: &mut ReplayReport,
) {
    let mut cursor = 0;
    while cursor < events.len() {
        while cursor < events.len() && !eligible(&events[cursor], cutoff) {
            cursor += 1;
        }
        let start = cursor;
        while cursor < events.len() && eligible(&events[cursor], cutoff) {
            cursor += 1;
        }
        if start == cursor {
            continue;
        }
        let run = &events[start..cursor];
        let first = JournalSequence(run[0].header.journal_sequence);
        let last = JournalSequence(run[run.len() - 1].header.journal_sequence);
        consumer.apply_batch(EventBatchRef {
            id: batch_id,
            first_sequence: first,
            events: run,
        });
        hasher.update(bytemuck::cast_slice(run));
        report.dispatched_batches += 1;
        report.event_count += run.len();
        report.first_sequence.get_or_insert(first);
        report.last_sequence = Some(last);
    }
}

#[inline]
fn eligible(event: &CanonicalEvent, cutoff: Option<i64>) -> bool {
    cutoff.is_none_or(|limit| event.header.ts_effective_ns <= limit)
}

/// The live path publishes only after the journal has acknowledged a durable
/// batch. The consumer sees the same `EventBatchRef` shape as replay.
pub struct LivePublisher<C> {
    writer: JournalWriter,
    consumer: C,
}

impl<C: CanonicalConsumer> LivePublisher<C> {
    pub const fn new(writer: JournalWriter, consumer: C) -> Self {
        Self { writer, consumer }
    }

    pub fn commit_and_publish(
        &mut self,
        batch: &mut CanonicalBatch,
    ) -> Result<CommitReceipt, JournalError> {
        let receipt = self.writer.append_batch(batch)?;
        self.consumer.apply_batch(EventBatchRef {
            id: batch.id,
            first_sequence: receipt.first_sequence,
            events: &batch.events,
        });
        Ok(receipt)
    }

    pub fn into_parts(self) -> (JournalWriter, C) {
        (self.writer, self.consumer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::event::{CanonicalEvent, TimeQuality};
    use crate::data_plane::ids::{InstrumentId, ReceiptId, SourceId, StreamId};

    #[derive(Default)]
    struct SequenceConsumer(Vec<u64>);

    impl CanonicalConsumer for SequenceConsumer {
        fn apply_batch(&mut self, batch: EventBatchRef<'_>) {
            self.0.extend(
                batch
                    .events
                    .iter()
                    .map(|event| event.header.journal_sequence),
            );
        }
    }

    fn event(id: u64, effective: i64) -> CanonicalEvent {
        CanonicalEvent::index_value(
            SourceId(1),
            StreamId(1),
            InstrumentId(1),
            ReceiptId(1),
            id,
            effective,
            effective + 1,
            id as i64,
            TimeQuality::VerifiedPublication,
        )
        .unwrap()
    }

    #[test]
    fn replay_is_repeatable_and_as_of_safe() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.nsj");
        let mut writer = JournalWriter::create(&path, 0).unwrap();
        let mut batch = CanonicalBatch::new(BatchId(1));
        batch.push(event(1, 10));
        batch.push(event(2, 30));
        batch.push(event(3, 20));
        writer.append_batch(&mut batch).unwrap();
        drop(writer);
        let mapped = MappedJournal::open(&path).unwrap();

        let config = ReplayConfig {
            as_of_effective_ns: Some(20),
        };
        let mut first = SequenceConsumer::default();
        let first_report = ReplayEngine::new(&mapped).run(config, &mut first);
        let mut second = SequenceConsumer::default();
        let second_report = ReplayEngine::new(&mapped).run(config, &mut second);
        assert_eq!(first.0, [1, 3]);
        assert_eq!(first.0, second.0);
        assert_eq!(first_report, second_report);
    }
}
