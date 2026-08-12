use super::{AutomaticCasePolicy, AutomaticLedgerEventCommand, VenueSnapshot};
use crate::data_plane::ids::SchemaVersion;
use crate::data_plane::providers::tradelocker::VenueSide;
use crate::ledger::{
    ActorId, ActorKind, CorrelationId, LedgerEventKind, LedgerEventStatus, LedgerFlags,
    SourceEventKey,
};
use std::sync::Arc;

const TRADELOCKER_ACTOR: ActorId = ActorId(20_001);

pub(super) fn venue_ledger_events(snapshot: &VenueSnapshot) -> Vec<AutomaticLedgerEventCommand> {
    let Some(account) = snapshot.account.as_ref() else {
        return Vec::new();
    };
    let mut events = Vec::with_capacity(
        1 + snapshot.positions.len()
            + snapshot.open_orders.len()
            + snapshot.final_orders.len()
            + snapshot.executions.len(),
    );
    let unknown = snapshot.unknown_orders.len() + snapshot.unknown_positions.len();
    events.push(command(
        source_key(b"epoch", snapshot.as_of_ns as u64, &snapshot.config_fingerprint),
        AutomaticCasePolicy::NoCase,
        CorrelationId::UNKNOWN,
        LedgerEventKind::Reconciliation,
        if unknown == 0 {
            LedgerEventStatus::Accepted
        } else {
            LedgerEventStatus::Incident
        },
        crate::data_plane::ids::InstrumentId::UNKNOWN,
        account.id,
        snapshot.as_of_ns,
        format!(
            "TradeLocker coherent epoch {}: {} contracts, {} quotes, {} positions, {} open orders, {} executions, {} unknown objects.",
            snapshot.generation,
            snapshot.contracts.len(),
            snapshot.quotes.len(),
            snapshot.positions.len(),
            snapshot.open_orders.len(),
            snapshot.executions.len(),
            unknown,
        ),
    ));
    for order in snapshot
        .open_orders
        .iter()
        .chain(snapshot.final_orders.iter())
    {
        let correlation = lifecycle_correlation(account.id, order.position_id, order.id);
        let mut identity = Vec::with_capacity(order.status.len() + 24);
        identity.extend_from_slice(order.status.as_bytes());
        identity.extend_from_slice(&order.created_ms.to_le_bytes());
        identity.extend_from_slice(&order.filled_quantity_micros.to_le_bytes());
        events.push(command(
            source_key(b"order", order.id, &identity),
            AutomaticCasePolicy::Correlate,
            correlation,
            LedgerEventKind::Order,
            if order.is_open {
                LedgerEventStatus::Pending
            } else {
                LedgerEventStatus::Accepted
            },
            order.instrument,
            account.id,
            millis_to_ns(order.created_ms, snapshot.as_of_ns),
            format!(
                "TradeLocker order {} {} {} {} @ {} / status {}.",
                order.id,
                side_label(order.side),
                quantity(order.quantity_micros),
                order.order_type,
                price(order.average_price_nanos),
                order.status,
            ),
        ));
    }
    for position in snapshot.positions.iter() {
        let correlation = lifecycle_correlation(account.id, position.id, position.id);
        let mut identity = Vec::with_capacity(32);
        identity.extend_from_slice(&position.quantity_micros.to_le_bytes());
        identity.extend_from_slice(&position.average_price_nanos.to_le_bytes());
        identity.extend_from_slice(&position.unrealized_pnl_micros.to_le_bytes());
        identity.extend_from_slice(&position.opened_ms.to_le_bytes());
        events.push(command(
            source_key(b"position", position.id, &identity),
            AutomaticCasePolicy::Correlate,
            correlation,
            LedgerEventKind::Position,
            LedgerEventStatus::Observed,
            position.instrument,
            account.id,
            millis_to_ns(position.opened_ms, snapshot.as_of_ns),
            format!(
                "TradeLocker position {} {} {} @ {} / open P&L {} {}.",
                position.id,
                side_label(position.side),
                quantity(position.quantity_micros),
                price(position.average_price_nanos),
                money(position.unrealized_pnl_micros),
                account.currency,
            ),
        ));
    }
    for execution in snapshot.executions.iter() {
        let correlation =
            lifecycle_correlation(account.id, execution.position_id, execution.order_id);
        let mut identity = Vec::with_capacity(24);
        identity.extend_from_slice(&execution.order_id.to_le_bytes());
        identity.extend_from_slice(&execution.position_id.to_le_bytes());
        identity.extend_from_slice(&execution.created_ms.to_le_bytes());
        events.push(command(
            source_key(b"execution", execution.id, &identity),
            AutomaticCasePolicy::Correlate,
            correlation,
            LedgerEventKind::Fill,
            LedgerEventStatus::Accepted,
            execution.instrument,
            account.id,
            millis_to_ns(execution.created_ms, snapshot.as_of_ns),
            format!(
                "TradeLocker execution {} filled order {} / position {} / {} {} @ {}.",
                execution.id,
                execution.order_id,
                execution.position_id,
                side_label(execution.side),
                quantity(execution.quantity_micros),
                price(execution.price_nanos),
            ),
        ));
    }
    events
}

#[allow(clippy::too_many_arguments)]
fn command(
    source_key: SourceEventKey,
    case_policy: AutomaticCasePolicy,
    correlation_id: CorrelationId,
    kind: LedgerEventKind,
    status: LedgerEventStatus,
    instrument_id: crate::data_plane::ids::InstrumentId,
    account_id: u64,
    ts_event_ns: i64,
    body: String,
) -> AutomaticLedgerEventCommand {
    AutomaticLedgerEventCommand {
        source_key,
        case_policy,
        correlation_id,
        actor_id: TRADELOCKER_ACTOR,
        actor_kind: ActorKind::Venue,
        kind,
        status,
        flags: LedgerFlags::MATERIAL,
        instrument_id,
        account_id,
        ts_event_ns,
        ts_received_ns: ts_event_ns,
        canonical_sequences: None,
        schema_version: SchemaVersion(1),
        body: Arc::from(body.into_bytes()),
    }
}

fn lifecycle_correlation(account_id: u64, position_id: u64, order_id: u64) -> CorrelationId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-tradelocker-lifecycle-v1");
    hasher.update(&account_id.to_le_bytes());
    if position_id != 0 {
        hasher.update(b"position");
        hasher.update(&position_id.to_le_bytes());
    } else {
        hasher.update(b"order");
        hasher.update(&order_id.to_le_bytes());
    }
    let hash = hasher.finalize();
    let mut id = [0u8; 16];
    id.copy_from_slice(&hash.as_bytes()[..16]);
    CorrelationId(u128::from_le_bytes(id))
}

fn source_key(namespace: &[u8], id: u64, material: &[u8]) -> SourceEventKey {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"northstar-tradelocker-ledger-v1");
    hasher.update(namespace);
    hasher.update(&id.to_le_bytes());
    hasher.update(material);
    let hash = hasher.finalize();
    let mut key = [0u8; 16];
    key.copy_from_slice(&hash.as_bytes()[..16]);
    SourceEventKey(u128::from_le_bytes(key))
}

fn millis_to_ns(milliseconds: i64, fallback: i64) -> i64 {
    milliseconds
        .checked_mul(1_000_000)
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn side_label(side: VenueSide) -> &'static str {
    match side {
        VenueSide::Buy => "BUY",
        VenueSide::Sell => "SELL",
        VenueSide::Unknown => "UNKNOWN",
    }
}

fn quantity(value: i64) -> String {
    format!("{:.6}", value as f64 / 1_000_000.0)
}
fn price(value: i64) -> String {
    format!("{:.5}", value as f64 / 1_000_000_000.0)
}
fn money(value: i64) -> String {
    format!("{:+.2}", value as f64 / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_lifecycle_correlation_prefers_position_identity() {
        assert_eq!(
            lifecycle_correlation(1, 7, 8),
            lifecycle_correlation(1, 7, 999)
        );
        assert_ne!(
            lifecycle_correlation(1, 7, 8),
            lifecycle_correlation(1, 9, 8)
        );
    }
}
