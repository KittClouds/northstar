use super::*;
use crate::data_plane::ids::instruments;
use serde_json::json;

fn layout(columns: &[&str]) -> Value {
    json!({
        "columns": columns.iter().map(|id| json!({ "id": id, "description": id })).collect::<Vec<_>>()
    })
}

fn response(data: Value) -> Value {
    json!({ "s": "ok", "d": data })
}

fn fixture() -> TradeLockerWireRefresh {
    let config = response(json!({
        "accountDetailsConfig": layout(&[
            "balance", "projectedBalance", "availableFunds", "todayNet", "todayFees", "openNetPnL"
        ]),
        "positionsConfig": layout(&[
            "id", "tradableInstrumentId", "side", "qty", "avgPrice", "unrealizedPl", "openDate"
        ]),
        "ordersConfig": layout(&[
            "id", "tradableInstrumentId", "positionId", "side", "type", "status", "qty", "filledQty", "avgPrice", "createdDate", "isOpen"
        ]),
        "ordersHistoryConfig": layout(&[
            "id", "tradableInstrumentId", "positionId", "side", "type", "status", "qty", "filledQty", "avgPrice", "createdDate", "isOpen"
        ]),
        "filledOrdersConfig": layout(&[
            "id", "tradableInstrumentId", "orderId", "positionId", "side", "qty", "price", "createdDate"
        ])
    }));
    TradeLockerWireRefresh {
        ts_started_ns: 1_000,
        ts_received_ns: 2_000,
        account_id: 77,
        acc_num: 88,
        account_name: "SANITIZED LIVE".into(),
        account_currency: "USD".into(),
        config,
        instruments: response(json!({
            "instruments": [{
                "tradableInstrumentId": 101,
                "name": "NAS100",
                "routes": [{ "id": 201, "type": "INFO" }, { "id": 202, "type": "TRADE" }]
            }]
        })),
        instrument_details: vec![(
            101,
            response(json!({
                "tradableInstrumentId": 101,
                "tradeSessionId": 301,
                "minLot": 0.01,
                "lotStep": 0.01,
                "tickSize": [{"leftRangeLimit": 0.0, "tickSize": 0.1}],
                "symbolStatus": "FULLY_OPEN"
            })),
        )]
        .into_boxed_slice(),
        account_state: response(json!({
            "accountDetailsData": [10_000.0, 10_025.0, 9_000.0, 25.0, -2.0, 10.0]
        })),
        positions: response(json!({
            "positions": [[501, 101, "buy", 0.2, 20_000.5, 10.0, 1_700_000_000_000i64]]
        })),
        orders: response(json!({
            "orders": [[401, 101, 501, "buy", "limit", "New", 0.2, 0.0, 0.0, 1_700_000_000_000i64, true]]
        })),
        order_history: response(json!({ "ordersHistory": [] })),
        executions: response(json!({
            "executions": [[601, 101, 401, 501, "buy", 0.2, 20_000.5, 1_700_000_000_100i64]]
        })),
        quotes: vec![(101, response(json!({ "bp": 20_010.0, "ap": 20_011.0 })))].into_boxed_slice(),
    }
}

fn decoder(known: bool) -> TradeLockerDecoder {
    TradeLockerDecoder::new(
        vec![TradeLockerBinding {
            instrument: instruments::US100,
            provider_symbol: "NAS100".into(),
            price_scale: 10_000,
        }],
        if known {
            HashSet::from([401])
        } else {
            HashSet::new()
        },
        if known {
            HashSet::from([501])
        } else {
            HashSet::new()
        },
    )
    .unwrap()
}

#[test]
fn complete_epoch_is_typed_and_detects_unknown_venue_objects() {
    let epoch = decoder(false).decode(&fixture()).unwrap();
    assert_eq!(epoch.contracts.len(), 1);
    assert_eq!(epoch.quotes[0].bid_scaled, 200_100_000);
    assert_eq!(epoch.account.projected_balance_micros, 10_025_000_000);
    assert_eq!(epoch.positions[0].quantity_micros, 200_000);
    assert_eq!(epoch.unknown_orders.as_ref(), &[401]);
    assert_eq!(epoch.unknown_positions.as_ref(), &[501]);
}

#[test]
fn known_objects_clear_account_truth_gate() {
    let epoch = decoder(true).decode(&fixture()).unwrap();
    assert!(epoch.unknown_orders.is_empty());
    assert!(epoch.unknown_positions.is_empty());
}

#[test]
fn missing_quote_rejects_entire_epoch() {
    let mut refresh = fixture();
    refresh.quotes = Box::new([]);
    assert!(matches!(
        decoder(false).decode(&refresh),
        Err(TradeLockerDecodeError::IncompleteQuoteCapture)
    ));
}

#[test]
fn config_column_reordering_drives_decoding() {
    let mut refresh = fixture();
    refresh.positions = response(json!({
        "positions": [[101, 501, "buy", 0.2, 20_000.5, 10.0, 1_700_000_000_000i64]]
    }));
    refresh.config["d"]["positionsConfig"] = layout(&[
        "tradableInstrumentId",
        "id",
        "side",
        "qty",
        "avgPrice",
        "unrealizedPl",
        "openDate",
    ]);
    let epoch = decoder(false).decode(&refresh).unwrap();
    assert_eq!(epoch.positions[0].id, 501);
    assert_eq!(epoch.positions[0].tradable_instrument_id, 101);
}

#[test]
fn tiered_tick_size_uses_the_epoch_midpoint_range() {
    let detail = json!({
        "tickSize": [
            {"leftRangeLimit": null, "tickSize": "0.01"},
            {"leftRangeLimit": "10000.0", "tickSize": "0.1"}
        ]
    });
    assert_eq!(contract_tick_size(&detail, 5_000.0).unwrap(), 0.01);
    assert_eq!(contract_tick_size(&detail, 20_010.5).unwrap(), 0.1);
    assert_eq!(decimal_precision(0.1).unwrap(), 1);
}
