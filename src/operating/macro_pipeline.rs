use super::{
    Availability, MacroMaterialBatch, MacroMaterialError, MacroMaterialKind, MacroProjector,
    MacroProjectorError, MacroSnapshot,
};
use crate::data_plane::event::{CanonicalBatch, EventFlags, EventKind, ParticipantClass};
use crate::data_plane::ids::{BatchId, InstrumentId, ReceiptId, SeriesId, SourceId};
use crate::data_plane::journal::{JournalError, JournalWriter, MappedJournal};
use crate::data_plane::providers::bea::{BeaNipaDecoder, BEA_NIPA_STREAM, BEA_SOURCE};
use crate::data_plane::providers::bls::{
    binding_for_code, BlsSeriesDecoder, BLS_SERIES, BLS_SOURCE, BLS_TIMESERIES_STREAM,
};
use crate::data_plane::providers::bls_calendar::{
    BlsReleaseCalendarDecoder, BLS_RELEASE_CALENDAR_STREAM,
};
use crate::data_plane::providers::boe::{BoeRateDecoder, BOE_RATES_STREAM, BOE_SOURCE};
use crate::data_plane::providers::boj::{BojTimeseriesDecoder, BOJ_SOURCE, BOJ_TIMESERIES_STREAM};
use crate::data_plane::providers::census::{
    CensusMartsDecoder, CENSUS_MARTS_STREAM, CENSUS_SOURCE,
};
use crate::data_plane::providers::cftc::{CftcTffDecoder, CFTC_SOURCE, CFTC_TFF_STREAM};
use crate::data_plane::providers::ecb::{
    EcbPolicyRateDecoder, ECB_POLICY_RATES_STREAM, ECB_SOURCE,
};
use crate::data_plane::providers::eurostat::{
    binding_for_dataset, EurostatDecoder, EUROSTAT_BINDINGS, EUROSTAT_SOURCE,
    EUROSTAT_STATISTICS_STREAM,
};
use crate::data_plane::providers::fred::{
    FredSeriesDecoder, FRED_OBSERVATIONS_STREAM, FRED_SOURCE,
};
use crate::data_plane::providers::ons::{OnsTimeseriesDecoder, ONS_SOURCE, ONS_TIMESERIES_STREAM};
use crate::data_plane::receipt::{
    MappedReceiptStore, RawReceipt, RawReceiptRef, ReceiptError, ReceiptStoreWriter,
};
use crate::data_plane::source::{CanonicalDecoder, LicenseClass};
use hashbrown::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

pub const BLS_REFRESH_INTERVAL_NS: i64 = 86_400_000_000_000;
pub const BLS_CALENDAR_REFRESH_INTERVAL_NS: i64 = 86_400_000_000_000;
pub const CFTC_REFRESH_INTERVAL_NS: i64 = 86_400_000_000_000;
pub const EUROSTAT_REFRESH_INTERVAL_NS: i64 = 86_400_000_000_000;
pub const FRED_REFRESH_INTERVAL_NS: i64 = 21_600_000_000_000;
pub const BEA_REFRESH_INTERVAL_NS: i64 = 86_400_000_000_000;
pub const CENSUS_REFRESH_INTERVAL_NS: i64 = 86_400_000_000_000;
pub const ECB_REFRESH_INTERVAL_NS: i64 = 21_600_000_000_000;
pub const ONS_REFRESH_INTERVAL_NS: i64 = 86_400_000_000_000;
pub const BOE_REFRESH_INTERVAL_NS: i64 = 21_600_000_000_000;
pub const BOJ_REFRESH_INTERVAL_NS: i64 = 21_600_000_000_000;

mod bea;
mod bls_calendar;
mod boe;
mod boj;
mod census;
mod ecb;
mod feeds;
mod fetched;
mod fred;
mod ons;
use feeds::FeedFailure;
pub use fetched::*;

pub struct MacroPlane {
    receipt_writer: ReceiptStoreWriter,
    journal_writer: JournalWriter,
    decoder: BlsSeriesDecoder,
    bls_calendar_decoder: BlsReleaseCalendarDecoder,
    cftc_decoder: CftcTffDecoder,
    eurostat_decoder: EurostatDecoder,
    fred_decoder: FredSeriesDecoder,
    bea_decoder: BeaNipaDecoder,
    census_decoder: CensusMartsDecoder,
    ecb_decoder: EcbPolicyRateDecoder,
    ons_decoder: OnsTimeseriesDecoder,
    boe_decoder: BoeRateDecoder,
    boj_decoder: BojTimeseriesDecoder,
    projector: MacroProjector,
    seen_events: HashSet<(u16, u16, u64)>,
    period_vintages: HashMap<(SeriesId, i64), u64>,
    positioning_versions: HashMap<(InstrumentId, ParticipantClass, i64), u64>,
    last_receipts: HashMap<(SourceId, crate::data_plane::ids::StreamId), i64>,
    unconfigured_feeds: HashSet<(SourceId, crate::data_plane::ids::StreamId)>,
    feed_failures: HashMap<(SourceId, crate::data_plane::ids::StreamId), FeedFailure>,
    raw_receipt_count: usize,
    next_receipt_id: u64,
    next_batch_id: u64,
    recovered_material: Vec<MacroMaterialBatch>,
}

impl MacroPlane {
    pub fn open_or_create(
        root: impl AsRef<Path>,
        created_ns: i64,
    ) -> Result<Self, MacroPlaneError> {
        let root = root.as_ref();
        std::fs::create_dir_all(root)?;
        let raw_path = root.join("macro.raw");
        let journal_path = root.join("macro.canonical");
        let receipt_writer = if raw_path.exists() {
            ReceiptStoreWriter::open(&raw_path)?
        } else {
            ReceiptStoreWriter::create(&raw_path, created_ns)?
        };
        let journal_writer = if journal_path.exists() {
            JournalWriter::open(&journal_path)?
        } else {
            JournalWriter::create(&journal_path, created_ns)?
        };

        let raw = MappedReceiptStore::open(&raw_path)?;
        let mut next_receipt_id = 1u64;
        let mut last_receipts = HashMap::new();
        let mut raw_receipt_count = 0usize;
        for receipt in raw.receipts() {
            raw_receipt_count = raw_receipt_count.saturating_add(1);
            next_receipt_id = next_receipt_id.max(receipt.id.get().saturating_add(1));
            last_receipts
                .entry((receipt.source, receipt.stream))
                .and_modify(|received: &mut i64| {
                    *received = (*received).max(receipt.ts_received_ns);
                })
                .or_insert(receipt.ts_received_ns);
        }
        let journal = MappedJournal::open(&journal_path)?;
        let projector = MacroProjector::from_mapped(&journal)?;
        let mut seen_events = HashSet::with_capacity(journal.event_count().saturating_mul(2));
        let mut period_vintages = HashMap::with_capacity(journal.event_count());
        let mut positioning_versions = HashMap::with_capacity(journal.event_count());
        let mut recovered_material = Vec::with_capacity(journal.batch_count());
        for batch in journal.batches() {
            recovered_material.extend(MacroMaterialBatch::partition(batch.events)?);
            for event in batch.events {
                seen_events.insert((
                    event.header.source_id,
                    event.header.stream_id,
                    event.header.source_event_id,
                ));
                match event.kind()? {
                    EventKind::MacroObservation => {
                        period_vintages
                            .insert((event.series_id(), event.values[0]), event.values[3] as u64);
                    }
                    EventKind::PositioningObservation => {
                        positioning_versions.insert(
                            (
                                event.instrument_id(),
                                event.participant_class()?,
                                event.header.ts_event_ns,
                            ),
                            event.header.source_event_id,
                        );
                    }
                    _ => {}
                }
            }
        }
        let next_batch_id = u64::try_from(journal.batch_count())
            .unwrap_or(u64::MAX - 1)
            .saturating_add(1)
            .max(1);
        Ok(Self {
            receipt_writer,
            journal_writer,
            decoder: BlsSeriesDecoder::official(),
            bls_calendar_decoder: BlsReleaseCalendarDecoder::official(),
            cftc_decoder: CftcTffDecoder::official(),
            eurostat_decoder: EurostatDecoder::official(),
            fred_decoder: FredSeriesDecoder::official(),
            bea_decoder: BeaNipaDecoder::official(),
            census_decoder: CensusMartsDecoder::official(),
            ecb_decoder: EcbPolicyRateDecoder::official(),
            ons_decoder: OnsTimeseriesDecoder::official(),
            boe_decoder: BoeRateDecoder::official(),
            boj_decoder: BojTimeseriesDecoder::official(),
            projector,
            seen_events,
            period_vintages,
            positioning_versions,
            last_receipts,
            unconfigured_feeds: HashSet::with_capacity(3),
            feed_failures: HashMap::with_capacity(feeds::MACRO_FEEDS.len()),
            raw_receipt_count,
            next_receipt_id,
            next_batch_id,
            recovered_material,
        })
    }

    pub fn take_recovered_material(&mut self) -> Vec<MacroMaterialBatch> {
        std::mem::take(&mut self.recovered_material)
    }

    pub fn recovered_snapshot(&self, now_ns: i64) -> Arc<MacroSnapshot> {
        let next_refresh_ns = self
            .next_refresh_for(
                BLS_SOURCE,
                BLS_TIMESERIES_STREAM,
                BLS_REFRESH_INTERVAL_NS,
                now_ns,
            )
            .min(self.next_refresh_for(
                BLS_SOURCE,
                BLS_RELEASE_CALENDAR_STREAM,
                BLS_CALENDAR_REFRESH_INTERVAL_NS,
                now_ns,
            ))
            .min(self.next_refresh_for(
                CFTC_SOURCE,
                CFTC_TFF_STREAM,
                CFTC_REFRESH_INTERVAL_NS,
                now_ns,
            ))
            .min(self.next_refresh_for(
                EUROSTAT_SOURCE,
                EUROSTAT_STATISTICS_STREAM,
                EUROSTAT_REFRESH_INTERVAL_NS,
                now_ns,
            ))
            .min(self.next_refresh_for(
                FRED_SOURCE,
                FRED_OBSERVATIONS_STREAM,
                FRED_REFRESH_INTERVAL_NS,
                now_ns,
            ))
            .min(self.next_refresh_for(
                BEA_SOURCE,
                BEA_NIPA_STREAM,
                BEA_REFRESH_INTERVAL_NS,
                now_ns,
            ))
            .min(self.next_refresh_for(
                CENSUS_SOURCE,
                CENSUS_MARTS_STREAM,
                CENSUS_REFRESH_INTERVAL_NS,
                now_ns,
            ))
            .min(self.next_refresh_for(
                ECB_SOURCE,
                ECB_POLICY_RATES_STREAM,
                ECB_REFRESH_INTERVAL_NS,
                now_ns,
            ))
            .min(self.next_refresh_for(
                ONS_SOURCE,
                ONS_TIMESERIES_STREAM,
                ONS_REFRESH_INTERVAL_NS,
                now_ns,
            ))
            .min(self.next_refresh_for(
                BOE_SOURCE,
                BOE_RATES_STREAM,
                BOE_REFRESH_INTERVAL_NS,
                now_ns,
            ))
            .min(self.next_refresh_for(
                BOJ_SOURCE,
                BOJ_TIMESERIES_STREAM,
                BOJ_REFRESH_INTERVAL_NS,
                now_ns,
            ));
        let current = !self.projector.is_empty()
            && now_ns
                <= self
                    .projector
                    .last_received_ns()
                    .saturating_add(BLS_REFRESH_INTERVAL_NS);
        self.published_snapshot(
            if current {
                Availability::Live
            } else {
                Availability::Stale
            },
            if self.projector.is_empty() {
                "Awaiting the first complete BLS refresh"
            } else if current {
                "Recovered current data from the canonical macro journal"
            } else {
                "Recovered stale data from the canonical macro journal"
            },
            next_refresh_ns,
        )
    }

    pub fn next_refresh_for(
        &self,
        source: SourceId,
        stream: crate::data_plane::ids::StreamId,
        interval_ns: i64,
        now_ns: i64,
    ) -> i64 {
        self.last_receipts
            .get(&(source, stream))
            .copied()
            .unwrap_or(0)
            .saturating_add(interval_ns)
            .max(now_ns)
    }

    pub fn ingest_complete_bls_refresh(
        &mut self,
        receipts: Vec<FetchedMacroReceipt>,
        next_refresh_ns: i64,
    ) -> Result<MacroIngestReport, MacroPlaneError> {
        validate_complete_refresh(&receipts)?;
        let raw_receipts = u32::try_from(receipts.len()).map_err(|_| MacroPlaneError::TooLarge)?;
        let mut batch = CanonicalBatch::new(BatchId(self.next_batch_id));
        let mut decoded_events = 0u32;

        for fetched in receipts {
            let receipt_id = ReceiptId(self.next_receipt_id);
            self.next_receipt_id = self.next_receipt_id.saturating_add(1);
            let metadata = format!(
                "GET /publicAPI/v1/timeseries/data/{}",
                fetched.provider_code
            )
            .into_bytes();
            let receipt = RawReceipt {
                id: receipt_id,
                source: BLS_SOURCE,
                stream: BLS_TIMESERIES_STREAM,
                status_code: fetched.status_code,
                content_type: 1,
                ts_started_ns: fetched.ts_started_ns,
                ts_received_ns: fetched.ts_received_ns,
                license: LicenseClass::OpenWithAttribution,
                metadata,
                payload: fetched.payload,
            };
            let payload_hash = self.receipt_writer.append(&receipt)?;
            self.raw_receipt_count = self.raw_receipt_count.saturating_add(1);
            self.last_receipts
                .insert((receipt.source, receipt.stream), receipt.ts_received_ns);
            let stats = self.decoder.decode(
                RawReceiptRef {
                    id: receipt.id,
                    source: receipt.source,
                    stream: receipt.stream,
                    status_code: receipt.status_code,
                    content_type: receipt.content_type,
                    ts_started_ns: receipt.ts_started_ns,
                    ts_received_ns: receipt.ts_received_ns,
                    metadata: &receipt.metadata,
                    payload: &receipt.payload,
                    payload_hash,
                },
                &mut batch,
            )?;
            decoded_events = decoded_events.saturating_add(stats.canonical_events);
        }

        batch.events.retain(|event| {
            !self.seen_events.contains(&(
                event.header.source_id,
                event.header.stream_id,
                event.header.source_event_id,
            ))
        });
        for event in &mut batch.events {
            if event.kind()? == EventKind::MacroObservation {
                let key = (event.series_id(), event.values[0]);
                if let Some(previous) = self.period_vintages.get(&key).copied() {
                    let vintage = event.values[3] as u64;
                    if previous != vintage {
                        event.header.flags |= EventFlags::CORRECTION;
                        event.values[4] = previous as i64;
                    }
                }
            }
        }

        let committed_events =
            u32::try_from(batch.events.len()).map_err(|_| MacroPlaneError::TooLarge)?;
        let material = if committed_events != 0 {
            self.journal_writer.append_batch(&mut batch)?;
            self.projector.apply_batch(&batch.events)?;
            for event in &batch.events {
                self.seen_events.insert((
                    event.header.source_id,
                    event.header.stream_id,
                    event.header.source_event_id,
                ));
                if event.kind()? == EventKind::MacroObservation {
                    self.period_vintages
                        .insert((event.series_id(), event.values[0]), event.values[3] as u64);
                }
            }
            self.next_batch_id = self.next_batch_id.saturating_add(1);
            Some(MacroMaterialBatch::from_events(&batch.events)?)
        } else {
            None
        };

        let snapshot = self.published_snapshot(
            Availability::Live,
            if committed_events == 0 {
                "BLS refresh current / no new observations"
            } else {
                "BLS refresh committed to raw and canonical journals"
            },
            next_refresh_ns,
        );
        Ok(MacroIngestReport {
            raw_receipts,
            decoded_events,
            committed_events,
            material,
            provenance_material: None,
            snapshot,
        })
    }

    pub fn ingest_complete_eurostat_refresh(
        &mut self,
        receipts: Vec<FetchedEurostatReceipt>,
        next_refresh_ns: i64,
    ) -> Result<MacroIngestReport, MacroPlaneError> {
        validate_complete_eurostat_refresh(&receipts)?;
        let raw_receipts = u32::try_from(receipts.len()).map_err(|_| MacroPlaneError::TooLarge)?;
        let mut batch = CanonicalBatch::new(BatchId(self.next_batch_id));
        let mut decoded_events = 0u32;

        for fetched in receipts {
            let receipt_id = ReceiptId(self.next_receipt_id);
            self.next_receipt_id = self.next_receipt_id.saturating_add(1);
            let receipt = RawReceipt {
                id: receipt_id,
                source: EUROSTAT_SOURCE,
                stream: EUROSTAT_STATISTICS_STREAM,
                status_code: fetched.status_code,
                content_type: 1,
                ts_started_ns: fetched.ts_started_ns,
                ts_received_ns: fetched.ts_received_ns,
                license: LicenseClass::OpenWithAttribution,
                metadata: format!("dataset={}", fetched.dataset_code).into_bytes(),
                payload: fetched.payload,
            };
            let payload_hash = self.receipt_writer.append(&receipt)?;
            self.raw_receipt_count = self.raw_receipt_count.saturating_add(1);
            self.last_receipts
                .insert((receipt.source, receipt.stream), receipt.ts_received_ns);
            let stats = self.eurostat_decoder.decode(
                RawReceiptRef {
                    id: receipt.id,
                    source: receipt.source,
                    stream: receipt.stream,
                    status_code: receipt.status_code,
                    content_type: receipt.content_type,
                    ts_started_ns: receipt.ts_started_ns,
                    ts_received_ns: receipt.ts_received_ns,
                    metadata: &receipt.metadata,
                    payload: &receipt.payload,
                    payload_hash,
                },
                &mut batch,
            )?;
            decoded_events = decoded_events.saturating_add(stats.canonical_events);
        }

        batch.events.retain(|event| {
            !self.seen_events.contains(&(
                event.header.source_id,
                event.header.stream_id,
                event.header.source_event_id,
            ))
        });
        for event in &mut batch.events {
            let key = (event.series_id(), event.values[0]);
            if let Some(previous) = self.period_vintages.get(&key).copied() {
                let vintage = event.values[3] as u64;
                if previous != vintage {
                    event.header.flags |= EventFlags::CORRECTION;
                    event.values[4] = previous as i64;
                }
            }
        }
        let committed_events =
            u32::try_from(batch.events.len()).map_err(|_| MacroPlaneError::TooLarge)?;
        let material = if committed_events != 0 {
            self.journal_writer.append_batch(&mut batch)?;
            self.projector.apply_batch(&batch.events)?;
            for event in &batch.events {
                self.seen_events.insert((
                    event.header.source_id,
                    event.header.stream_id,
                    event.header.source_event_id,
                ));
                self.period_vintages
                    .insert((event.series_id(), event.values[0]), event.values[3] as u64);
            }
            self.next_batch_id = self.next_batch_id.saturating_add(1);
            Some(MacroMaterialBatch::from_events(&batch.events)?)
        } else {
            None
        };
        let snapshot = self.published_snapshot(
            Availability::Live,
            if committed_events == 0 {
                "Eurostat refresh current / no new observations"
            } else {
                "Eurostat observations committed to canonical replay"
            },
            next_refresh_ns,
        );
        Ok(MacroIngestReport {
            raw_receipts,
            decoded_events,
            committed_events,
            material,
            provenance_material: None,
            snapshot,
        })
    }

    pub fn ingest_complete_cftc_refresh(
        &mut self,
        fetched: FetchedCftcReceipt,
        next_refresh_ns: i64,
    ) -> Result<MacroIngestReport, MacroPlaneError> {
        if fetched.status_code != 200 {
            return Err(MacroPlaneError::HttpStatus {
                series: "CFTC TFF".into(),
                status: fetched.status_code,
            });
        }
        let receipt_id = ReceiptId(self.next_receipt_id);
        self.next_receipt_id = self.next_receipt_id.saturating_add(1);
        let receipt = RawReceipt {
            id: receipt_id,
            source: CFTC_SOURCE,
            stream: CFTC_TFF_STREAM,
            status_code: fetched.status_code,
            content_type: 1,
            ts_started_ns: fetched.ts_started_ns,
            ts_received_ns: fetched.ts_received_ns,
            license: LicenseClass::OpenWithAttribution,
            metadata: b"GET /resource/gpe5-46if.json / Northstar frozen index contracts".to_vec(),
            payload: fetched.payload,
        };
        let payload_hash = self.receipt_writer.append(&receipt)?;
        self.raw_receipt_count = self.raw_receipt_count.saturating_add(1);
        self.last_receipts
            .insert((receipt.source, receipt.stream), receipt.ts_received_ns);
        let mut batch = CanonicalBatch::new(BatchId(self.next_batch_id));
        let stats = self.cftc_decoder.decode(
            RawReceiptRef {
                id: receipt.id,
                source: receipt.source,
                stream: receipt.stream,
                status_code: receipt.status_code,
                content_type: receipt.content_type,
                ts_started_ns: receipt.ts_started_ns,
                ts_received_ns: receipt.ts_received_ns,
                metadata: &receipt.metadata,
                payload: &receipt.payload,
                payload_hash,
            },
            &mut batch,
        )?;
        batch.events.retain(|event| {
            !self.seen_events.contains(&(
                event.header.source_id,
                event.header.stream_id,
                event.header.source_event_id,
            ))
        });
        for event in &mut batch.events {
            let key = (
                event.instrument_id(),
                event.participant_class()?,
                event.header.ts_event_ns,
            );
            if self
                .positioning_versions
                .get(&key)
                .is_some_and(|previous| *previous != event.header.source_event_id)
            {
                event.header.flags |= EventFlags::CORRECTION;
            }
        }
        let committed_events =
            u32::try_from(batch.events.len()).map_err(|_| MacroPlaneError::TooLarge)?;
        let material = if committed_events != 0 {
            self.journal_writer.append_batch(&mut batch)?;
            self.projector.apply_batch(&batch.events)?;
            for event in &batch.events {
                self.seen_events.insert((
                    event.header.source_id,
                    event.header.stream_id,
                    event.header.source_event_id,
                ));
                self.positioning_versions.insert(
                    (
                        event.instrument_id(),
                        event.participant_class()?,
                        event.header.ts_event_ns,
                    ),
                    event.header.source_event_id,
                );
            }
            self.next_batch_id = self.next_batch_id.saturating_add(1);
            Some(MacroMaterialBatch::from_events(&batch.events)?)
        } else {
            None
        };
        let snapshot = self.published_snapshot(
            Availability::Live,
            if committed_events == 0 {
                "CFTC TFF refresh current / no new reports"
            } else {
                "CFTC TFF positioning committed to canonical replay"
            },
            next_refresh_ns,
        );
        Ok(MacroIngestReport {
            raw_receipts: 1,
            decoded_events: stats.canonical_events,
            committed_events,
            material,
            provenance_material: None,
            snapshot,
        })
    }
}

fn validate_complete_refresh(receipts: &[FetchedMacroReceipt]) -> Result<(), MacroPlaneError> {
    if receipts.len() != BLS_SERIES.len() {
        return Err(MacroPlaneError::IncompleteRefresh {
            expected: BLS_SERIES.len(),
            actual: receipts.len(),
        });
    }
    let mut seen = HashSet::with_capacity(BLS_SERIES.len());
    for receipt in receipts {
        if binding_for_code(receipt.provider_code).is_none() {
            return Err(MacroPlaneError::UnknownSeries(receipt.provider_code.into()));
        }
        if !seen.insert(receipt.provider_code) {
            return Err(MacroPlaneError::DuplicateSeries(
                receipt.provider_code.into(),
            ));
        }
        if receipt.status_code != 200 {
            return Err(MacroPlaneError::HttpStatus {
                series: receipt.provider_code.into(),
                status: receipt.status_code,
            });
        }
    }
    Ok(())
}

fn validate_complete_eurostat_refresh(
    receipts: &[FetchedEurostatReceipt],
) -> Result<(), MacroPlaneError> {
    if receipts.len() != EUROSTAT_BINDINGS.len() {
        return Err(MacroPlaneError::IncompleteEurostatRefresh {
            expected: EUROSTAT_BINDINGS.len(),
            actual: receipts.len(),
        });
    }
    let mut seen = HashSet::with_capacity(EUROSTAT_BINDINGS.len());
    for receipt in receipts {
        if binding_for_dataset(receipt.dataset_code).is_none() {
            return Err(MacroPlaneError::UnknownEurostatDataset(
                receipt.dataset_code.into(),
            ));
        }
        if !seen.insert(receipt.dataset_code) {
            return Err(MacroPlaneError::DuplicateSeries(
                receipt.dataset_code.into(),
            ));
        }
        if receipt.status_code != 200 {
            return Err(MacroPlaneError::HttpStatus {
                series: receipt.dataset_code.into(),
                status: receipt.status_code,
            });
        }
    }
    Ok(())
}

fn split_material(
    events: &[crate::data_plane::event::CanonicalEvent],
) -> Result<(Option<MacroMaterialBatch>, Option<MacroMaterialBatch>), MacroMaterialError> {
    let mut primary = None;
    let mut provenance = None;
    for batch in MacroMaterialBatch::partition(events)? {
        let target = if batch.kind == MacroMaterialKind::Provenance {
            &mut provenance
        } else {
            &mut primary
        };
        if target.replace(batch).is_some() {
            return Err(MacroMaterialError::MixedKind);
        }
    }
    Ok((primary, provenance))
}

#[derive(Debug, Error)]
pub enum MacroPlaneError {
    #[error("macro I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
    #[error(transparent)]
    Journal(#[from] JournalError),
    #[error(transparent)]
    Decode(#[from] crate::data_plane::source::DecodeError),
    #[error(transparent)]
    Canonical(#[from] crate::data_plane::event::CanonicalError),
    #[error(transparent)]
    Projector(#[from] MacroProjectorError),
    #[error(transparent)]
    Material(#[from] MacroMaterialError),
    #[error("BLS refresh is incomplete: expected {expected}, received {actual}")]
    IncompleteRefresh { expected: usize, actual: usize },
    #[error("Eurostat refresh is incomplete: expected {expected}, received {actual}")]
    IncompleteEurostatRefresh { expected: usize, actual: usize },
    #[error("FRED refresh is incomplete: expected {expected}, received {actual}")]
    IncompleteFredRefresh { expected: usize, actual: usize },
    #[error("BEA refresh is incomplete: expected {expected}, received {actual}")]
    IncompleteBeaRefresh { expected: usize, actual: usize },
    #[error("ECB refresh is incomplete: expected {expected}, received {actual}")]
    IncompleteEcbRefresh { expected: usize, actual: usize },
    #[error("ONS refresh is incomplete: expected {expected}, received {actual}")]
    IncompleteOnsRefresh { expected: usize, actual: usize },
    #[error("Bank of England refresh is incomplete: expected {expected}, received {actual}")]
    IncompleteBoeRefresh { expected: usize, actual: usize },
    #[error("Bank of Japan refresh is incomplete: expected {expected}, received {actual}")]
    IncompleteBojRefresh { expected: usize, actual: usize },
    #[error("BLS refresh contains an unknown series: {0}")]
    UnknownSeries(String),
    #[error("Eurostat refresh contains an unknown dataset: {0}")]
    UnknownEurostatDataset(String),
    #[error("FRED refresh contains an unknown series: {0}")]
    UnknownFredSeries(String),
    #[error("BEA refresh contains an unknown series: {0}")]
    UnknownBeaSeries(String),
    #[error("ECB refresh contains an unknown series: {0}")]
    UnknownEcbSeries(String),
    #[error("ONS refresh contains an unknown series: {0}")]
    UnknownOnsSeries(String),
    #[error("Bank of England refresh contains an unknown series: {0}")]
    UnknownBoeSeries(String),
    #[error("Bank of Japan refresh contains an unknown series: {0}")]
    UnknownBojSeries(String),
    #[error("BLS refresh repeats a series: {0}")]
    DuplicateSeries(String),
    #[error("BLS returned HTTP {status} for {series}")]
    HttpStatus { series: String, status: u16 },
    #[error("macro batch exceeds supported bounds")]
    TooLarge,
}

#[cfg(test)]
#[path = "macro_pipeline_tests.rs"]
mod tests;
