use super::ledger_commands::append_automatic_event;
use super::massive_source::{open_massive_source, unavailable_snapshot};
use super::official_sources::{
    bea_supervisor, bls_calendar_supervisor, bls_supervisor, boe_supervisor, boj_supervisor,
    census_supervisor, cftc_supervisor, ecb_supervisor, eurostat_supervisor, fred_supervisor,
    ons_supervisor, BEA_FAILURE_RETRY, BLS_CALENDAR_FAILURE_RETRY, BLS_FAILURE_RETRY,
    BOE_FAILURE_RETRY, BOJ_FAILURE_RETRY, CENSUS_FAILURE_RETRY, CFTC_FAILURE_RETRY,
    ECB_FAILURE_RETRY, EUROSTAT_FAILURE_RETRY, FRED_FAILURE_RETRY, ONS_FAILURE_RETRY,
};
use super::operator_ledger::append_operator_event;
use super::snapshot::{
    OfficeSnapshot, OfficeSnapshotPort, OperatingMode, SnapshotPublisher, SnapshotStore,
    SnapshotSubscription,
};
use super::tradelocker_source::open_tradelocker_source;
use super::LedgerProjector;
use super::{
    AutomaticLedgerEventCommand, FetchedBeaReceipt, FetchedBlsCalendarReceipt, FetchedBoeReceipt,
    FetchedBojReceipt, FetchedCensusReceipt, FetchedCftcReceipt, FetchedEcbReceipt,
    FetchedEurostatReceipt, FetchedFredReceipt, FetchedMacroReceipt, FetchedOnsReceipt,
    MacroIngestReport, MacroPlane, MacroPlaneError, MassiveMarketError, MassiveMarketPlane,
    OperatorLedgerCommand, BEA_REFRESH_INTERVAL_NS, BLS_CALENDAR_REFRESH_INTERVAL_NS,
    BLS_REFRESH_INTERVAL_NS, BOE_REFRESH_INTERVAL_NS, BOJ_REFRESH_INTERVAL_NS,
    CENSUS_REFRESH_INTERVAL_NS, CFTC_REFRESH_INTERVAL_NS, ECB_REFRESH_INTERVAL_NS,
    EUROSTAT_REFRESH_INTERVAL_NS, FRED_REFRESH_INTERVAL_NS, ONS_REFRESH_INTERVAL_NS,
};
use crate::data_plane::ids::SourceId;
use crate::data_plane::providers::bea::{BEA_NIPA_STREAM, BEA_SOURCE};
use crate::data_plane::providers::bls::{BLS_SOURCE, BLS_TIMESERIES_STREAM};
use crate::data_plane::providers::bls_calendar::BLS_RELEASE_CALENDAR_STREAM;
use crate::data_plane::providers::boe::{BOE_RATES_STREAM, BOE_SOURCE};
use crate::data_plane::providers::boj::{BOJ_SOURCE, BOJ_TIMESERIES_STREAM};
use crate::data_plane::providers::census::{CENSUS_MARTS_STREAM, CENSUS_SOURCE};
use crate::data_plane::providers::cftc::{CFTC_SOURCE, CFTC_TFF_STREAM};
use crate::data_plane::providers::ecb::{ECB_POLICY_RATES_STREAM, ECB_SOURCE};
use crate::data_plane::providers::eurostat::{EUROSTAT_SOURCE, EUROSTAT_STATISTICS_STREAM};
use crate::data_plane::providers::fred::{FRED_OBSERVATIONS_STREAM, FRED_SOURCE};
use crate::data_plane::providers::massive_rest::MassiveRestSnapshotAdapter;
use crate::data_plane::providers::ons::{ONS_SOURCE, ONS_TIMESERIES_STREAM};
use crate::data_plane::providers::tradelocker::{TradeLockerDecoder, VenueEpochData};
use crate::data_plane::providers::tradelocker_rest::TradeLockerReadClient;
use crate::data_plane::receipt::RawReceipt;
use crate::data_plane::source::{SourceAdapter, SourceError};
use crate::ledger::{LedgerError, LedgerMappedStore, LedgerStore};
use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender, TrySendError};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use thiserror::Error;

mod macro_commands;
mod support;
use macro_commands::handle_macro_command;
pub(in crate::operating) use macro_commands::MacroRuntimeCommand;
use support::{
    massive_supervisor, publish_ledger_failure, publish_macro_result, tradelocker_supervisor,
};

const LEDGER_QUEUE_CAPACITY: usize = 64;
const MASSIVE_SNAPSHOT_INTERVAL: Duration = Duration::from_secs(60);
const MASSIVE_FAILURE_RETRY: Duration = Duration::from_secs(60);
const TRADELOCKER_REFRESH_INTERVAL: Duration = Duration::from_secs(60);
const TRADELOCKER_FAILURE_RETRY: Duration = Duration::from_secs(30);

pub trait LedgerCommandPort: Send + Sync {
    fn submit_operator_event(
        &self,
        command: OperatorLedgerCommand,
    ) -> Result<(), LedgerSubmitError>;
    fn submit_automatic_event(
        &self,
        command: AutomaticLedgerEventCommand,
    ) -> Result<(), LedgerSubmitError>;
}

pub(super) enum RuntimeCommand {
    OperatorEvent(OperatorLedgerCommand),
    AutomaticEvent(AutomaticLedgerEventCommand),
    Macro(MacroRuntimeCommand),
    MassiveRefresh(Vec<RawReceipt>),
    DeskFailure(super::Availability),
    TradeLockerEpoch(VenueEpochData),
    TradeLockerFailure(super::Availability, Arc<str>),
}

pub(super) struct MacroRefresh<T> {
    pub(super) payload: T,
    pub(super) next_refresh_ns: i64,
    pub(super) acknowledgement: Sender<bool>,
}

/// Production composition root for immutable operating state.
///
/// Provider supervision and domain projectors are attached incrementally. An
/// empty runtime is still truthful: it exposes unavailable states and never
/// fills missing authorities with fixture values.
pub struct NorthstarRuntime {
    mode: OperatingMode,
    snapshots: Arc<SnapshotStore>,
    commands: Option<Sender<RuntimeCommand>>,
    state_thread: std::sync::Mutex<Option<JoinHandle<()>>>,
    source_cancels: Vec<Sender<()>>,
    source_threads: std::sync::Mutex<Vec<JoinHandle<()>>>,
}

impl NorthstarRuntime {
    pub fn empty(mode: OperatingMode) -> (Arc<Self>, SnapshotPublisher) {
        let (snapshots, publisher) = SnapshotStore::new(OfficeSnapshot::empty(mode));
        let runtime = Arc::new(Self {
            mode,
            snapshots,
            commands: None,
            state_thread: std::sync::Mutex::new(None),
            source_cancels: Vec::new(),
            source_threads: std::sync::Mutex::new(Vec::new()),
        });
        (runtime, publisher)
    }

    /// Opens or creates the local durable Ledger and gives its single writer
    /// ownership of the snapshot publisher. Provider supervisors will join
    /// this same composition thread as their transports are authorized.
    pub fn with_ledger(
        mode: OperatingMode,
        root: impl AsRef<Path>,
    ) -> Result<Arc<Self>, LedgerRuntimeError> {
        let root = root.as_ref();
        std::fs::create_dir_all(root).map_err(LedgerRuntimeError::Io)?;
        let hot_path = root.join("ledger.hot");
        let cold_path = root.join("ledger.body");
        let mut store = open_or_create_ledger(&hot_path, &cold_path)?;
        let mapped = LedgerMappedStore::open(&hot_path, &cold_path)?;
        let mut projector = LedgerProjector::from_mapped(&mapped);
        drop(mapped);
        let current_ns = now_ns();
        let mut macro_plane = MacroPlane::open_or_create(root, current_ns)?;
        for material in macro_plane.take_recovered_material() {
            let command = AutomaticLedgerEventCommand::from_macro_batch(material);
            append_automatic_event(&mut store, &mut projector, &command, current_ns)
                .map_err(|error| LedgerRuntimeError::Automatic(error.to_string()))?;
        }
        let initial_bls_refresh_ns = macro_plane.next_refresh_for(
            BLS_SOURCE,
            BLS_TIMESERIES_STREAM,
            BLS_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_bls_calendar_refresh_ns = macro_plane.next_refresh_for(
            BLS_SOURCE,
            BLS_RELEASE_CALENDAR_STREAM,
            BLS_CALENDAR_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_cftc_refresh_ns = macro_plane.next_refresh_for(
            CFTC_SOURCE,
            CFTC_TFF_STREAM,
            CFTC_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_eurostat_refresh_ns = macro_plane.next_refresh_for(
            EUROSTAT_SOURCE,
            EUROSTAT_STATISTICS_STREAM,
            EUROSTAT_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_fred_refresh_ns = macro_plane.next_refresh_for(
            FRED_SOURCE,
            FRED_OBSERVATIONS_STREAM,
            FRED_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_bea_refresh_ns = macro_plane.next_refresh_for(
            BEA_SOURCE,
            BEA_NIPA_STREAM,
            BEA_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_census_refresh_ns = macro_plane.next_refresh_for(
            CENSUS_SOURCE,
            CENSUS_MARTS_STREAM,
            CENSUS_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_ecb_refresh_ns = macro_plane.next_refresh_for(
            ECB_SOURCE,
            ECB_POLICY_RATES_STREAM,
            ECB_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_ons_refresh_ns = macro_plane.next_refresh_for(
            ONS_SOURCE,
            ONS_TIMESERIES_STREAM,
            ONS_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_boe_refresh_ns = macro_plane.next_refresh_for(
            BOE_SOURCE,
            BOE_RATES_STREAM,
            BOE_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let initial_boj_refresh_ns = macro_plane.next_refresh_for(
            BOJ_SOURCE,
            BOJ_TIMESERIES_STREAM,
            BOJ_REFRESH_INTERVAL_NS,
            current_ns,
        );
        let network_enabled = !cfg!(test)
            && std::env::var_os("NORTHSTAR_DISABLE_NETWORK").as_deref()
                != Some(std::ffi::OsStr::new("1"));
        let fred_api_key = std::env::var("NORTHSTAR_FRED_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty());
        let bea_api_key = std::env::var("NORTHSTAR_BEA_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty());
        let census_api_key = std::env::var("NORTHSTAR_CENSUS_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty());
        macro_plane.set_feed_configured(
            FRED_SOURCE,
            FRED_OBSERVATIONS_STREAM,
            network_enabled && fred_api_key.is_some(),
        );
        macro_plane.set_feed_configured(
            BEA_SOURCE,
            BEA_NIPA_STREAM,
            network_enabled && bea_api_key.is_some(),
        );
        macro_plane.set_feed_configured(
            CENSUS_SOURCE,
            CENSUS_MARTS_STREAM,
            network_enabled && census_api_key.is_some(),
        );
        let recovered_macro = macro_plane.recovered_snapshot(current_ns);

        let (massive_plane, massive_adapter, initial_desk) = if network_enabled {
            match open_massive_source(root, mode, current_ns) {
                Ok(startup) => (
                    Some(startup.plane),
                    Some(startup.adapter),
                    Some(startup.initial_snapshot),
                ),
                Err(error) => {
                    eprintln!("NORTHSTAR_MASSIVE_UNAVAILABLE error={error}");
                    (None, None, Some(unavailable_snapshot(error.availability())))
                }
            }
        } else {
            (None, None, None)
        };
        let (tradelocker_client, tradelocker_decoder, initial_venue) = if network_enabled {
            match open_tradelocker_source(root) {
                Ok(startup) => (
                    Some(startup.client),
                    Some(startup.decoder),
                    startup.initial_snapshot,
                ),
                Err(error) => {
                    eprintln!("NORTHSTAR_TRADELOCKER_UNAVAILABLE error={error}");
                    (
                        None,
                        None,
                        Arc::new(super::VenueSnapshot::unavailable(
                            error.availability(),
                            format!("TradeLocker unavailable: {error}"),
                        )),
                    )
                }
            }
        } else {
            (
                None,
                None,
                Arc::new(super::VenueSnapshot::unavailable(
                    super::Availability::NotConfigured,
                    "network disabled",
                )),
            )
        };

        let (snapshots, mut publisher) = SnapshotStore::new(OfficeSnapshot::empty(mode));
        publisher.publish_ledger(Arc::new(projector.snapshot()));
        publisher.publish_macro(recovered_macro);
        if let Some(initial_desk) = initial_desk {
            publisher.publish_desk(initial_desk);
        }
        publisher.publish_venue(Arc::clone(&initial_venue));
        let (command_tx, command_rx) = bounded(LEDGER_QUEUE_CAPACITY);
        let state_thread = std::thread::Builder::new()
            .name("northstar-state-writer".into())
            .spawn(move || {
                state_worker(
                    store,
                    projector,
                    macro_plane,
                    massive_plane,
                    initial_venue,
                    publisher,
                    command_rx,
                )
            })
            .map_err(LedgerRuntimeError::Io)?;
        let mut source_cancels = Vec::with_capacity(13);
        let mut source_threads = Vec::with_capacity(13);
        if network_enabled {
            if let Some(adapter) = massive_adapter {
                let (massive_cancel_tx, massive_cancel_rx) = bounded(1);
                let massive_commands = command_tx.clone();
                source_threads.push(
                    std::thread::Builder::new()
                        .name("northstar-massive-source".into())
                        .spawn(move || {
                            massive_supervisor(adapter, massive_commands, massive_cancel_rx)
                        })
                        .map_err(LedgerRuntimeError::Io)?,
                );
                source_cancels.push(massive_cancel_tx);
            }
            if let (Some(client), Some(decoder)) = (tradelocker_client, tradelocker_decoder) {
                let (tradelocker_cancel_tx, tradelocker_cancel_rx) = bounded(1);
                let tradelocker_commands = command_tx.clone();
                source_threads.push(
                    std::thread::Builder::new()
                        .name("northstar-tradelocker-source".into())
                        .spawn(move || {
                            tradelocker_supervisor(
                                client,
                                decoder,
                                tradelocker_commands,
                                tradelocker_cancel_rx,
                            )
                        })
                        .map_err(LedgerRuntimeError::Io)?,
                );
                source_cancels.push(tradelocker_cancel_tx);
            }
            let (bls_cancel_tx, bls_cancel_rx) = bounded(1);
            let bls_commands = command_tx.clone();
            source_threads.push(
                std::thread::Builder::new()
                    .name("northstar-bls-source".into())
                    .spawn(move || {
                        bls_supervisor(bls_commands, bls_cancel_rx, initial_bls_refresh_ns)
                    })
                    .map_err(LedgerRuntimeError::Io)?,
            );
            source_cancels.push(bls_cancel_tx);

            let (bls_calendar_cancel_tx, bls_calendar_cancel_rx) = bounded(1);
            let bls_calendar_commands = command_tx.clone();
            source_threads.push(
                std::thread::Builder::new()
                    .name("northstar-bls-calendar-source".into())
                    .spawn(move || {
                        bls_calendar_supervisor(
                            bls_calendar_commands,
                            bls_calendar_cancel_rx,
                            initial_bls_calendar_refresh_ns,
                        )
                    })
                    .map_err(LedgerRuntimeError::Io)?,
            );
            source_cancels.push(bls_calendar_cancel_tx);

            let (cftc_cancel_tx, cftc_cancel_rx) = bounded(1);
            let cftc_commands = command_tx.clone();
            source_threads.push(
                std::thread::Builder::new()
                    .name("northstar-cftc-source".into())
                    .spawn(move || {
                        cftc_supervisor(cftc_commands, cftc_cancel_rx, initial_cftc_refresh_ns)
                    })
                    .map_err(LedgerRuntimeError::Io)?,
            );
            source_cancels.push(cftc_cancel_tx);

            let (eurostat_cancel_tx, eurostat_cancel_rx) = bounded(1);
            let eurostat_commands = command_tx.clone();
            source_threads.push(
                std::thread::Builder::new()
                    .name("northstar-eurostat-source".into())
                    .spawn(move || {
                        eurostat_supervisor(
                            eurostat_commands,
                            eurostat_cancel_rx,
                            initial_eurostat_refresh_ns,
                        )
                    })
                    .map_err(LedgerRuntimeError::Io)?,
            );
            source_cancels.push(eurostat_cancel_tx);

            let (ecb_cancel_tx, ecb_cancel_rx) = bounded(1);
            let ecb_commands = command_tx.clone();
            source_threads.push(
                std::thread::Builder::new()
                    .name("northstar-ecb-source".into())
                    .spawn(move || {
                        ecb_supervisor(ecb_commands, ecb_cancel_rx, initial_ecb_refresh_ns)
                    })
                    .map_err(LedgerRuntimeError::Io)?,
            );
            source_cancels.push(ecb_cancel_tx);

            let (ons_cancel_tx, ons_cancel_rx) = bounded(1);
            let ons_commands = command_tx.clone();
            source_threads.push(
                std::thread::Builder::new()
                    .name("northstar-ons-source".into())
                    .spawn(move || {
                        ons_supervisor(ons_commands, ons_cancel_rx, initial_ons_refresh_ns)
                    })
                    .map_err(LedgerRuntimeError::Io)?,
            );
            source_cancels.push(ons_cancel_tx);

            let (boe_cancel_tx, boe_cancel_rx) = bounded(1);
            let boe_commands = command_tx.clone();
            source_threads.push(
                std::thread::Builder::new()
                    .name("northstar-boe-source".into())
                    .spawn(move || {
                        boe_supervisor(boe_commands, boe_cancel_rx, initial_boe_refresh_ns)
                    })
                    .map_err(LedgerRuntimeError::Io)?,
            );
            source_cancels.push(boe_cancel_tx);

            let (boj_cancel_tx, boj_cancel_rx) = bounded(1);
            let boj_commands = command_tx.clone();
            source_threads.push(
                std::thread::Builder::new()
                    .name("northstar-boj-source".into())
                    .spawn(move || {
                        boj_supervisor(boj_commands, boj_cancel_rx, initial_boj_refresh_ns)
                    })
                    .map_err(LedgerRuntimeError::Io)?,
            );
            source_cancels.push(boj_cancel_tx);

            if let Some(api_key) = fred_api_key {
                let (fred_cancel_tx, fred_cancel_rx) = bounded(1);
                let fred_commands = command_tx.clone();
                source_threads.push(
                    std::thread::Builder::new()
                        .name("northstar-fred-source".into())
                        .spawn(move || {
                            fred_supervisor(
                                fred_commands,
                                fred_cancel_rx,
                                initial_fred_refresh_ns,
                                api_key,
                            )
                        })
                        .map_err(LedgerRuntimeError::Io)?,
                );
                source_cancels.push(fred_cancel_tx);
            }
            if let Some(api_key) = bea_api_key {
                let (bea_cancel_tx, bea_cancel_rx) = bounded(1);
                let bea_commands = command_tx.clone();
                source_threads.push(
                    std::thread::Builder::new()
                        .name("northstar-bea-source".into())
                        .spawn(move || {
                            bea_supervisor(
                                bea_commands,
                                bea_cancel_rx,
                                initial_bea_refresh_ns,
                                api_key,
                            )
                        })
                        .map_err(LedgerRuntimeError::Io)?,
                );
                source_cancels.push(bea_cancel_tx);
            }
            if let Some(api_key) = census_api_key {
                let (census_cancel_tx, census_cancel_rx) = bounded(1);
                let census_commands = command_tx.clone();
                source_threads.push(
                    std::thread::Builder::new()
                        .name("northstar-census-source".into())
                        .spawn(move || {
                            census_supervisor(
                                census_commands,
                                census_cancel_rx,
                                initial_census_refresh_ns,
                                api_key,
                            )
                        })
                        .map_err(LedgerRuntimeError::Io)?,
                );
                source_cancels.push(census_cancel_tx);
            }
        }
        Ok(Arc::new(Self {
            mode,
            snapshots,
            commands: Some(command_tx),
            state_thread: std::sync::Mutex::new(Some(state_thread)),
            source_cancels,
            source_threads: std::sync::Mutex::new(source_threads),
        }))
    }

    #[inline]
    pub const fn mode(&self) -> OperatingMode {
        self.mode
    }
}

impl LedgerCommandPort for NorthstarRuntime {
    fn submit_operator_event(
        &self,
        command: OperatorLedgerCommand,
    ) -> Result<(), LedgerSubmitError> {
        command
            .validate()
            .map_err(LedgerSubmitError::InvalidOperatorEvent)?;
        let sender = self
            .commands
            .as_ref()
            .ok_or(LedgerSubmitError::NotConfigured)?;
        sender
            .try_send(RuntimeCommand::OperatorEvent(command))
            .map_err(|error| match error {
                TrySendError::Full(_) => LedgerSubmitError::Busy,
                TrySendError::Disconnected(_) => LedgerSubmitError::Disconnected,
            })
    }

    fn submit_automatic_event(
        &self,
        command: AutomaticLedgerEventCommand,
    ) -> Result<(), LedgerSubmitError> {
        command
            .validate()
            .map_err(LedgerSubmitError::InvalidAutomaticEvent)?;
        let sender = self
            .commands
            .as_ref()
            .ok_or(LedgerSubmitError::NotConfigured)?;
        sender
            .try_send(RuntimeCommand::AutomaticEvent(command))
            .map_err(|error| match error {
                TrySendError::Full(_) => LedgerSubmitError::Busy,
                TrySendError::Disconnected(_) => LedgerSubmitError::Disconnected,
            })
    }
}

impl Drop for NorthstarRuntime {
    fn drop(&mut self) {
        for cancel in self.source_cancels.drain(..) {
            let _ = cancel.try_send(());
        }
        let source_threads = self
            .source_threads
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for thread in source_threads.drain(..) {
            let _ = thread.join();
        }
        self.commands.take();
        let state_thread = self
            .state_thread
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(thread) = state_thread {
            let _ = thread.join();
        }
    }
}

impl OfficeSnapshotPort for NorthstarRuntime {
    #[inline]
    fn current(&self) -> Arc<OfficeSnapshot> {
        self.snapshots.current()
    }

    #[inline]
    fn subscribe(&self) -> SnapshotSubscription {
        self.snapshots.subscribe()
    }
}

fn open_or_create_ledger(hot: &Path, cold: &Path) -> Result<LedgerStore, LedgerRuntimeError> {
    match (hot.exists(), cold.exists()) {
        (false, false) => Ok(LedgerStore::create(hot, cold, now_ns())?),
        (true, true) => Ok(LedgerStore::open(hot, cold)?),
        _ => Err(LedgerRuntimeError::IncompletePair {
            hot: hot.to_path_buf(),
            cold: cold.to_path_buf(),
        }),
    }
}

fn state_worker(
    mut store: LedgerStore,
    mut projector: LedgerProjector,
    mut macro_plane: MacroPlane,
    mut massive_plane: Option<MassiveMarketPlane>,
    mut venue_snapshot: Arc<super::VenueSnapshot>,
    mut publisher: SnapshotPublisher,
    commands: Receiver<RuntimeCommand>,
) {
    while let Ok(command) = commands.recv() {
        match command {
            RuntimeCommand::OperatorEvent(command) => {
                match append_operator_event(&mut store, &mut projector, &command, now_ns()) {
                    Ok(true) => {
                        publisher.publish_ledger(Arc::new(projector.snapshot()));
                    }
                    Ok(false) => {}
                    Err(error) => {
                        let mut snapshot = projector.snapshot();
                        snapshot.availability = super::Availability::Invalid;
                        snapshot.health =
                            Arc::from(format!("Operator Ledger append failed: {error}"));
                        publisher.publish_ledger(Arc::new(snapshot));
                    }
                }
            }
            RuntimeCommand::AutomaticEvent(command) => {
                match append_automatic_event(&mut store, &mut projector, &command, now_ns()) {
                    Ok(true) => {
                        publisher.publish_ledger(Arc::new(projector.snapshot()));
                    }
                    Ok(false) => {}
                    Err(error) => publish_ledger_failure(
                        &projector,
                        &mut publisher,
                        format!("Automatic Ledger append failed: {error}"),
                    ),
                }
            }
            RuntimeCommand::Macro(command) => handle_macro_command(
                command,
                &mut macro_plane,
                &mut store,
                &mut projector,
                &mut publisher,
            ),
            RuntimeCommand::MassiveRefresh(receipts) => {
                let Some(plane) = massive_plane.as_mut() else {
                    publisher
                        .publish_desk(unavailable_snapshot(super::Availability::NotConfigured));
                    continue;
                };
                match plane.ingest_refresh(receipts) {
                    Ok(report) => {
                        publisher.publish_desk(report.snapshot);
                    }
                    Err(MassiveMarketError::Journal(
                        crate::data_plane::journal::JournalError::DuplicateSourceEvent { .. },
                    )) => {}
                    Err(MassiveMarketError::Decode(
                        crate::data_plane::source::DecodeError::NotEntitled(_),
                    )) => {
                        publisher
                            .publish_desk(plane.status_snapshot(super::Availability::NotEntitled));
                    }
                    Err(MassiveMarketError::Decode(
                        crate::data_plane::source::DecodeError::RateLimited,
                    )) => {
                        publisher.publish_desk(plane.status_snapshot(super::Availability::Stale));
                    }
                    Err(_) => {
                        publisher.publish_desk(plane.status_snapshot(super::Availability::Invalid));
                    }
                }
            }
            RuntimeCommand::DeskFailure(availability) => {
                let snapshot = massive_plane.as_ref().map_or_else(
                    || unavailable_snapshot(availability),
                    |plane| plane.status_snapshot(availability),
                );
                publisher.publish_desk(snapshot);
            }
            RuntimeCommand::TradeLockerEpoch(epoch) => {
                let generation = venue_snapshot.generation.saturating_add(1);
                let candidate = Arc::new(super::VenueSnapshot::from_epoch(generation, epoch));
                let mut failure = None;
                for command in super::venue_ledger::venue_ledger_events(&candidate) {
                    if let Err(error) =
                        append_automatic_event(&mut store, &mut projector, &command, now_ns())
                    {
                        failure = Some(error.to_string());
                        break;
                    }
                }
                if let Some(error) = failure {
                    let mut invalid = projector.snapshot();
                    invalid.availability = super::Availability::Invalid;
                    invalid.health = Arc::from(format!(
                        "TradeLocker epoch Ledger transaction rejected: {error}"
                    ));
                    publisher.publish_ledger(Arc::new(invalid));
                } else {
                    venue_snapshot = candidate;
                    publisher.publish_venue_and_ledger(
                        Arc::clone(&venue_snapshot),
                        Arc::new(projector.snapshot()),
                    );
                }
            }
            RuntimeCommand::TradeLockerFailure(availability, health) => {
                let mut stale = (*venue_snapshot).clone();
                stale.availability = if stale.generation == 0 {
                    availability
                } else {
                    super::Availability::Stale
                };
                stale.health = health;
                venue_snapshot = Arc::new(stale);
                publisher.publish_venue(Arc::clone(&venue_snapshot));
            }
        }
    }
}

fn duration_ns(duration: Duration) -> i64 {
    i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX)
}

fn now_ns() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

#[derive(Debug, Error)]
pub enum LedgerRuntimeError {
    #[error("ledger I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Store(#[from] LedgerError),
    #[error(transparent)]
    Macro(#[from] MacroPlaneError),
    #[error("automatic Ledger reconciliation failed: {0}")]
    Automatic(String),
    #[error("ledger files must exist as a pair: hot={hot:?}, cold={cold:?}")]
    IncompletePair { hot: PathBuf, cold: PathBuf },
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum LedgerSubmitError {
    #[error("Ledger is not configured")]
    NotConfigured,
    #[error("Ledger writer is busy")]
    Busy,
    #[error("Ledger writer disconnected")]
    Disconnected,
    #[error("Operator Ledger event is invalid: {0}")]
    InvalidOperatorEvent(&'static str),
    #[error("Automatic Ledger event is invalid: {0}")]
    InvalidAutomaticEvent(&'static str),
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
