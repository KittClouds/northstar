use super::super::snapshot::SnapshotPublisher;
use super::MarketProjector;
use crate::data_plane::replay::{CanonicalConsumer, EventBatchRef};

pub struct MarketSnapshotBridge {
    projector: MarketProjector,
    publisher: SnapshotPublisher,
}

impl MarketSnapshotBridge {
    pub const fn new(projector: MarketProjector, publisher: SnapshotPublisher) -> Self {
        Self {
            projector,
            publisher,
        }
    }

    #[inline]
    pub const fn projector(&self) -> &MarketProjector {
        &self.projector
    }

    pub fn into_parts(self) -> (MarketProjector, SnapshotPublisher) {
        (self.projector, self.publisher)
    }
}

impl CanonicalConsumer for MarketSnapshotBridge {
    fn apply_batch(&mut self, batch: EventBatchRef<'_>) {
        if let Some(snapshot) = self.projector.apply_batch(batch) {
            self.publisher.publish_desk(snapshot);
        }
    }
}
