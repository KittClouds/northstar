use super::{
    Availability, DeskSnapshot, MarketInstrument, MarketProjector, MarketProjectorError,
    OperatingMode, ReferenceRole,
};
use crate::data_plane::calendar::SessionCalendar;
use crate::data_plane::event::CanonicalBatch;
use crate::data_plane::ids::{BatchId, DerivationVersion, JournalSequence};
use crate::data_plane::journal::{JournalError, JournalWriter, MappedJournal};
use crate::data_plane::providers::massive_catalog::MassiveReferenceRole;
use crate::data_plane::providers::massive_catalog::ValidatedMassiveCatalog;
use crate::data_plane::providers::massive_snapshot::MassiveIndexSnapshotDecoder;
use crate::data_plane::receipt::{RawReceipt, RawReceiptRef, ReceiptError, ReceiptStoreWriter};
use crate::data_plane::replay::{
    CanonicalConsumer, EventBatchRef, ReplayConfig, ReplayEngine, ReplayReport,
};
use crate::data_plane::source::{DecodeError, DecodeStats};
use smallvec::SmallVec;
use std::sync::Arc;
use thiserror::Error;

/// Single-writer Massive L0 -> L1 -> Desk transaction. Provider requests may
/// arrive as six receipts, but the projector observes one durable batch.
pub struct MassiveMarketPlane {
    receipts: ReceiptStoreWriter,
    decoder: MassiveIndexSnapshotDecoder,
    journal: JournalWriter,
    projector: MarketProjector,
    next_batch: BatchId,
}

impl MassiveMarketPlane {
    pub fn new(
        receipts: ReceiptStoreWriter,
        journal: JournalWriter,
        decoder: MassiveIndexSnapshotDecoder,
        projector: MarketProjector,
        first_batch: BatchId,
    ) -> Result<Self, MassiveMarketError> {
        if first_batch == BatchId::UNKNOWN {
            return Err(MassiveMarketError::MissingBatchId);
        }
        Ok(Self {
            receipts,
            decoder,
            journal,
            projector,
            next_batch: first_batch,
        })
    }

    pub fn ingest_refresh(
        &mut self,
        receipts: Vec<RawReceipt>,
    ) -> Result<MassiveMarketIngestReport, MassiveMarketError> {
        if receipts.is_empty() {
            return Err(MassiveMarketError::EmptyRefresh);
        }
        let mut payload_hashes = SmallVec::<[[u8; 32]; 8]>::new();
        for receipt in &receipts {
            payload_hashes.push(self.receipts.append(receipt)?);
        }
        let receipt_refs = receipts
            .iter()
            .zip(payload_hashes.iter().copied())
            .map(|(receipt, payload_hash)| RawReceiptRef {
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
            })
            .collect::<SmallVec<[_; 8]>>();

        let batch_id = self.next_batch;
        let mut batch = CanonicalBatch::new(batch_id);
        let decoded = self.decoder.decode_complete(&receipt_refs, &mut batch)?;
        let committed = self.journal.append_batch(&mut batch)?;
        let snapshot = self
            .projector
            .apply_batch(EventBatchRef {
                id: batch_id,
                first_sequence: committed.first_sequence,
                events: &batch.events,
            })
            .ok_or(MassiveMarketError::ProjectorDidNotChange)?;
        self.next_batch = BatchId(
            batch_id
                .get()
                .checked_add(1)
                .ok_or(MassiveMarketError::BatchIdExhausted)?,
        );
        Ok(MassiveMarketIngestReport {
            batch_id,
            first_sequence: committed.first_sequence,
            last_sequence: committed.last_sequence,
            payload_hashes,
            decoded,
            snapshot,
        })
    }

    pub fn recover(
        &mut self,
        journal: &MappedJournal,
    ) -> (ReplayReport, Option<Arc<DeskSnapshot>>) {
        let mut capture = CapturingProjector {
            projector: &mut self.projector,
            latest: None,
        };
        let report = ReplayEngine::new(journal).run(ReplayConfig::default(), &mut capture);
        (report, capture.latest)
    }

    #[inline]
    pub fn status_snapshot(&self, availability: Availability) -> Arc<DeskSnapshot> {
        self.projector.current_snapshot(availability)
    }

    #[inline]
    pub const fn next_batch(&self) -> BatchId {
        self.next_batch
    }

    pub fn into_parts(self) -> (ReceiptStoreWriter, JournalWriter, MarketProjector) {
        (self.receipts, self.journal, self.projector)
    }
}

struct CapturingProjector<'a> {
    projector: &'a mut MarketProjector,
    latest: Option<Arc<DeskSnapshot>>,
}

impl CanonicalConsumer for CapturingProjector<'_> {
    fn apply_batch(&mut self, batch: EventBatchRef<'_>) {
        if let Some(snapshot) = self.projector.apply_batch(batch) {
            self.latest = Some(snapshot);
        }
    }
}

pub fn projector_from_massive_catalog(
    mode: OperatingMode,
    catalog: &ValidatedMassiveCatalog,
    calendar: Arc<SessionCalendar>,
    derivation: DerivationVersion,
) -> Result<MarketProjector, MarketProjectorError> {
    let mut instruments = Vec::with_capacity(catalog.bindings().len());
    for binding in catalog.bindings() {
        instruments.push(MarketInstrument::new_with_role(
            binding.instrument,
            binding.display_name.as_ref(),
            binding.provider_ticker.as_ref(),
            binding.price_scale,
            match binding.reference_role {
                MassiveReferenceRole::ExactBenchmark => ReferenceRole::ExactBenchmark,
                MassiveReferenceRole::ReferenceProxy => ReferenceRole::ContextProxy,
            },
        )?);
    }
    MarketProjector::new(mode, calendar, derivation, instruments)
}

#[derive(Debug)]
pub struct MassiveMarketIngestReport {
    pub batch_id: BatchId,
    pub first_sequence: JournalSequence,
    pub last_sequence: JournalSequence,
    pub payload_hashes: SmallVec<[[u8; 32]; 8]>,
    pub decoded: DecodeStats,
    pub snapshot: Arc<DeskSnapshot>,
}

#[derive(Debug, Error)]
pub enum MassiveMarketError {
    #[error("Massive market first batch id must be non-zero")]
    MissingBatchId,
    #[error("Massive market refresh has no receipts")]
    EmptyRefresh,
    #[error("Massive market batch id space is exhausted")]
    BatchIdExhausted,
    #[error("durable Massive batch did not change the Desk projector")]
    ProjectorDidNotChange,
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    Journal(#[from] JournalError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::calendar::SessionCalendar;
    use crate::data_plane::ids::{instruments, CalendarId, CatalogVersion};
    use crate::data_plane::providers::massive_catalog::{
        MassiveCatalogManifestV1, MassiveIndexBindingV1, MassiveRecency, MassiveReferenceRole,
        MASSIVE_SOURCE,
    };
    use crate::data_plane::source::LicenseClass;

    #[test]
    fn proxy_role_reaches_the_immutable_desk_instrument() {
        let manifest = MassiveCatalogManifestV1 {
            schema_version: 1,
            catalog_version: CatalogVersion(1),
            source_id: MASSIVE_SOURCE,
            captured_at_ns: 10,
            license: LicenseClass::InternalRestricted,
            indices: vec![MassiveIndexBindingV1 {
                instrument: instruments::DE40,
                northstar_symbol: "DE40".into(),
                display_name: "Germany context".into(),
                provider_ticker: "I:BDE40P".into(),
                provider_name: "Cboe Germany 40".into(),
                reference_role: MassiveReferenceRole::ReferenceProxy,
                binding_basis: "methodology comparison required".into(),
                price_scale: 100,
                effective_from_ns: 1,
                verified_at_ns: 5,
                recency: MassiveRecency::Delayed,
            }],
        }
        .validate()
        .unwrap();
        let calendar =
            Arc::new(SessionCalendar::new(CalendarId(1), CatalogVersion(1), Vec::new()).unwrap());
        let projector = projector_from_massive_catalog(
            OperatingMode::LiveData,
            &manifest,
            calendar,
            DerivationVersion(1),
        )
        .unwrap();
        let snapshot = projector.current_snapshot(Availability::AwaitingFirstReceipt);
        assert_eq!(
            snapshot.instruments[0].instrument.reference_role,
            ReferenceRole::ContextProxy
        );
    }
}
