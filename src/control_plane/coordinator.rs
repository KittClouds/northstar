use super::{ExecutionReconciler, IntentId, StrategyLineage};
use crate::contracts::IndexKey;
use smallvec::SmallVec;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CoreMode {
    #[default]
    Disconnected,
    ReadOnly,
    Simulation,
    Shadow,
    LiveArmed,
    Halted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrderIntent {
    pub intent_id: IntentId,
    pub index: IndexKey,
    pub lineage: StrategyLineage,
    pub side: OrderSide,
    pub quantity_micros: u64,
    pub limit_price: Option<f64>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub ts_init_ns: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteDisposition {
    Simulate,
    ShadowOnly,
    Transmit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreBlock {
    MassiveStale,
    NautilusNotReady,
    TradeLockerNotReady,
    InstrumentContractMismatch,
    ReconciliationIncomplete,
    UnknownOrderOutcome,
    ClockUnsynchronized,
    Halted,
    LiveNotArmed,
}

/// Northstar owns this gate; Nautilus and venue adapters remain replaceable.
pub struct CoreCoordinator {
    mode: CoreMode,
    massive_fresh: bool,
    nautilus_ready: bool,
    tradelocker_ready: bool,
    instrument_contracts_agree: bool,
    clock_synchronized: bool,
    reconciler: ExecutionReconciler,
}

impl Default for CoreCoordinator {
    fn default() -> Self {
        Self {
            mode: CoreMode::Disconnected,
            massive_fresh: false,
            nautilus_ready: false,
            tradelocker_ready: false,
            instrument_contracts_agree: false,
            clock_synchronized: false,
            reconciler: ExecutionReconciler::default(),
        }
    }
}

impl CoreCoordinator {
    pub fn set_provider_health(
        &mut self,
        massive_fresh: bool,
        nautilus_ready: bool,
        tradelocker_ready: bool,
        clock_synchronized: bool,
    ) {
        self.massive_fresh = massive_fresh;
        self.nautilus_ready = nautilus_ready;
        self.tradelocker_ready = tradelocker_ready;
        self.clock_synchronized = clock_synchronized;
        if self.mode == CoreMode::Disconnected {
            self.mode = CoreMode::ReadOnly;
        }
        if !self.block_reasons().is_empty() && self.mode == CoreMode::LiveArmed {
            self.mode = CoreMode::ReadOnly;
        }
    }

    pub fn set_non_live_mode(&mut self, mode: CoreMode) -> Result<(), CoreBlock> {
        if mode == CoreMode::LiveArmed {
            return Err(CoreBlock::LiveNotArmed);
        }
        self.mode = mode;
        Ok(())
    }

    pub fn set_instrument_contract_agreement(&mut self, agrees: bool) {
        self.instrument_contracts_agree = agrees;
        if !agrees && self.mode == CoreMode::LiveArmed {
            self.mode = CoreMode::ReadOnly;
        }
    }

    pub fn arm_live(&mut self) -> Result<(), SmallVec<[CoreBlock; 8]>> {
        let reasons = self.block_reasons();
        if reasons.is_empty() {
            self.mode = CoreMode::LiveArmed;
            Ok(())
        } else {
            Err(reasons)
        }
    }

    pub fn route_intent(&self, _intent: &OrderIntent) -> Result<RouteDisposition, CoreBlock> {
        match self.mode {
            CoreMode::Simulation => Ok(RouteDisposition::Simulate),
            CoreMode::Shadow => Ok(RouteDisposition::ShadowOnly),
            CoreMode::LiveArmed if self.block_reasons().is_empty() => {
                Ok(RouteDisposition::Transmit)
            }
            CoreMode::Halted => Err(CoreBlock::Halted),
            _ => Err(CoreBlock::LiveNotArmed),
        }
    }

    pub fn halt(&mut self) {
        self.mode = CoreMode::Halted;
    }

    pub fn restart(&mut self) {
        *self = Self::default();
    }

    pub fn block_reasons(&self) -> SmallVec<[CoreBlock; 8]> {
        let mut reasons = SmallVec::new();
        if !self.massive_fresh {
            reasons.push(CoreBlock::MassiveStale);
        }
        if !self.nautilus_ready {
            reasons.push(CoreBlock::NautilusNotReady);
        }
        if !self.tradelocker_ready {
            reasons.push(CoreBlock::TradeLockerNotReady);
        }
        if !self.instrument_contracts_agree {
            reasons.push(CoreBlock::InstrumentContractMismatch);
        }
        if !self.clock_synchronized {
            reasons.push(CoreBlock::ClockUnsynchronized);
        }
        if !matches!(
            self.reconciler.state(),
            super::ReconciliationState::Ready { .. }
        ) {
            reasons.push(CoreBlock::ReconciliationIncomplete);
        }
        if self.reconciler.unknown_count() > 0 {
            reasons.push(CoreBlock::UnknownOrderOutcome);
        }
        reasons
    }

    #[inline]
    pub fn reconciler(&self) -> &ExecutionReconciler {
        &self.reconciler
    }

    #[inline]
    pub fn reconciler_mut(&mut self) -> &mut ExecutionReconciler {
        &mut self.reconciler
    }

    #[inline]
    pub fn mode(&self) -> CoreMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent() -> OrderIntent {
        OrderIntent {
            intent_id: IntentId(41),
            index: IndexKey::Us100,
            lineage: StrategyLineage::new("MOM-US100-L").expect("lineage"),
            side: OrderSide::Buy,
            quantity_micros: 1_000_000,
            limit_price: None,
            stop_loss: Some(20_072.0),
            take_profit: Some(20_180.0),
            ts_init_ns: 1,
        }
    }

    #[test]
    fn live_route_requires_every_authority_gate() {
        let mut core = CoreCoordinator::default();
        core.set_provider_health(true, true, true, true);
        core.set_instrument_contract_agreement(true);
        assert_eq!(
            core.arm_live(),
            Err(SmallVec::from_slice(&[CoreBlock::ReconciliationIncomplete]))
        );
        core.reconciler_mut().begin(9);
        core.reconciler_mut().finish(9).expect("reconciled");
        core.arm_live().expect("live gate");
        assert_eq!(core.route_intent(&intent()), Ok(RouteDisposition::Transmit));
    }

    #[test]
    fn restart_never_restores_live_arming() {
        let mut core = CoreCoordinator::default();
        core.set_non_live_mode(CoreMode::Shadow)
            .expect("shadow mode");
        assert_eq!(
            core.route_intent(&intent()),
            Ok(RouteDisposition::ShadowOnly)
        );
        core.restart();
        assert_eq!(core.mode(), CoreMode::Disconnected);
        assert_eq!(core.route_intent(&intent()), Err(CoreBlock::LiveNotArmed));
    }

    #[test]
    fn live_mode_cannot_bypass_arming_and_contract_drift_disarms() {
        let mut core = CoreCoordinator::default();
        assert_eq!(
            core.set_non_live_mode(CoreMode::LiveArmed),
            Err(CoreBlock::LiveNotArmed)
        );

        core.set_provider_health(true, true, true, true);
        core.set_instrument_contract_agreement(true);
        core.reconciler_mut().begin(5);
        core.reconciler_mut().finish(5).expect("reconciled");
        core.arm_live().expect("armed");
        core.set_instrument_contract_agreement(false);
        assert_eq!(core.mode(), CoreMode::ReadOnly);
        assert_eq!(core.route_intent(&intent()), Err(CoreBlock::LiveNotArmed));
    }
}
