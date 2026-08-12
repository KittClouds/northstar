use gpui::{px, size, App, AppContext as _, Application, Bounds, WindowBounds, WindowOptions};
use gpui_component::Root;
use northstar_index_fund::data_plane::bars::MINUTE_NS;
use northstar_index_fund::data_plane::calendar::{Session, SessionCalendar, SessionSegment};
use northstar_index_fund::data_plane::event::{CanonicalBatch, CanonicalEvent, TimeQuality};
use northstar_index_fund::data_plane::ids::{
    instruments, BatchId, CalendarId, CatalogVersion, DerivationVersion, ReceiptId, SourceId,
    StreamId,
};
use northstar_index_fund::data_plane::journal::JournalWriter;
use northstar_index_fund::data_plane::replay::LivePublisher;
use northstar_index_fund::operating::{
    MarketInstrument, MarketProjector, MarketSnapshotBridge, NorthstarRuntime, OperatingMode,
};
use northstar_index_fund::OperatingApp;
use std::sync::Arc;

const SOURCE: SourceId = SourceId(10);
const STREAM: StreamId = StreamId(1);
const PRICE_SCALE: i64 = 10_000;
const SESSION_MINUTES: i64 = 600;
const OBSERVATION_MINUTES: i64 = 520;

fn main() {
    let runtime = canonical_replay_runtime();
    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);
        cx.on_window_closed(|cx| cx.quit()).detach();
        let bounds = Bounds::centered(None, size(px(1_440.0), px(780.0)), cx);
        let app_runtime = Arc::clone(&runtime);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title("Northstar Index Office / Canonical Replay Lab");
                let view = cx.new(|cx| OperatingApp::new(Arc::clone(&app_runtime), window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("open Northstar canonical replay window");
    });
}

fn canonical_replay_runtime() -> Arc<NorthstarRuntime> {
    let (runtime, snapshots) = NorthstarRuntime::empty(OperatingMode::OfflineReplay);
    let calendar = Arc::new(
        SessionCalendar::new(
            CalendarId(1),
            CatalogVersion(1),
            vec![Session {
                instrument: instruments::US100,
                open_ns: 0,
                close_ns: SESSION_MINUTES * MINUTE_NS,
                segment: SessionSegment::MainReference,
                flags: 0,
            }],
        )
        .expect("valid replay calendar"),
    );
    let projector = MarketProjector::new(
        OperatingMode::OfflineReplay,
        calendar,
        DerivationVersion(1),
        vec![
            MarketInstrument::new(instruments::US100, "NASDAQ 100", "I:NDX", PRICE_SCALE)
                .expect("valid replay instrument"),
        ],
    )
    .expect("valid replay projector");

    let journal_path = std::env::temp_dir().join(format!(
        "northstar-canonical-replay-{}.nsj",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&journal_path);
    let writer = JournalWriter::create(&journal_path, 0).expect("create replay journal");
    let bridge = MarketSnapshotBridge::new(projector, snapshots);
    let mut live = LivePublisher::new(writer, bridge);
    let mut batch = CanonicalBatch::new(BatchId(1));
    batch.events.reserve(OBSERVATION_MINUTES as usize);
    for minute in 0..OBSERVATION_MINUTES {
        batch.push(replay_value(minute));
    }
    live.commit_and_publish(&mut batch)
        .expect("publish canonical replay batch");
    let (writer, bridge) = live.into_parts();
    drop(writer);
    drop(bridge);
    std::fs::remove_file(journal_path).expect("remove transient replay journal");
    runtime
}

fn replay_value(minute: i64) -> CanonicalEvent {
    let cycle = minute % 64;
    let triangle = if cycle < 32 { cycle } else { 64 - cycle };
    let pulse = if minute % 113 < 9 { -180_000 } else { 0 };
    let value = 198_000_000 + minute * 4_200 + triangle * 26_000 + pulse;
    let timestamp_ns = minute * MINUTE_NS;
    CanonicalEvent::index_value(
        SOURCE,
        STREAM,
        instruments::US100,
        ReceiptId(1),
        minute as u64 + 1,
        timestamp_ns,
        timestamp_ns + 1,
        value,
        TimeQuality::VerifiedPublication,
    )
    .expect("valid replay value")
}
