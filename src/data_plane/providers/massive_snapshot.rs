use super::massive::scale_exact;
use super::massive_catalog::{
    MassiveCatalogError, MassiveRecency, ValidatedMassiveCatalog, MASSIVE_DESK_CAPACITY,
};
use crate::data_plane::catalog::{AuthorityDomain, CanonicalCatalog};
use crate::data_plane::event::{CanonicalBatch, CanonicalEvent, TimeQuality};
use crate::data_plane::ids::{ReceiptId, SourceId, StreamId};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use serde::Deserialize;
use smallvec::SmallVec;
use std::sync::Arc;

#[derive(Clone)]
pub struct MassiveIndexSnapshotDecoder {
    source: SourceId,
    stream: StreamId,
    manifest: Arc<ValidatedMassiveCatalog>,
    catalog: Arc<CanonicalCatalog>,
}

impl MassiveIndexSnapshotDecoder {
    pub fn new(
        stream: StreamId,
        manifest: Arc<ValidatedMassiveCatalog>,
    ) -> Result<Self, MassiveCatalogError> {
        let source = manifest.manifest().source_id;
        let catalog = Arc::new(manifest.canonical_catalog()?);
        Ok(Self {
            source,
            stream,
            manifest,
            catalog,
        })
    }
}

#[derive(Debug, Deserialize)]
struct WireSnapshotEnvelope<'a> {
    #[serde(borrow)]
    status: &'a str,
    #[serde(borrow)]
    results: Option<Vec<WireSnapshotResult<'a>>>,
}

#[derive(Debug, Deserialize)]
struct WireSnapshotResult<'a> {
    #[serde(borrow)]
    ticker: &'a str,
    #[serde(borrow)]
    error: Option<&'a str>,
    #[serde(borrow)]
    message: Option<&'a str>,
    last_updated: Option<i64>,
    value: Option<f64>,
    #[serde(borrow)]
    timeframe: Option<&'a str>,
    #[serde(borrow, rename = "type")]
    market_type: Option<&'a str>,
}

impl CanonicalDecoder for MassiveIndexSnapshotDecoder {
    fn source_id(&self) -> SourceId {
        self.source
    }

    fn decode(
        &self,
        receipt: RawReceiptRef<'_>,
        output: &mut CanonicalBatch,
    ) -> Result<DecodeStats, DecodeError> {
        self.decode_complete(std::slice::from_ref(&receipt), output)
    }
}

impl MassiveIndexSnapshotDecoder {
    /// Decode one logical six-index refresh assembled from one or more exact
    /// HTTP receipts. This supports one request per verified ticker without
    /// inventing an undocumented multi-ticker query contract.
    pub fn decode_complete(
        &self,
        receipts: &[RawReceiptRef<'_>],
        output: &mut CanonicalBatch,
    ) -> Result<DecodeStats, DecodeError> {
        let expected = self.manifest.bindings().len();
        let mut prepared = SmallVec::<[CanonicalEvent; MASSIVE_DESK_CAPACITY]>::new();
        let mut seen = 0u8;
        let mut provider_records = 0usize;
        for receipt in receipts {
            self.decode_receipt(*receipt, &mut seen, &mut provider_records, &mut prepared)?;
        }
        let actual = seen.count_ones() as usize;
        if actual != expected || provider_records != expected {
            return Err(DecodeError::IncompleteBatch { expected, actual });
        }
        for event in prepared {
            output.push(event);
        }
        Ok(DecodeStats {
            provider_records: provider_records as u32,
            canonical_events: expected as u32,
        })
    }

    fn decode_receipt(
        &self,
        receipt: RawReceiptRef<'_>,
        seen: &mut u8,
        provider_records: &mut usize,
        prepared: &mut SmallVec<[CanonicalEvent; MASSIVE_DESK_CAPACITY]>,
    ) -> Result<(), DecodeError> {
        if receipt.source != self.source {
            return Err(DecodeError::WrongSource {
                expected: self.source,
                actual: receipt.source,
            });
        }
        if matches!(receipt.status_code, 401 | 403) {
            return Err(DecodeError::NotEntitled(format!(
                "Massive snapshot HTTP status {}",
                receipt.status_code
            )));
        }
        if receipt.status_code == 429 {
            return Err(DecodeError::RateLimited);
        }
        if receipt.status_code != 200 {
            return Err(DecodeError::UnsupportedSchema(format!(
                "Massive snapshot HTTP status {}",
                receipt.status_code
            )));
        }
        let envelope: WireSnapshotEnvelope<'_> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        if envelope.status != "OK" {
            return Err(DecodeError::UnsupportedSchema(format!(
                "Massive snapshot status {}",
                envelope.status
            )));
        }
        let results = envelope.results.ok_or(DecodeError::IncompleteBatch {
            expected: self.manifest.bindings().len(),
            actual: 0,
        })?;
        for record in results {
            *provider_records = (*provider_records).saturating_add(1);
            if let Some(error) = record.error {
                let detail = record.message.unwrap_or(error);
                return match error {
                    "NOT_ENTITLED" => Err(DecodeError::NotEntitled(format!(
                        "{}: {detail}",
                        record.ticker
                    ))),
                    "NOT_FOUND" => Err(DecodeError::NotFound(format!(
                        "{}: {detail}",
                        record.ticker
                    ))),
                    other => Err(DecodeError::UnsupportedSchema(format!(
                        "Massive result error {other} for {}: {detail}",
                        record.ticker
                    ))),
                };
            }
            let (slot, binding) = self
                .manifest
                .bindings()
                .iter()
                .enumerate()
                .find(|(_, binding)| binding.provider_ticker.as_ref() == record.ticker)
                .ok_or_else(|| DecodeError::UnknownIdentity(record.ticker.to_owned()))?;
            let bit = 1u8 << slot;
            if *seen & bit != 0 {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "duplicate Massive snapshot ticker {}",
                    record.ticker
                )));
            }
            *seen |= bit;
            if record.market_type != Some("indices") {
                return Err(DecodeError::UnsupportedSchema(format!(
                    "{} is not an indices snapshot",
                    record.ticker
                )));
            }
            let recency = parse_recency(record.ticker, record.timeframe)?;
            if recency != binding.recency {
                return Err(DecodeError::EntitlementMismatch(format!(
                    "{} catalog={} response={}",
                    record.ticker,
                    binding.recency.wire_name(),
                    recency.wire_name()
                )));
            }
            let timestamp_ns = record.last_updated.ok_or_else(|| {
                DecodeError::UnsupportedSchema(format!("{} has no last_updated", record.ticker))
            })?;
            if timestamp_ns <= 0 {
                return Err(DecodeError::TimestampOverflow);
            }
            let value = record.value.ok_or_else(|| {
                DecodeError::InvalidNumber(format!("{} has no value", record.ticker))
            })?;
            let canonical_binding = self
                .catalog
                .resolve_symbol(self.source, record.ticker, timestamp_ns)
                .ok_or_else(|| DecodeError::UnknownIdentity(record.ticker.to_owned()))?;
            if canonical_binding.instrument != binding.instrument
                || self.catalog.authority(
                    AuthorityDomain::ReferenceValue,
                    binding.instrument.get(),
                    timestamp_ns,
                ) != Some(self.source)
            {
                return Err(DecodeError::UnknownIdentity(format!(
                    "{} has no active reference authority",
                    record.ticker
                )));
            }
            let scaled = scale_exact(value, binding.price_scale)?;
            let quality = match recency {
                MassiveRecency::RealTime => TimeQuality::ObservedLive,
                MassiveRecency::Delayed => TimeQuality::ProviderDelayed,
            };
            prepared.push(CanonicalEvent::index_value(
                self.source,
                self.stream,
                binding.instrument,
                ReceiptId(receipt.id.get()),
                stable_snapshot_event_id(record.ticker, timestamp_ns, scaled, recency),
                timestamp_ns,
                receipt.ts_received_ns,
                scaled,
                quality,
            )?);
        }
        Ok(())
    }
}

fn parse_recency(ticker: &str, value: Option<&str>) -> Result<MassiveRecency, DecodeError> {
    match value {
        Some("REAL-TIME") => Ok(MassiveRecency::RealTime),
        Some("DELAYED") => Ok(MassiveRecency::Delayed),
        Some(other) => Err(DecodeError::UnsupportedSchema(format!(
            "unknown Massive timeframe {other} for {ticker}"
        ))),
        None => Err(DecodeError::UnsupportedSchema(format!(
            "missing Massive timeframe for {ticker}"
        ))),
    }
}

fn stable_snapshot_event_id(
    ticker: &str,
    timestamp_ns: i64,
    value: i64,
    recency: MassiveRecency,
) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"massive-index-snapshot-v1");
    hasher.update(ticker.as_bytes());
    hasher.update(&timestamp_ns.to_le_bytes());
    hasher.update(&value.to_le_bytes());
    hasher.update(&[recency as u8]);
    let bytes = hasher.finalize();
    let identity = u64::from_le_bytes(bytes.as_bytes()[..8].try_into().expect("eight bytes"));
    identity.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::ids::{instruments, BatchId, CatalogVersion};
    use crate::data_plane::providers::massive_catalog::{
        MassiveCatalogManifestV1, MassiveIndexBindingV1, MassiveReferenceRole, MASSIVE_SOURCE,
    };
    use crate::data_plane::source::LicenseClass;
    use serde_json::{json, Value};

    fn decoder(recency: MassiveRecency) -> MassiveIndexSnapshotDecoder {
        let definitions = [
            (instruments::US100, "US100", "I:ONE"),
            (instruments::US500, "US500", "I:TWO"),
            (instruments::US30, "US30", "I:THREE"),
            (instruments::DE40, "DE40", "I:FOUR"),
            (instruments::UK100, "UK100", "I:FIVE"),
            (instruments::JP225, "JP225", "I:SIX"),
        ];
        let manifest = MassiveCatalogManifestV1 {
            schema_version: 1,
            catalog_version: CatalogVersion(1),
            source_id: MASSIVE_SOURCE,
            captured_at_ns: 10,
            license: LicenseClass::InternalRestricted,
            indices: definitions
                .into_iter()
                .map(|(instrument, symbol, ticker)| MassiveIndexBindingV1 {
                    instrument,
                    northstar_symbol: symbol.into(),
                    display_name: symbol.into(),
                    provider_ticker: ticker.into(),
                    provider_name: symbol.into(),
                    reference_role: MassiveReferenceRole::ExactBenchmark,
                    binding_basis: format!("fixture exact binding for {symbol}").into(),
                    price_scale: 100,
                    effective_from_ns: 1,
                    verified_at_ns: 5,
                    recency,
                })
                .collect(),
        }
        .validate()
        .unwrap();
        MassiveIndexSnapshotDecoder::new(StreamId(3), Arc::new(manifest)).unwrap()
    }

    fn payload(recency: MassiveRecency) -> Vec<u8> {
        let timeframe = recency.wire_name();
        let results = ["I:ONE", "I:TWO", "I:THREE", "I:FOUR", "I:FIVE", "I:SIX"]
            .into_iter()
            .enumerate()
            .map(|(index, ticker)| {
                json!({
                    "ticker": ticker,
                    "type": "indices",
                    "timeframe": timeframe,
                    "last_updated": 1_000_000_000 + index as i64,
                    "value": 100.25 + index as f64,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_vec(&json!({
            "status": "OK",
            "results": results,
        }))
        .unwrap()
    }

    fn receipt<'a>(payload: &'a [u8]) -> RawReceiptRef<'a> {
        RawReceiptRef {
            id: ReceiptId(1),
            source: MASSIVE_SOURCE,
            stream: StreamId(3),
            status_code: 200,
            content_type: 1,
            ts_started_ns: 2_000_000_000,
            ts_received_ns: 2_000_000_010,
            metadata: b"fixture",
            payload,
            payload_hash: *blake3::hash(payload).as_bytes(),
        }
    }

    #[test]
    fn complete_snapshot_decodes_atomically_and_preserves_delay_truth() {
        let payload = payload(MassiveRecency::Delayed);
        let mut output = CanonicalBatch::new(BatchId(1));
        let stats = decoder(MassiveRecency::Delayed)
            .decode(receipt(&payload), &mut output)
            .unwrap();
        assert_eq!(stats.canonical_events, 6);
        assert_eq!(output.events.len(), 6);
        assert!(output.events.iter().all(|event| {
            event.flags().time_quality() == TimeQuality::ProviderDelayed
                && event.header.ts_effective_ns == 2_000_000_010
        }));
    }

    #[test]
    fn reviewed_subset_is_complete_without_claiming_absent_instruments() {
        let full = decoder(MassiveRecency::Delayed);
        let mut manifest = full.manifest.manifest().clone();
        manifest.indices.truncate(3);
        let subset =
            MassiveIndexSnapshotDecoder::new(StreamId(3), Arc::new(manifest.validate().unwrap()))
                .unwrap();
        let mut value: Value = serde_json::from_slice(&payload(MassiveRecency::Delayed)).unwrap();
        value["results"].as_array_mut().unwrap().truncate(3);
        let payload = serde_json::to_vec(&value).unwrap();
        let mut output = CanonicalBatch::new(BatchId(1));
        let stats = subset.decode(receipt(&payload), &mut output).unwrap();
        assert_eq!(stats.canonical_events, 3);
        assert_eq!(output.events.len(), 3);
        assert!(output
            .events
            .iter()
            .all(|event| event.instrument_id().get() <= instruments::US30.get()));
    }

    #[test]
    fn one_entitlement_error_rejects_the_whole_batch() {
        let payload = payload(MassiveRecency::RealTime);
        let mut value: Value = serde_json::from_slice(&payload).unwrap();
        value["results"][2] = json!({
            "ticker": "I:THREE",
            "error": "NOT_ENTITLED",
            "message": "Not entitled to this ticker."
        });
        let payload = serde_json::to_vec(&value).unwrap();
        let mut output = CanonicalBatch::new(BatchId(1));
        let error = decoder(MassiveRecency::RealTime)
            .decode(receipt(&payload), &mut output)
            .unwrap_err();
        assert!(matches!(error, DecodeError::NotEntitled(_)));
        assert!(output.events.is_empty());
    }

    #[test]
    fn missing_or_changed_recency_fails_closed() {
        let mut value: Value = serde_json::from_slice(&payload(MassiveRecency::RealTime)).unwrap();
        value["results"].as_array_mut().unwrap().pop();
        let missing_payload = serde_json::to_vec(&value).unwrap();
        let mut output = CanonicalBatch::new(BatchId(1));
        assert!(matches!(
            decoder(MassiveRecency::RealTime).decode(receipt(&missing_payload), &mut output),
            Err(DecodeError::IncompleteBatch { .. })
        ));

        let delayed = payload(MassiveRecency::Delayed);
        assert!(matches!(
            decoder(MassiveRecency::RealTime).decode(receipt(&delayed), &mut output),
            Err(DecodeError::EntitlementMismatch(_))
        ));
    }
}
