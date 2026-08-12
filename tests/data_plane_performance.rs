use northstar_index_fund::data_plane::event::{CanonicalBatch, CanonicalEvent, TimeQuality};
use northstar_index_fund::data_plane::ids::{BatchId, InstrumentId, ReceiptId, SourceId, StreamId};
use northstar_index_fund::data_plane::journal::{JournalWriter, MappedJournal};
use northstar_index_fund::data_plane::replay::{
    CanonicalConsumer, EventBatchRef, ReplayConfig, ReplayEngine,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Counter(usize);

impl CanonicalConsumer for Counter {
    fn apply_batch(&mut self, batch: EventBatchRef<'_>) {
        self.0 += batch.events.len();
        black_box(batch.events);
    }
}

#[test]
#[ignore = "manual release-mode data-plane performance gate"]
fn append_and_replay_one_hundred_thousand_events() {
    const COUNT: u64 = 100_000;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("perf.nsj");
    let mut writer = JournalWriter::create(&path, 0).unwrap();
    let mut batch = CanonicalBatch::new(BatchId(1));
    batch.events.reserve(COUNT as usize);
    for id in 1..=COUNT {
        batch.push(
            CanonicalEvent::index_value(
                SourceId(1),
                StreamId(1),
                InstrumentId(1),
                ReceiptId(1),
                id,
                id as i64,
                id as i64 + 1,
                2_000_000 + id as i64,
                TimeQuality::ObservedLive,
            )
            .unwrap(),
        );
    }

    let append_started = Instant::now();
    writer.append_batch(&mut batch).unwrap();
    let append_elapsed = append_started.elapsed();
    drop(writer);

    let replay_started = Instant::now();
    let mapped = MappedJournal::open(&path).unwrap();
    let mut counter = Counter::default();
    let report = ReplayEngine::new(&mapped).run(ReplayConfig::default(), &mut counter);
    let replay_elapsed = replay_started.elapsed();
    eprintln!(
        "events={COUNT} bytes={} append={append_elapsed:?} replay={replay_elapsed:?}",
        std::fs::metadata(&path).unwrap().len()
    );
    assert_eq!(counter.0, COUNT as usize);
    assert_eq!(report.event_count, COUNT as usize);
    assert!(append_elapsed < Duration::from_secs(5));
    assert!(replay_elapsed < Duration::from_secs(2));
}
