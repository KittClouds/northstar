use crate::data_plane::bars::{CanonicalBar, Timeframe};
use crate::data_plane::ids::{DerivationVersion, JournalSequence, SourceId, StreamId};
use arc_swap::ArcSwap;
#[cfg(any(feature = "desktop", test))]
use crossbeam_channel::select;
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError, TrySendError};
use std::ops::{BitOr, BitOrAssign};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, Weak};

pub const BAR_SERIES_BLOCK_LEN: usize = 256;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum Availability {
    Live = 0,
    Delayed = 1,
    Stale = 2,
    Replaying = 3,
    #[default]
    AwaitingFirstReceipt = 4,
    NotConfigured = 5,
    NotEntitled = 6,
    Disconnected = 7,
    Invalid = 8,
    Deferred = 9,
    Ready = 10,
}

impl Availability {
    #[inline]
    pub const fn has_value(self) -> bool {
        matches!(
            self,
            Self::Live | Self::Delayed | Self::Stale | Self::Replaying | Self::Ready
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum OperatingMode {
    #[default]
    OfflineReplay = 0,
    LiveData = 1,
    Paper = 2,
    Live = 3,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct ValueMeta {
    pub ts_event_ns: i64,
    pub ts_received_ns: i64,
    pub ts_effective_ns: i64,
    pub journal_sequence: JournalSequence,
    pub derivation_version: DerivationVersion,
    pub source: SourceId,
    pub stream: StreamId,
    pub availability: Availability,
    pub reserved: [u8; 3],
}

impl ValueMeta {
    pub const fn unavailable(availability: Availability) -> Self {
        Self {
            ts_event_ns: 0,
            ts_received_ns: 0,
            ts_effective_ns: 0,
            journal_sequence: JournalSequence::UNKNOWN,
            derivation_version: DerivationVersion::UNKNOWN,
            source: SourceId::UNKNOWN,
            stream: StreamId::UNKNOWN,
            availability,
            reserved: [0; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct OfficeGeneration(pub u64);

impl OfficeGeneration {
    pub const INITIAL: Self = Self(0);

    #[inline]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct DomainMask(u8);

impl DomainMask {
    pub const NONE: Self = Self(0);
    pub const DESK: Self = Self(1 << 0);
    pub const MACRO: Self = Self(1 << 1);
    pub const FUND: Self = Self(1 << 2);
    pub const SYSTEMS: Self = Self(1 << 3);
    pub const LEDGER: Self = Self(1 << 4);
    pub const VENUE: Self = Self(1 << 5);
    pub const ALL: Self = Self((1 << 6) - 1);

    #[inline]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn from_bits_truncate(bits: u8) -> Self {
        Self(bits & Self::ALL.0)
    }

    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for DomainMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for DomainMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Debug)]
pub struct BarSeriesSnapshot {
    pub timeframe: Timeframe,
    pub blocks: Arc<[Arc<[CanonicalBar]>]>,
    pub tail: Arc<[CanonicalBar]>,
    pub len: usize,
    pub revision: u64,
}

impl BarSeriesSnapshot {
    pub fn empty(timeframe: Timeframe) -> Self {
        Self {
            timeframe,
            blocks: Arc::from([]),
            tail: Arc::from([]),
            len: 0,
            revision: 0,
        }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &CanonicalBar> {
        self.blocks
            .iter()
            .flat_map(|block| block.iter())
            .chain(self.tail.iter())
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&CanonicalBar> {
        if index >= self.len {
            return None;
        }
        let sealed_len = self.blocks.len() * BAR_SERIES_BLOCK_LEN;
        if index < sealed_len {
            return self
                .blocks
                .get(index / BAR_SERIES_BLOCK_LEN)
                .and_then(|block| block.get(index % BAR_SERIES_BLOCK_LEN));
        }
        self.tail.get(index - sealed_len)
    }

    #[inline]
    pub fn last(&self) -> Option<&CanonicalBar> {
        self.get(self.len.checked_sub(1)?)
    }
}

#[derive(Clone, Debug)]
pub struct DeskSnapshot {
    pub generation: u64,
    pub published_ns: i64,
    pub last_sequence: JournalSequence,
    pub availability: Availability,
    pub instruments: Arc<[Arc<super::market::IndexMarketSnapshot>]>,
}

impl DeskSnapshot {
    pub fn awaiting_first_receipt() -> Self {
        Self {
            generation: 0,
            published_ns: 0,
            last_sequence: JournalSequence::UNKNOWN,
            availability: Availability::AwaitingFirstReceipt,
            instruments: Arc::from([]),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DomainSnapshot {
    pub generation: u64,
    pub as_of_ns: i64,
    pub last_sequence: JournalSequence,
    pub availability: Availability,
}

impl DomainSnapshot {
    pub const fn unavailable(availability: Availability) -> Self {
        Self {
            generation: 0,
            as_of_ns: 0,
            last_sequence: JournalSequence::UNKNOWN,
            availability,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OfficeSnapshot {
    pub generation: OfficeGeneration,
    pub mode: OperatingMode,
    pub desk: Arc<DeskSnapshot>,
    pub macro_office: Arc<super::macro_office::MacroSnapshot>,
    pub venue: Arc<super::venue::VenueSnapshot>,
    pub fund: Arc<DomainSnapshot>,
    pub systems: Arc<DomainSnapshot>,
    pub ledger: Arc<super::ledger::LedgerSnapshot>,
}

impl OfficeSnapshot {
    pub fn empty(mode: OperatingMode) -> Self {
        Self {
            generation: OfficeGeneration::INITIAL,
            mode,
            desk: Arc::new(DeskSnapshot::awaiting_first_receipt()),
            macro_office: Arc::new(super::macro_office::MacroSnapshot::awaiting_first_receipt()),
            venue: Arc::new(super::venue::VenueSnapshot::unavailable(
                Availability::NotConfigured,
                "TradeLocker protected configuration is absent",
            )),
            fund: Arc::new(DomainSnapshot::unavailable(Availability::NotConfigured)),
            systems: Arc::new(DomainSnapshot::unavailable(
                Availability::AwaitingFirstReceipt,
            )),
            ledger: Arc::new(super::ledger::LedgerSnapshot::awaiting_store()),
        }
    }

    fn with_desk(&self, desk: Arc<DeskSnapshot>) -> Self {
        Self {
            generation: self.generation.next(),
            mode: self.mode,
            desk,
            macro_office: Arc::clone(&self.macro_office),
            venue: Arc::clone(&self.venue),
            fund: Arc::clone(&self.fund),
            systems: Arc::clone(&self.systems),
            ledger: Arc::clone(&self.ledger),
        }
    }

    fn with_ledger(&self, ledger: Arc<super::ledger::LedgerSnapshot>) -> Self {
        Self {
            generation: self.generation.next(),
            mode: self.mode,
            desk: Arc::clone(&self.desk),
            macro_office: Arc::clone(&self.macro_office),
            venue: Arc::clone(&self.venue),
            fund: Arc::clone(&self.fund),
            systems: Arc::clone(&self.systems),
            ledger,
        }
    }

    fn with_macro(&self, macro_office: Arc<super::macro_office::MacroSnapshot>) -> Self {
        Self {
            generation: self.generation.next(),
            mode: self.mode,
            desk: Arc::clone(&self.desk),
            macro_office,
            venue: Arc::clone(&self.venue),
            fund: Arc::clone(&self.fund),
            systems: Arc::clone(&self.systems),
            ledger: Arc::clone(&self.ledger),
        }
    }

    fn with_venue(&self, venue: Arc<super::venue::VenueSnapshot>) -> Self {
        let domain = Arc::new(DomainSnapshot {
            generation: venue.generation,
            as_of_ns: venue.as_of_ns,
            last_sequence: JournalSequence::UNKNOWN,
            availability: venue.availability,
        });
        Self {
            generation: self.generation.next(),
            mode: self.mode,
            desk: Arc::clone(&self.desk),
            macro_office: Arc::clone(&self.macro_office),
            venue,
            fund: Arc::clone(&domain),
            systems: domain,
            ledger: Arc::clone(&self.ledger),
        }
    }

    fn with_venue_and_ledger(
        &self,
        venue: Arc<super::venue::VenueSnapshot>,
        ledger: Arc<super::ledger::LedgerSnapshot>,
    ) -> Self {
        let domain = Arc::new(DomainSnapshot {
            generation: venue.generation,
            as_of_ns: venue.as_of_ns,
            last_sequence: ledger.last_sequence,
            availability: venue.availability,
        });
        Self {
            generation: self.generation.next(),
            mode: self.mode,
            desk: Arc::clone(&self.desk),
            macro_office: Arc::clone(&self.macro_office),
            venue,
            fund: Arc::clone(&domain),
            systems: domain,
            ledger,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotNotice {
    pub generation: OfficeGeneration,
    pub changed: DomainMask,
}

struct SubscriberState {
    pending: AtomicU8,
    generation: AtomicU64,
    wake: Sender<()>,
}

pub struct SnapshotSubscription {
    state: Arc<SubscriberState>,
    wake: Receiver<()>,
}

impl SnapshotSubscription {
    pub fn try_take_notice(&self) -> Option<SnapshotNotice> {
        match self.wake.try_recv() {
            Ok(()) => self.take_pending(),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    #[cfg(any(feature = "desktop", test))]
    pub(crate) fn wait_or_cancel(&self, cancel: &Receiver<()>) -> Option<SnapshotNotice> {
        select! {
            recv(self.wake) -> wake => wake.ok().and_then(|()| self.take_pending()),
            recv(cancel) -> _ => None,
        }
    }

    fn take_pending(&self) -> Option<SnapshotNotice> {
        let changed = DomainMask::from_bits_truncate(self.state.pending.swap(0, Ordering::AcqRel));
        let generation = OfficeGeneration(self.state.generation.load(Ordering::Acquire));
        (!changed.is_empty()).then_some(SnapshotNotice {
            generation,
            changed,
        })
    }
}

pub trait OfficeSnapshotPort: Send + Sync {
    fn current(&self) -> Arc<OfficeSnapshot>;
    fn subscribe(&self) -> SnapshotSubscription;
}

pub struct SnapshotStore {
    current: ArcSwap<OfficeSnapshot>,
    subscribers: Mutex<Vec<Weak<SubscriberState>>>,
}

impl SnapshotStore {
    pub fn new(initial: OfficeSnapshot) -> (Arc<Self>, SnapshotPublisher) {
        let store = Arc::new(Self {
            current: ArcSwap::from_pointee(initial),
            subscribers: Mutex::new(Vec::new()),
        });
        let publisher = SnapshotPublisher {
            store: Arc::clone(&store),
        };
        (store, publisher)
    }

    fn publish(&self, snapshot: Arc<OfficeSnapshot>, changed: DomainMask) {
        debug_assert!(!changed.is_empty());
        let generation = snapshot.generation.0;
        self.current.store(snapshot);

        let mut subscribers = self
            .subscribers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        subscribers.retain(|subscriber| {
            let Some(subscriber) = subscriber.upgrade() else {
                return false;
            };
            subscriber
                .pending
                .fetch_or(changed.bits(), Ordering::AcqRel);
            subscriber.generation.store(generation, Ordering::Release);
            match subscriber.wake.try_send(()) {
                Ok(()) | Err(TrySendError::Full(())) => true,
                Err(TrySendError::Disconnected(())) => false,
            }
        });
    }
}

impl OfficeSnapshotPort for SnapshotStore {
    #[inline]
    fn current(&self) -> Arc<OfficeSnapshot> {
        self.current.load_full()
    }

    fn subscribe(&self) -> SnapshotSubscription {
        let (wake_tx, wake_rx) = bounded(1);
        let state = Arc::new(SubscriberState {
            pending: AtomicU8::new(0),
            generation: AtomicU64::new(self.current.load().generation.0),
            wake: wake_tx,
        });
        self.subscribers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Arc::downgrade(&state));
        SnapshotSubscription {
            state,
            wake: wake_rx,
        }
    }
}

/// Unique single-writer handle. All domain updates eventually flow through one
/// assembler so concurrent publishers cannot overwrite each other's arcs.
pub struct SnapshotPublisher {
    store: Arc<SnapshotStore>,
}

impl SnapshotPublisher {
    pub fn publish_desk(&mut self, desk: Arc<DeskSnapshot>) -> OfficeGeneration {
        let current = self.store.current();
        let next = Arc::new(current.with_desk(desk));
        let generation = next.generation;
        self.store.publish(next, DomainMask::DESK);
        generation
    }

    pub fn publish_ledger(
        &mut self,
        ledger: Arc<super::ledger::LedgerSnapshot>,
    ) -> OfficeGeneration {
        let current = self.store.current();
        let next = Arc::new(current.with_ledger(ledger));
        let generation = next.generation;
        self.store.publish(next, DomainMask::LEDGER);
        generation
    }

    pub fn publish_macro(
        &mut self,
        macro_office: Arc<super::macro_office::MacroSnapshot>,
    ) -> OfficeGeneration {
        let current = self.store.current();
        let next = Arc::new(current.with_macro(macro_office));
        let generation = next.generation;
        self.store.publish(next, DomainMask::MACRO);
        generation
    }

    pub fn publish_venue(&mut self, venue: Arc<super::venue::VenueSnapshot>) -> OfficeGeneration {
        let current = self.store.current();
        let next = Arc::new(current.with_venue(venue));
        let generation = next.generation;
        self.store.publish(
            next,
            DomainMask::VENUE | DomainMask::FUND | DomainMask::SYSTEMS,
        );
        generation
    }

    pub fn publish_venue_and_ledger(
        &mut self,
        venue: Arc<super::venue::VenueSnapshot>,
        ledger: Arc<super::ledger::LedgerSnapshot>,
    ) -> OfficeGeneration {
        let current = self.store.current();
        let next = Arc::new(current.with_venue_and_ledger(venue, ledger));
        let generation = next.generation;
        self.store.publish(
            next,
            DomainMask::VENUE | DomainMask::FUND | DomainMask::SYSTEMS | DomainMask::LEDGER,
        );
        generation
    }
}

const _: () = assert!(std::mem::size_of::<ValueMeta>() <= 48);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_never_treats_missing_as_a_value() {
        assert!(Availability::Live.has_value());
        assert!(Availability::Stale.has_value());
        assert!(!Availability::NotConfigured.has_value());
        assert!(!Availability::AwaitingFirstReceipt.has_value());
    }

    #[test]
    fn subscriber_wakeups_coalesce_without_losing_domain_bits() {
        let (store, mut publisher) =
            SnapshotStore::new(OfficeSnapshot::empty(OperatingMode::OfflineReplay));
        let subscription = store.subscribe();

        let first = Arc::new(DeskSnapshot {
            generation: 1,
            published_ns: 10,
            last_sequence: JournalSequence(1),
            availability: Availability::Replaying,
            instruments: Arc::from([]),
        });
        let second = Arc::new(DeskSnapshot {
            generation: 2,
            published_ns: 20,
            last_sequence: JournalSequence(2),
            availability: Availability::Replaying,
            instruments: Arc::from([]),
        });
        assert_eq!(publisher.publish_desk(first), OfficeGeneration(1));
        assert_eq!(publisher.publish_desk(second), OfficeGeneration(2));

        assert_eq!(
            subscription.try_take_notice(),
            Some(SnapshotNotice {
                generation: OfficeGeneration(2),
                changed: DomainMask::DESK,
            })
        );
        assert!(subscription.try_take_notice().is_none());
        assert_eq!(store.current().desk.generation, 2);
    }

    #[test]
    fn desk_publication_reuses_unchanged_domain_arcs() {
        let (store, mut publisher) =
            SnapshotStore::new(OfficeSnapshot::empty(OperatingMode::LiveData));
        let before = store.current();
        publisher.publish_desk(Arc::new(DeskSnapshot::awaiting_first_receipt()));
        let after = store.current();
        assert!(Arc::ptr_eq(&before.macro_office, &after.macro_office));
        assert!(Arc::ptr_eq(&before.fund, &after.fund));
        assert!(Arc::ptr_eq(&before.systems, &after.systems));
        assert!(Arc::ptr_eq(&before.ledger, &after.ledger));
    }

    #[test]
    fn cancellation_unblocks_a_waiting_subscription_without_polling() {
        let (store, _) = SnapshotStore::new(OfficeSnapshot::empty(OperatingMode::LiveData));
        let subscription = store.subscribe();
        let (cancel_tx, cancel_rx) = bounded(1);
        let waiter = std::thread::spawn(move || subscription.wait_or_cancel(&cancel_rx));
        cancel_tx.send(()).unwrap();
        assert_eq!(waiter.join().unwrap(), None);
    }
}
