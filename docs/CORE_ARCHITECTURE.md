# Northstar core architecture

Northstar owns the product domain and embeds trading infrastructure behind a
small compatibility boundary. TradeLocker Studio is not an architectural
dependency.

```text
Massive / TradeLocker / official sources
                  |
                  v
Northstar L0 receipts -> canonical decoders -> L1 journal
                                                |
                                 +--------------+--------------+
                                 |                             |
                              live tail                  replay reader
                                 +--------------+--------------+
                                                |
                                  deterministic L2 + UI snapshot

Later only:
Northstar TradeIntent -> Nautilus risk/portfolio/execution -> TradeLocker
```

The implemented data-plane contract is frozen in
`docs/CANONICAL_DATA_PLANE.md`. Nautilus is a later consumer and does not own
provider ingestion, canonical bars, journal time, provenance, or replay.

## Verified Nautilus cut

The current machine uses `rustc 1.96.0`. Nautilus `0.61.0` declares Rust
`1.97.1`, so it is not compatible with this toolchain. The compatible seam is
the exact `0.60.0` component set:

```toml
nautilus-common = "=0.60.0"
nautilus-model = "=0.60.0"
nautilus-data = "=0.60.0"
nautilus-execution = "=0.60.0"
nautilus-portfolio = "=0.60.0"
nautilus-risk = "=0.60.0"
nautilus-system = "=0.60.0"
nautilus-trading = "=0.60.0"
nautilus-backtest = "=0.60.0"
nautilus-live = "=0.60.0"
```

These dependencies belong in a later `northstar-nautilus` integration crate,
not in the GPUI page crate. The official Rust API is under active development,
so upgrades require an explicit compatibility pass. Nautilus crates are
LGPL-3.0-or-later; packaging and dynamic/static-linking obligations must be
reviewed before distribution.

The verified `0.60.0` traits separate `InstrumentProvider`, `DataClient`, and
`ExecutionClient`. Execution reconciliation produces order, fill, and position
reports. Northstar's ports intentionally mirror only that stable shape.

## Authority split

- Massive: reference quotes, bars, history, chart and strategy inputs.
- Nautilus: event routing, order lifecycle, risk, portfolio, cache, backtest and
  live-node semantics.
- TradeLocker: account, instruments, accepted orders, fills, positions, venue
  prices, and execution truth.
- Northstar: explicit symbol bindings, operating mode, safety gates, lineage,
  UI snapshots, and command authorization.

TradeLocker market data is a venue sanity check, not the research data plane.

## Execution lineage and reconciliation

TradeLocker documents `orderId` and `positionId` as different identities: an
order is created first and a filled order can result in a position with another
ID. Northstar represents them as separate Rust newtypes and never joins them by
their raw integer alone.

TradeLocker's `strategyId` accepts at most 31 characters and is visible across
orders, order history, and positions. `StrategyLineage` is a 32-byte,
allocation-free value containing a validated 31-byte ASCII identifier plus its
length. It is copied from intent through every venue report.

Startup is fail closed:

```text
Disconnected -> Reconciling(epoch) -> Ready(epoch)
```

Order flow remains blocked for missing opening orders, mismatched epochs,
unknown command outcomes, stale Massive data, an unhealthy Nautilus engine, an
unsynchronized TradeLocker adapter, clock drift, or disagreement between
Northstar's cached instrument terms and TradeLocker discovery. Contract
agreement covers minimum quantity, quantity step, price tick/precision, margin
rate, route availability, and session state. Duplicate fill IDs are idempotent.
A restart always returns to `Disconnected`; live arming is never restored.

Passing every operational gate means only that the system is eligible to arm.
It does not arm automatically. Market setup, strategy, portfolio/risk, and
execution policy retain independent vetoes before any order can be routed.

## Explicitly excluded from this slice

- indicators;
- Markov or HSMM models;
- event grammar or market-state inference;
- network authentication or credential storage;
- real Massive or TradeLocker HTTP/WebSocket calls;
- a concrete Nautilus adapter implementation;
- TradeLocker Studio.

Those exclusions keep this slice focused on durable ownership, identities,
ports, reconciliation, and live-safety behavior.
