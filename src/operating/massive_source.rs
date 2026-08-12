use super::{
    projector_from_massive_catalog, Availability, DeskSnapshot, MarketProjectorError,
    MassiveMarketPlane, OperatingMode,
};
use crate::data_plane::calendar::{CalendarError, SessionCalendar};
use crate::data_plane::ids::{BatchId, CalendarId, DerivationVersion};
use crate::data_plane::journal::{JournalError, JournalWriter, MappedJournal};
use crate::data_plane::providers::massive_rest::{
    MassiveRestSnapshotAdapter, MASSIVE_SNAPSHOT_STREAM,
};
use crate::data_plane::providers::massive_runtime::{
    MassiveRuntimeConfig, MassiveRuntimeConfigError,
};
use crate::data_plane::providers::massive_snapshot::MassiveIndexSnapshotDecoder;
use crate::data_plane::receipt::{ReceiptError, ReceiptStoreWriter};
use crate::data_plane::source::SourceError;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

pub(super) struct MassiveSourceStartup {
    pub plane: MassiveMarketPlane,
    pub adapter: MassiveRestSnapshotAdapter,
    pub initial_snapshot: Arc<DeskSnapshot>,
}

pub(super) fn open_massive_source(
    root: &Path,
    mode: OperatingMode,
    created_ns: i64,
) -> Result<MassiveSourceStartup, MassiveSourceStartupError> {
    let config = MassiveRuntimeConfig::from_environment(root.join("massive.catalog.json"))?;
    let catalog = Arc::clone(&config.catalog);
    let raw_path = root.join("massive.raw");
    let journal_path = root.join("massive.canonical");
    let existing = match (raw_path.exists(), journal_path.exists()) {
        (false, false) => false,
        (true, true) => true,
        _ => {
            return Err(MassiveSourceStartupError::IncompletePair {
                raw: raw_path,
                canonical: journal_path,
            })
        }
    };

    let mapped = existing
        .then(|| MappedJournal::open(&journal_path))
        .transpose()?;
    let next_batch = next_batch_id(mapped.as_ref())?;
    let receipts = if existing {
        ReceiptStoreWriter::open(&raw_path)?
    } else {
        ReceiptStoreWriter::create(&raw_path, created_ns)?
    };
    let first_receipt = receipts.next_receipt_id()?;
    let journal = if existing {
        JournalWriter::open(&journal_path)?
    } else {
        JournalWriter::create(&journal_path, created_ns)?
    };
    let calendar = Arc::new(SessionCalendar::new(
        CalendarId(10),
        catalog.manifest().catalog_version,
        Vec::new(),
    )?);
    let projector = projector_from_massive_catalog(mode, &catalog, calendar, DerivationVersion(1))?;
    let decoder = MassiveIndexSnapshotDecoder::new(MASSIVE_SNAPSHOT_STREAM, Arc::clone(&catalog))?;
    let mut plane = MassiveMarketPlane::new(receipts, journal, decoder, projector, next_batch)
        .map_err(MassiveSourceStartupError::MarketPlane)?;
    let recovered = mapped.as_ref().and_then(|mapped| plane.recover(mapped).1);
    let initial_snapshot =
        recovered.unwrap_or_else(|| plane.status_snapshot(Availability::AwaitingFirstReceipt));
    let adapter = MassiveRestSnapshotAdapter::new(config, first_receipt)?;
    Ok(MassiveSourceStartup {
        plane,
        adapter,
        initial_snapshot,
    })
}

fn next_batch_id(journal: Option<&MappedJournal>) -> Result<BatchId, MassiveSourceStartupError> {
    let last = journal
        .and_then(|journal| journal.batches().last())
        .map_or(0, |batch| batch.id.get());
    let next = last
        .checked_add(1)
        .ok_or(MassiveSourceStartupError::BatchIdExhausted)?;
    Ok(BatchId(next))
}

pub(super) fn unavailable_snapshot(availability: Availability) -> Arc<DeskSnapshot> {
    Arc::new(DeskSnapshot {
        generation: 0,
        published_ns: 0,
        last_sequence: crate::data_plane::ids::JournalSequence::UNKNOWN,
        availability,
        instruments: Arc::from([]),
    })
}

#[derive(Debug, Error)]
pub(super) enum MassiveSourceStartupError {
    #[error(transparent)]
    Config(#[from] MassiveRuntimeConfigError),
    #[error("Massive files must exist as a pair: raw={raw:?}, canonical={canonical:?}")]
    IncompletePair { raw: PathBuf, canonical: PathBuf },
    #[error("Massive batch id space is exhausted")]
    BatchIdExhausted,
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
    #[error(transparent)]
    Journal(#[from] JournalError),
    #[error(transparent)]
    Calendar(#[from] CalendarError),
    #[error(transparent)]
    Projector(#[from] MarketProjectorError),
    #[error(transparent)]
    Catalog(#[from] crate::data_plane::providers::massive_catalog::MassiveCatalogError),
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error(transparent)]
    MarketPlane(super::MassiveMarketError),
}

impl MassiveSourceStartupError {
    pub(super) fn availability(&self) -> Availability {
        match self {
            Self::Config(MassiveRuntimeConfigError::CredentialMissing) => {
                Availability::NotConfigured
            }
            Self::Config(MassiveRuntimeConfigError::CatalogIo { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound => Availability::NotConfigured,
            Self::Config(MassiveRuntimeConfigError::Catalog(
                crate::data_plane::providers::massive_catalog::MassiveCatalogError::RetentionForbidden(_),
            )) => Availability::NotEntitled,
            _ => Availability::Invalid,
        }
    }
}
