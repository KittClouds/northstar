# HeroFX venue-data authority plan

Status: architecture frozen; LIVE desktop account verified; REST capture pending

## Decision

HeroFX moves ahead of Massive in Northstar's market-data activation order because
the broker offers both MetaTrader 5 and TradeLocker account paths. Those paths
are complementary, not interchangeable:

```text
HeroFX MT5 account
  -> broker-native CFD observation sensor
  -> candidate signal-price authority

HeroFX TradeLocker account
  -> execution quote and instrument contract
  -> account, order, position, fill, and reconciliation authority

Massive
  -> optional independent benchmark/reference
  -> never required for HeroFX venue truth
```

MT5 and TradeLocker remain separate platform accounts with separate credentials,
symbols, routes, sessions, and observable prices. A HeroFX product being offered
on both platforms does not establish that the two feeds or account contracts are
identical.

## Three price roles

Northstar preserves source observations and assigns explicit roles instead of
merging quotes into an indistinguishable price:

| Role | Meaning | Initial authority |
|---|---|---|
| Reference price | Independent benchmark context | Massive when licensed and exactly bound |
| Signal price | Price history used by Northstar research and later deterministic strategies | HeroFX MT5 after rights and identity audit |
| Execution price | Bid/ask and executable contract seen by the trading account | HeroFX TradeLocker |

Roles are versioned bindings. They may point to the same source temporarily, but
the semantic distinction remains. An unlicensed or unverified source cannot be
promoted merely because it is convenient.

## Source disagreement is retained

Each raw tick retains source, account/server identity, provider symbol, binding
version, provider event time, receive wall-clock and monotonic time, prices,
volume, flags, sequence, receipt identity, and license-contract identity.

Derived cross-feed state may later contain basis, lag, spread, and health, but it
never overwrites source truth:

```text
reference basis
execution basis
median / p95 / p99 relative lag
venue-only excursion
spread regime
NORMAL / STALE / WIDE / DIVERGENT / UNTRUSTED
```

These are execution-quality observations, not strategies or trade signals in
this phase.

## MT5 sensor boundary

The preferred local bridge is a minimal MQL5 Expert Advisor connected to a Rust
named-pipe server. MQL5 officially supports local Windows named pipes through
`FileOpen`, and `SymbolInfoTick` exposes the complete latest `MqlTick` record.

```text
HeroFX MT5 terminal / sensor-only EA
  -> fixed-size binary frames
  -> local named pipe
  -> bounded Rust reader
  -> L0 raw receipt batches
  -> pure decoder
  -> canonical venue/market events
```

The EA does not calculate indicators, aggregate bars, contain strategy logic, or
send orders. AutoTrading can remain disabled. Northstar owns receipt IDs,
receive timestamps, canonicalization, journaling, replay, gap detection, and
publication.

The wire format will use a versioned fixed header, compact integer symbol ID,
millisecond provider timestamp, scaled integer prices, volume, tick flags,
sequence, and CRC. Strings and broker metadata are exchanged only during a cold
handshake. The hot tick path performs no formatting or allocation.

`OnTick` is not assumed to be a lossless tick-history transport: the MQL5 event
queue can coalesce a new tick while an earlier tick handler is running. The
sensor must remain extremely small, publish sequence/gap telemetry, and use
bounded history recovery where the account supports it.

## TradeLocker boundary

The HeroFX TradeLocker LIVE account and desktop market/history surfaces were
verified read-only on 2026-08-11. Northstar now captures `/trade/config` first
and treats it as the capability and rate-limit contract. Only then does it bind
account/route IDs, symbols, precision, tick size, lot steps, session state,
history limits, positions, active orders, fills, and final orders. The observed
desktop contract and remaining REST gates are recorded in
`TRADELOCKER_HEROFX_ACTIVATION_AUDIT_2026-08-11.md`.

TradeLocker owns execution/account truth. MT5 is not allowed to reconcile or
invent TradeLocker account state, even if both accounts belong to HeroFX.

## Rights gate

Architecture does not imply permission to retain or analyze a feed. Before the
first durable MT5 or TradeLocker capture, record the applicable HeroFX account
agreement and data-provider terms as a versioned `SourceContract`.

The default MetaTrader terms describe platform market data as display-only.
Therefore the MT5 bridge remains disabled until the HeroFX-specific account/data
agreement permits Northstar's personal internal non-display processing,
retention, replay, derived bars, and research use. If permission is narrower,
Northstar enforces the narrower scope.

No credentials are stored in project files or canonical journals. Secrets are
resolved from Windows-protected storage at runtime.

## Failure behavior

| Condition | Northstar behavior |
|---|---|
| MT5 terminal absent | Signal feed unavailable; no fixture fallback |
| Pipe disconnect | Publish stale, preserve last receipt, retry boundedly |
| Tick sequence gap | Emit data gap; recover if supported; never manufacture ticks |
| Unknown symbol | Retain quarantined L0 evidence; publish no canonical quote |
| Contract drift | Invalidate binding and require a reviewed new version |
| MT5/TradeLocker disagreement | Preserve both; update basis/health state |
| TradeLocker epoch incomplete | Publish no mixed account snapshot |
| Rights unknown or display-only | No durable canonical ingestion |

## Implementation order

1. Capture HeroFX account agreements and both platform instrument catalogs.
   TradeLocker desktop discovery is complete; rights and REST catalog capture
   remain open, while MT5 stays deferred.
2. Capture HeroFX TradeLocker `/trade/config` and read-only golden payloads.
3. Freeze exact TradeLocker symbol, route, session, and contract bindings.
4. Publish coherent execution/account epochs and automatic Ledger evidence.
5. Implement and fixture-test the fixed MT5 bridge protocol when that deferred
   phase is authorized.
6. Run a sensor-only MT5 soak with exact gap, latency, allocation, and restart
   proof.
7. Add Massive only as an independently licensed exact reference feed.

## Exit test

> Given retained, licensed HeroFX MT5 and TradeLocker input streams, Northstar
> deterministically reconstructs signal and execution market state without
> conflating their prices or authorities, exposes every gap and binding version,
> and reproduces the same frontend and Ledger state after restart.

## Official references

- <https://herofx.co/metatrader-5/>
- <https://herofx.co/tradelocker/>
- <https://herofx.co/10x/>
- <https://support.tradelocker.com/en/articles/13742938-mt5-mt4-c-trader-match-trader>
- <https://public-api.tradelocker.com/docs/getting-started>
- [HeroFX TradeLocker activation audit](TRADELOCKER_HEROFX_ACTIVATION_AUDIT_2026-08-11.md)
- <https://www.mql5.com/en/docs/files/fileopen>
- <https://www.mql5.com/en/docs/marketinformation/symbolinfotick>
- <https://www.mql5.com/en/docs/event_handlers/ontick>
- <https://www.metatrader.com/en/about/terms>
