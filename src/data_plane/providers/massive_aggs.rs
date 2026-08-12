use super::massive::scale_exact;
use super::massive_catalog::{MassiveCatalogError, ValidatedMassiveCatalog};
use crate::data_plane::bars::Timeframe;
use crate::data_plane::catalog::{AuthorityDomain, CanonicalCatalog};
use crate::data_plane::event::{CanonicalBatch, CanonicalEvent, TimeQuality};
use crate::data_plane::ids::{ReceiptId, SourceId, StreamId};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use serde::Deserialize;
use std::sync::Arc;

pub const MASSIVE_AGGREGATE_STREAM_M4: StreamId = StreamId(2);
pub const MASSIVE_AGGREGATE_STREAM_M20: StreamId = StreamId(3);
pub const MASSIVE_AGGREGATE_STREAM_H2: StreamId = StreamId(4);
pub const MASSIVE_AGGREGATE_STREAM_H4: StreamId = StreamId(5);

#[derive(Clone)]
pub struct MassiveAggregateDecoder {
    source: SourceId,
    stream: StreamId,
    timeframe: Timeframe,
    manifest: Arc<ValidatedMassiveCatalog>,
    catalog: Arc<CanonicalCatalog>,
}

impl MassiveAggregateDecoder {
    pub fn new(
        stream: StreamId,
        timeframe: Timeframe,
        manifest: Arc<ValidatedMassiveCatalog>,
    ) -> Result<Self, MassiveCatalogError> {
        let source = manifest.manifest().source_id;
        let catalog = Arc::new(manifest.canonical_catalog()?);
        Ok(Self {
            source,
            stream,
            timeframe,
            manifest,
            catalog,
        })
    }

    #[inline]
    pub const fn timeframe(&self) -> Timeframe {
        self.timeframe
    }
}

#[derive(Debug, Deserialize)]
struct WireAggregateEnvelope<'a> {
    #[serde(borrow)]
    status: &'a str,
    #[serde(borrow)]
    ticker: Option<&'a str>,
    #[serde(rename = "resultsCount")]
    results_count: Option<usize>,
    results: Option<Vec<WireAggregate>>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct WireAggregate {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    /// Provider bar-open time in Unix milliseconds.
    t: i64,
}

impl CanonicalDecoder for MassiveAggregateDecoder {
    fn source_id(&self) -> SourceId {
        self.source
    }

    fn decode(
        &self,
        receipt: RawReceiptRef<'_>,
        output: &mut CanonicalBatch,
    ) -> Result<DecodeStats, DecodeError> {
        if receipt.source != self.source {
            return Err(DecodeError::WrongSource {
                expected: self.source,
                actual: receipt.source,
            });
        }
        match receipt.status_code {
            200 => {}
            401 | 403 => {
                return Err(DecodeError::NotEntitled(format!(
                    "Massive aggregates HTTP status {}",
                    receipt.status_code
                )))
            }
            429 => return Err(DecodeError::RateLimited),
            other => {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "Massive aggregates HTTP status {other}"
                )))
            }
        }

        let envelope: WireAggregateEnvelope<'_> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        if envelope.status != "OK" {
            return Err(DecodeError::UnsupportedSchema(format!(
                "Massive aggregates status {}",
                envelope.status
            )));
        }
        let ticker = envelope
            .ticker
            .ok_or_else(|| DecodeError::UnsupportedSchema("aggregate ticker missing".into()))?;
        let binding = self
            .manifest
            .binding_for_ticker(ticker)
            .ok_or_else(|| DecodeError::UnknownIdentity(ticker.to_owned()))?;
        let mut records = envelope.results.unwrap_or_default();
        if envelope
            .results_count
            .is_some_and(|count| count != records.len())
        {
            return Err(DecodeError::IncompleteBatch {
                expected: envelope.results_count.unwrap_or_default(),
                actual: records.len(),
            });
        }
        records.sort_unstable_by_key(|record| record.t);
        if records.windows(2).any(|pair| pair[0].t == pair[1].t) {
            return Err(DecodeError::UnsupportedSchema(format!(
                "duplicate aggregate timestamp for {ticker}"
            )));
        }

        let mut prepared = Vec::with_capacity(records.len());
        for record in records.iter().copied() {
            let ts_open_ns = record
                .t
                .checked_mul(1_000_000)
                .ok_or(DecodeError::TimestampOverflow)?;
            let ts_close_ns = ts_open_ns
                .checked_add(self.timeframe.duration_ns())
                .ok_or(DecodeError::TimestampOverflow)?;
            let canonical_binding = self
                .catalog
                .resolve_symbol(self.source, ticker, ts_open_ns)
                .ok_or_else(|| DecodeError::UnknownIdentity(ticker.to_owned()))?;
            if canonical_binding.instrument != binding.instrument
                || self.catalog.authority(
                    AuthorityDomain::ReferenceValue,
                    binding.instrument.get(),
                    ts_open_ns,
                ) != Some(self.source)
            {
                return Err(DecodeError::UnknownIdentity(format!(
                    "{ticker} has no active reference authority"
                )));
            }
            let open = scale_exact(record.o, binding.price_scale)?;
            let high = scale_exact(record.h, binding.price_scale)?;
            let low = scale_exact(record.l, binding.price_scale)?;
            let close = scale_exact(record.c, binding.price_scale)?;
            prepared.push(CanonicalEvent::external_bar(
                self.source,
                self.stream,
                binding.instrument,
                ReceiptId(receipt.id.get()),
                stable_aggregate_event_id(
                    ticker,
                    self.timeframe,
                    ts_open_ns,
                    [open, high, low, close],
                ),
                ts_open_ns,
                ts_close_ns,
                receipt.ts_received_ns,
                open,
                high,
                low,
                close,
                self.timeframe as u16,
                TimeQuality::EstimatedHistorical,
            )?);
        }
        for event in prepared {
            output.push(event);
        }
        Ok(DecodeStats {
            provider_records: records.len() as u32,
            canonical_events: records.len() as u32,
        })
    }
}

fn stable_aggregate_event_id(
    ticker: &str,
    timeframe: Timeframe,
    ts_open_ns: i64,
    ohlc: [i64; 4],
) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"massive-index-aggregate-v1");
    hasher.update(ticker.as_bytes());
    hasher.update(&(timeframe as u16).to_le_bytes());
    hasher.update(&ts_open_ns.to_le_bytes());
    for value in ohlc {
        hasher.update(&value.to_le_bytes());
    }
    let bytes = hasher.finalize();
    u64::from_le_bytes(bytes.as_bytes()[..8].try_into().expect("eight bytes")).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::{instruments, BatchId, CatalogVersion};
    use crate::data_plane::providers::massive_catalog::{
        MassiveCatalogManifestV1, MassiveIndexBindingV1, MassiveRecency, MassiveReferenceRole,
        MASSIVE_SOURCE,
    };
    use crate::data_plane::source::LicenseClass;

    fn manifest(license: LicenseClass) -> Arc<ValidatedMassiveCatalog> {
        Arc::new(
            MassiveCatalogManifestV1 {
                schema_version: 1,
                catalog_version: CatalogVersion(1),
                source_id: MASSIVE_SOURCE,
                captured_at_ns: 10,
                license,
                indices: vec![MassiveIndexBindingV1 {
                    instrument: instruments::US100,
                    northstar_symbol: "US100".into(),
                    display_name: "NASDAQ-100".into(),
                    provider_ticker: "I:NDX".into(),
                    provider_name: "NASDAQ-100".into(),
                    reference_role: MassiveReferenceRole::ExactBenchmark,
                    binding_basis: "Massive reference API exact name and ticker".into(),
                    price_scale: 100,
                    effective_from_ns: 1,
                    verified_at_ns: 5,
                    recency: MassiveRecency::Delayed,
                }],
            }
            .validate()
            .unwrap(),
        )
    }

    fn receipt<'a>(payload: &'a [u8]) -> RawReceiptRef<'a> {
        RawReceiptRef {
            id: ReceiptId(1),
            source: MASSIVE_SOURCE,
            stream: MASSIVE_AGGREGATE_STREAM_M4,
            status_code: 200,
            content_type: 1,
            ts_started_ns: 500_000_000,
            ts_received_ns: 600_000_000,
            metadata: b"{}",
            payload,
            payload_hash: [0; 32],
        }
    }

    #[test]
    fn aggregate_rows_become_sorted_stale_external_bars() {
        let decoder = MassiveAggregateDecoder::new(
            MASSIVE_AGGREGATE_STREAM_M4,
            Timeframe::M4,
            manifest(LicenseClass::InternalRestricted),
        )
        .unwrap();
        let payload = br#"{
            "status":"OK","ticker":"I:NDX","resultsCount":2,
            "results":[
                {"o":102.0,"h":105.0,"l":101.0,"c":104.0,"t":2000},
                {"o":100.0,"h":103.0,"l":99.0,"c":102.0,"t":1000}
            ]
        }"#;
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = decoder.decode(receipt(payload), &mut batch).unwrap();
        assert_eq!(stats.canonical_events, 2);
        assert_eq!(batch.events[0].values[4], 1_000_000_000);
        assert_eq!(batch.events[1].values[4], 2_000_000_000);
        assert_eq!(
            batch.events[0].kind().unwrap(),
            crate::data_plane::event::EventKind::ExternalBar
        );
        assert_eq!(
            batch.events[0].flags().time_quality(),
            TimeQuality::EstimatedHistorical
        );
    }

    #[test]
    fn invalid_ohlc_is_atomic_and_display_only_cannot_construct_decoder() {
        assert!(matches!(
            MassiveAggregateDecoder::new(
                MASSIVE_AGGREGATE_STREAM_M4,
                Timeframe::M4,
                manifest(LicenseClass::PersonalDisplayOnly),
            ),
            Err(MassiveCatalogError::RetentionForbidden(_))
        ));

        let decoder = MassiveAggregateDecoder::new(
            MASSIVE_AGGREGATE_STREAM_M4,
            Timeframe::M4,
            manifest(LicenseClass::InternalRestricted),
        )
        .unwrap();
        let payload = br#"{
            "status":"OK","ticker":"I:NDX","resultsCount":1,
            "results":[{"o":100.0,"h":99.0,"l":98.0,"c":101.0,"t":1000}]
        }"#;
        let mut batch = CanonicalBatch::new(BatchId(1));
        assert!(decoder.decode(receipt(payload), &mut batch).is_err());
        assert!(batch.is_empty());
    }
}
