use super::StrategyLineage;
use hashbrown::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct IntentId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct VenueOrderId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct VenuePositionId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderStatus {
    Accepted,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrderSnapshot {
    pub venue_order_id: VenueOrderId,
    pub intent_id: Option<IntentId>,
    pub lineage: StrategyLineage,
    pub status: OrderStatus,
    pub filled_quantity_micros: u64,
    pub position_id: Option<VenuePositionId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PositionSnapshot {
    pub venue_position_id: VenuePositionId,
    pub lineage: StrategyLineage,
    pub quantity_micros: i64,
    pub opened_by: VenueOrderId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FillReport {
    pub fill_id: u64,
    pub venue_order_id: VenueOrderId,
    pub venue_position_id: VenuePositionId,
    pub quantity_micros: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReconciliationState {
    #[default]
    Disconnected,
    Reconciling {
        epoch: u64,
    },
    Ready {
        epoch: u64,
    },
}

/// Maintains venue order and position identities as distinct namespaces.
pub struct ExecutionReconciler {
    state: ReconciliationState,
    orders: HashMap<VenueOrderId, OrderSnapshot>,
    positions: HashMap<VenuePositionId, PositionSnapshot>,
    seen_fills: HashSet<u64>,
    unknown_intents: HashSet<IntentId>,
}

impl Default for ExecutionReconciler {
    fn default() -> Self {
        Self {
            state: ReconciliationState::Disconnected,
            orders: HashMap::new(),
            positions: HashMap::new(),
            seen_fills: HashSet::new(),
            unknown_intents: HashSet::new(),
        }
    }
}

impl ExecutionReconciler {
    pub fn begin(&mut self, epoch: u64) {
        self.state = ReconciliationState::Reconciling { epoch };
        self.orders.clear();
        self.positions.clear();
        self.seen_fills.clear();
    }

    pub fn apply_orders(&mut self, orders: &[OrderSnapshot]) -> Result<(), ReconciliationError> {
        self.require_reconciling()?;
        self.orders.reserve(orders.len());
        for order in orders {
            self.orders.insert(order.venue_order_id, *order);
        }
        Ok(())
    }

    pub fn apply_positions(
        &mut self,
        positions: &[PositionSnapshot],
    ) -> Result<(), ReconciliationError> {
        self.require_reconciling()?;
        for position in positions {
            if !self.orders.contains_key(&position.opened_by) {
                return Err(ReconciliationError::MissingOpeningOrder(position.opened_by));
            }
        }
        self.positions.reserve(positions.len());
        for position in positions {
            self.positions.insert(position.venue_position_id, *position);
        }
        Ok(())
    }

    pub fn apply_fill(&mut self, fill: FillReport) -> Result<bool, ReconciliationError> {
        if self.seen_fills.contains(&fill.fill_id) {
            return Ok(false);
        }
        let order = self
            .orders
            .get_mut(&fill.venue_order_id)
            .ok_or(ReconciliationError::UnknownOrder(fill.venue_order_id))?;
        order.filled_quantity_micros = order
            .filled_quantity_micros
            .saturating_add(fill.quantity_micros);
        order.position_id = Some(fill.venue_position_id);
        order.status = OrderStatus::PartiallyFilled;
        self.seen_fills.insert(fill.fill_id);
        Ok(true)
    }

    pub fn finish(&mut self, epoch: u64) -> Result<(), ReconciliationError> {
        match self.state {
            ReconciliationState::Reconciling { epoch: current } if current == epoch => {
                self.state = ReconciliationState::Ready { epoch };
                Ok(())
            }
            other => Err(ReconciliationError::WrongEpoch {
                expected: epoch,
                actual: other,
            }),
        }
    }

    pub fn mark_unknown(&mut self, intent_id: IntentId) {
        self.unknown_intents.insert(intent_id);
    }

    pub fn resolve_unknown(&mut self, intent_id: IntentId) -> bool {
        self.unknown_intents.remove(&intent_id)
    }

    #[inline]
    pub fn ready_for_order_flow(&self) -> bool {
        matches!(self.state, ReconciliationState::Ready { .. }) && self.unknown_intents.is_empty()
    }

    #[inline]
    pub fn unknown_count(&self) -> usize {
        self.unknown_intents.len()
    }

    #[inline]
    pub fn state(&self) -> ReconciliationState {
        self.state
    }

    #[inline]
    pub fn order(&self, id: VenueOrderId) -> Option<&OrderSnapshot> {
        self.orders.get(&id)
    }

    #[inline]
    pub fn position(&self, id: VenuePositionId) -> Option<&PositionSnapshot> {
        self.positions.get(&id)
    }

    fn require_reconciling(&self) -> Result<u64, ReconciliationError> {
        match self.state {
            ReconciliationState::Reconciling { epoch } => Ok(epoch),
            actual => Err(ReconciliationError::NotReconciling(actual)),
        }
    }
}

#[derive(Clone, Copy, Debug, thiserror::Error, Eq, PartialEq)]
pub enum ReconciliationError {
    #[error("execution state is not reconciling: {0:?}")]
    NotReconciling(ReconciliationState),
    #[error("reconciliation epoch mismatch: expected {expected}, actual {actual:?}")]
    WrongEpoch {
        expected: u64,
        actual: ReconciliationState,
    },
    #[error("position references missing opening order {0:?}")]
    MissingOpeningOrder(VenueOrderId),
    #[error("fill references unknown order {0:?}")]
    UnknownOrder(VenueOrderId),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lineage() -> StrategyLineage {
        StrategyLineage::new("ORB-US30-L").expect("lineage")
    }

    fn order() -> OrderSnapshot {
        OrderSnapshot {
            venue_order_id: VenueOrderId(7),
            intent_id: Some(IntentId(11)),
            lineage: lineage(),
            status: OrderStatus::Accepted,
            filled_quantity_micros: 0,
            position_id: None,
        }
    }

    #[test]
    fn order_and_position_ids_are_reconciled_as_distinct_truth() {
        let mut reconciler = ExecutionReconciler::default();
        reconciler.begin(3);
        reconciler.apply_orders(&[order()]).expect("orders");
        reconciler
            .apply_positions(&[PositionSnapshot {
                venue_position_id: VenuePositionId(99),
                lineage: lineage(),
                quantity_micros: 1_000_000,
                opened_by: VenueOrderId(7),
            }])
            .expect("positions");
        reconciler.finish(3).expect("finish");

        assert!(reconciler.ready_for_order_flow());
        assert!(reconciler.order(VenueOrderId(7)).is_some());
        assert!(reconciler.position(VenuePositionId(99)).is_some());
    }

    #[test]
    fn duplicate_fills_are_idempotent_and_unknown_outcomes_fail_closed() {
        let mut reconciler = ExecutionReconciler::default();
        reconciler.begin(4);
        reconciler.apply_orders(&[order()]).expect("orders");
        let fill = FillReport {
            fill_id: 88,
            venue_order_id: VenueOrderId(7),
            venue_position_id: VenuePositionId(99),
            quantity_micros: 500_000,
        };
        assert!(reconciler.apply_fill(fill).expect("first fill"));
        assert!(!reconciler.apply_fill(fill).expect("duplicate fill"));
        reconciler.finish(4).expect("finish");
        reconciler.mark_unknown(IntentId(12));
        assert!(!reconciler.ready_for_order_flow());
        assert!(reconciler.resolve_unknown(IntentId(12)));
        assert!(reconciler.ready_for_order_flow());
        assert_eq!(
            reconciler
                .order(VenueOrderId(7))
                .unwrap()
                .filled_quantity_micros,
            500_000
        );
    }

    #[test]
    fn rejected_fill_is_retryable_after_order_truth_arrives() {
        let mut reconciler = ExecutionReconciler::default();
        reconciler.begin(8);
        let fill = FillReport {
            fill_id: 91,
            venue_order_id: VenueOrderId(7),
            venue_position_id: VenuePositionId(100),
            quantity_micros: 250_000,
        };
        assert_eq!(
            reconciler.apply_fill(fill),
            Err(ReconciliationError::UnknownOrder(VenueOrderId(7)))
        );
        reconciler.apply_orders(&[order()]).expect("order truth");
        assert!(reconciler.apply_fill(fill).expect("retry fill"));
    }
}
