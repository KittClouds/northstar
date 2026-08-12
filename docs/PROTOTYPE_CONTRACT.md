# Prototype contract

## Authority

- Massive is reference market-data authority.
- TradeLocker is broker instrument, account, order, fill, and position authority.
- NautilusTrader is strategy, portfolio, risk, execution lifecycle, and
  reconciliation authority.
- GPUI is presentation and guarded command intent only.

## Symbol binding

An index binding is explicit and versioned. It must contain the internal
`IndexKey`, Massive ticker, TradeLocker tradable instrument identity, information
and trade route IDs, precision, point value, session calendar, and permitted
reference/execution basis. Symbol text is never an identity join.

## Modes

```text
Disconnected -> Syncing -> ReadOnly -> Simulation -> Shadow -> LiveArmed
                                  \-----------------------------> Halted
```

Any stale provider, incomplete reconciliation, unknown order outcome, clock
violation, instrument contract mismatch, or breached risk gate removes live
permission. Operational readiness means only eligible to arm: strategy,
portfolio/risk, and execution policy remain independent downstream vetoes. A
restart begins at `Disconnected`, never `LiveArmed`.

## Chart boundary

The prototype uses a native GPUI canvas surface with an incremental-compatible
data contract. Logical chart state is separated from GPUI painting so history,
delta, overlay, and cursor contracts do not leak into strategies, providers, or
host integration code. Lightweight Charts and Lumen Charts are behavioral and
architectural references only, not runtime dependencies.

## Acceptance gates for a connected prototype

- Replay determinism for decisions and order intentions.
- Duplicate and out-of-order stream fixtures.
- Partial fill, cancel race, unknown command outcome, and restart-with-position.
- Session-aware Massive silence vs genuinely stale data.
- Reference/execution basis ceiling.
- TradeLocker startup reconciliation before order eligibility.
- UI snapshot-to-paint within one 16.7 ms frame at p95 for the six-index target.
- No full history replacement on a live bar update.

## Core ownership

Northstar owns operating mode, provider freshness, clock, lineage, explicit
symbol bindings, reconciliation readiness, and command authorization. Nautilus
is embedded infrastructure behind `NautilusEnginePort`; it does not own the UI
or Northstar domain. Massive is data-only and TradeLocker is execution-only in
the primary route.

The current core slice intentionally excludes indicators, Markov/HSMM models,
TradeLocker Studio, credentials, networking, and live order transmission.
