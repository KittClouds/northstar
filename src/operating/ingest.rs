use super::{MarketProjector, MarketSnapshotBridge, SnapshotPublisher};
use crate::data_plane::event::CanonicalBatch;
use crate::data_plane::ids::{BatchId, JournalSequence};
use crate::data_plane::journal::{JournalError, JournalWriter};
use crate::data_plane::providers::massive::MassiveIndexValueDecoder;
use crate::data_plane::receipt::{RawReceipt, RawReceiptRef, ReceiptError, ReceiptStoreWriter};
use crate::data_plane::replay::LivePublisher;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use thiserror::Error;

/// Durable Massive receipt -> canonical journal -> immutable Desk snapshot.
///
/// The exact provider payload is committed before decoding. A decode failure
/// therefore leaves auditable L0 truth but cannot mutate canonical/UI state.
pub struct MassiveDeskPipeline {
    receipts: ReceiptStoreWriter,
    decoder: MassiveIndexValueDecoder,
    live: LivePublisher<MarketSnapshotBridge>,
    next_batch: BatchId,
}

impl MassiveDeskPipeline {
    pub fn new(
        receipts: ReceiptStoreWriter,
        journal: JournalWriter,
        decoder: MassiveIndexValueDecoder,
        projector: MarketProjector,
        snapshots: SnapshotPublisher,
        first_batch: BatchId,
    ) -> Result<Self, MassiveIngestError> {
        if first_batch == BatchId::UNKNOWN {
            return Err(MassiveIngestError::MissingBatchId);
        }
        Ok(Self {
            receipts,
            decoder,
            live: LivePublisher::new(journal, MarketSnapshotBridge::new(projector, snapshots)),
            next_batch: first_batch,
        })
    }

    pub fn ingest(
        &mut self,
        receipt: RawReceipt,
    ) -> Result<MassiveIngestReport, MassiveIngestError> {
        let payload_hash = self.receipts.append(&receipt)?;
        let receipt_ref = RawReceiptRef {
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
        };
        let batch_id = self.next_batch;
        let mut batch = CanonicalBatch::new(batch_id);
        let decoded = self.decoder.decode(receipt_ref, &mut batch)?;
        if batch.events.is_empty() {
            return Err(MassiveIngestError::EmptyCanonicalBatch);
        }
        let committed = self.live.commit_and_publish(&mut batch)?;
        self.next_batch = BatchId(
            batch_id
                .get()
                .checked_add(1)
                .ok_or(MassiveIngestError::BatchIdExhausted)?,
        );
        Ok(MassiveIngestReport {
            batch_id,
            first_sequence: committed.first_sequence,
            last_sequence: committed.last_sequence,
            payload_hash,
            decoded,
        })
    }

    #[inline]
    pub const fn next_batch(&self) -> BatchId {
        self.next_batch
    }

    pub fn into_parts(
        self,
    ) -> (
        ReceiptStoreWriter,
        JournalWriter,
        MarketProjector,
        SnapshotPublisher,
    ) {
        let (journal, bridge) = self.live.into_parts();
        let (projector, snapshots) = bridge.into_parts();
        (self.receipts, journal, projector, snapshots)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MassiveIngestReport {
    pub batch_id: BatchId,
    pub first_sequence: JournalSequence,
    pub last_sequence: JournalSequence,
    pub payload_hash: [u8; 32],
    pub decoded: DecodeStats,
}

#[derive(Debug, Error)]
pub enum MassiveIngestError {
    #[error("first batch id must be non-zero")]
    MissingBatchId,
    #[error("Massive batch id space is exhausted")]
    BatchIdExhausted,
    #[error("Massive receipt decoded to no canonical events")]
    EmptyCanonicalBatch,
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    Journal(#[from] JournalError),
}
