use crate::data_plane::catalog::{AuthorityDomain, CanonicalCatalog};
use crate::data_plane::event::{CanonicalEvent, TimeQuality};
use crate::data_plane::ids::{ReceiptId, SourceId, StreamId};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct MassiveIndexValueDecoder {
    source: SourceId,
    stream: StreamId,
    catalog: Arc<CanonicalCatalog>,
}

impl MassiveIndexValueDecoder {
    pub const fn new(source: SourceId, stream: StreamId, catalog: Arc<CanonicalCatalog>) -> Self {
        Self {
            source,
            stream,
            catalog,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WireEnvelope<'a> {
    One(#[serde(borrow)] WireIndexValue<'a>),
    Many(#[serde(borrow)] Vec<WireIndexValue<'a>>),
}

#[derive(Debug, Deserialize)]
struct WireIndexValue<'a> {
    #[serde(borrow)]
    ev: &'a str,
    val: f64,
    #[serde(borrow, rename = "T")]
    ticker: &'a str,
    t: i64,
}

impl CanonicalDecoder for MassiveIndexValueDecoder {
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
        let envelope: WireEnvelope<'_> = serde_json::from_slice(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        let count = match envelope {
            WireEnvelope::One(record) => {
                self.decode_record(receipt, record, output)?;
                1
            }
            WireEnvelope::Many(records) => {
                let count = u32::try_from(records.len()).map_err(|_| {
                    DecodeError::UnsupportedSchema("event array is too large".into())
                })?;
                for record in records {
                    self.decode_record(receipt, record, output)?;
                }
                count
            }
        };
        Ok(DecodeStats {
            provider_records: count,
            canonical_events: count,
        })
    }
}

impl MassiveIndexValueDecoder {
    fn decode_record(
        &self,
        receipt: RawReceiptRef<'_>,
        record: WireIndexValue<'_>,
        output: &mut CanonicalBatch,
    ) -> Result<(), DecodeError> {
        if record.ev != "V" {
            return Err(DecodeError::UnsupportedSchema(format!(
                "expected Massive V event, got {}",
                record.ev
            )));
        }
        let timestamp_ns = record
            .t
            .checked_mul(1_000_000)
            .ok_or(DecodeError::TimestampOverflow)?;
        let binding = self
            .catalog
            .resolve_symbol(self.source, record.ticker, timestamp_ns)
            .ok_or_else(|| DecodeError::UnknownIdentity(record.ticker.to_owned()))?;
        if self.catalog.authority(
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
        let scaled = scale_exact(record.val, binding.price_scale)?;
        let source_event_id = stable_source_event_id(record.ticker, record.t, scaled);
        output.push(CanonicalEvent::index_value(
            self.source,
            self.stream,
            binding.instrument,
            ReceiptId(receipt.id.get()),
            source_event_id,
            timestamp_ns,
            receipt.ts_received_ns,
            scaled,
            TimeQuality::ObservedLive,
        )?);
        Ok(())
    }
}

pub(super) fn scale_exact(value: f64, scale: i64) -> Result<i64, DecodeError> {
    if !value.is_finite() || scale <= 0 {
        return Err(DecodeError::InvalidNumber(value.to_string()));
    }
    let scaled = value * scale as f64;
    if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
        return Err(DecodeError::InvalidNumber(value.to_string()));
    }
    let rounded = scaled.round();
    if (scaled - rounded).abs() > 1e-6 {
        return Err(DecodeError::InvalidNumber(format!(
            "{value} exceeds configured precision"
        )));
    }
    Ok(rounded as i64)
}

fn stable_source_event_id(ticker: &str, timestamp_ms: i64, value: i64) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"massive-index-value-v1");
    hasher.update(ticker.as_bytes());
    hasher.update(&timestamp_ms.to_le_bytes());
    hasher.update(&value.to_le_bytes());
    let bytes = hasher.finalize();
    let identity = u64::from_le_bytes(bytes.as_bytes()[..8].try_into().expect("eight bytes"));
    if identity == 0 {
        1
    } else {
        identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::catalog::{AuthorityLease, ProviderSymbolBinding};
    use crate::data_plane::ids::{BatchId, CatalogVersion, InstrumentId};

    fn catalog() -> Arc<CanonicalCatalog> {
        Arc::new(
            CanonicalCatalog::new(
                CatalogVersion(1),
                vec![ProviderSymbolBinding {
                    source: SourceId(10),
                    instrument: InstrumentId(1),
                    symbol: "I:TEST".into(),
                    price_scale: 100,
                    effective_from_ns: 0,
                    effective_to_ns: i64::MAX,
                    catalog_version: CatalogVersion(1),
                }],
                vec![AuthorityLease {
                    domain: AuthorityDomain::ReferenceValue,
                    entity_id: 1,
                    source: SourceId(10),
                    effective_from_ns: 0,
                    effective_to_ns: i64::MAX,
                }],
            )
            .unwrap(),
        )
    }

    #[test]
    fn documented_value_shape_decodes_without_provider_objects_leaking() {
        let payload = br#"[{"ev":"V","val":20123.45,"T":"I:TEST","t":1000}]"#;
        let receipt = RawReceiptRef {
            id: ReceiptId(1),
            source: SourceId(10),
            stream: StreamId(2),
            status_code: 101,
            content_type: 1,
            ts_started_ns: 1_000_000_000,
            ts_received_ns: 1_000_000_010,
            metadata: b"fixture",
            payload,
            payload_hash: *blake3::hash(payload).as_bytes(),
        };
        let decoder = MassiveIndexValueDecoder::new(SourceId(10), StreamId(2), catalog());
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = decoder.decode(receipt, &mut batch).unwrap();
        assert_eq!(stats.canonical_events, 1);
        assert_eq!(batch.events[0].values[0], 2_012_345);
    }
}
