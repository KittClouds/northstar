# HeroFX TradeLocker activation audit — 2026-08-11

## Result

The TradeLocker dependency is no longer hypothetical. A read-only audit of the
already authenticated desktop client verified one HeroFX LIVE account, a live
indices market surface, and the venue-side tables Northstar needs for quotes,
account truth, reconciliation, and the automatic Ledger.

This audit did not submit, modify, cancel, or close an order. It did not enter,
copy, persist, or log a password, JWT, refresh token, or API key. Account
identifiers are intentionally redacted from this document. Northstar, Phoenix,
TradeLocker, MetaTrader, and MetaEditor were left running.

The activation state is now:

```text
desktop account and visible venue schema     VERIFIED
official REST capability contract            VERIFIED
Northstar ownership/boundary audit            VERIFIED
licensed durable-retention rights             OPEN
authenticated REST payload capture            OPEN
typed TradeLocker decoders                     OPEN
coherent reconciliation publication            OPEN
order transmission                             DEFERRED
MT5 bridge                                     DEFERRED
```

## Live desktop evidence

The target was the installed TradeLocker desktop client at version `3.19.35`.
The embedded application reported build `3.94.8`, network state `Stable`, broker
`Hero FX`, environment `LIVE`, and one real account. The broker account route
displayed `HEROFX-ZERO-COMMISSION-REAL-B`; the account identifier is runtime
secret material and is not recorded here.

The audited account was empty at the time of observation: no open positions,
no pending orders, no visible history in the selected period, and zero account
metrics. This is enough to verify navigation and table contracts, but it is not
evidence for partial fills, multi-order positions, amendments, swaps, fees, or
close workflows.

## HeroFX index catalog observed in the client

The desktop market panel exposed the following index symbols and descriptions:

| Symbol | Description |
|---|---|
| `EUSTX50` | Euro Stoxx 50 Index |
| `HK50` | Hong Kong Index |
| `RUS2000` | Russel 2000 Index |
| `AUS200` | Australia 200 Index |
| `US30` | US 30 Index |
| `NAS100` | US Tech 100 Index |
| `F40` | France 40 Index |
| `JP225` | Japan 225 Index |
| `SPX500` | US 500 Index |
| `UK100` | UK 100 Index |
| `DE40` | German 40 Index |
| `CH20` | Switzerland 20 index |
| `NL25` | Netherlands 25 Index |
| `ES35` | Spain 35 Index |

This establishes candidate symbol names only. It does not freeze
`tradableInstrumentId`, INFO/TRADE routes, tick size, lot rules, precision,
session, margin, or effective contract versions. Those values must come from
account-scoped REST discovery and a versioned `VenueInstrumentContract`.

The market table visibly supplies:

```text
Instrument
Bid
Ask
Spread
Leverage
Day High
Day Low
Description
```

All visible index rows showed leverage `100` during the audit. That observation
is not promoted to an immutable contract because leverage and margin terms are
account/broker configuration and may change. The selected `US30` ticket showed
`0.01` lots and an estimated initial margin, but the ticket is presentation
evidence, not the authoritative lot-step or margin schema.

The chart exposed broker-side OHLC and volume on a three-minute interval. The
public history API does not document `3m`, so Northstar must not assume every UI
interval is retrievable from REST.

## Venue tables observed for the Ledger

### Closed positions

```text
Instrument
Entry Time (EET)
Type
Side
Amount
Entry Price
SL Price
TP Price
Exit Time (EET)
Exit Price
Fee
Swap
P&L
Net P&L
Order ID
Position ID
Actions
```

The surface also provides a selectable time window and summary totals for P&L,
amount, fee, and swap.

### Balance history

```text
Date
Type
Instrument
Change
Resulting Balance
Position ID
```

### Fills/trades

```text
Time (EET)
Side
Amount
Instrument
Price
P&L
Fee
Order ID
Position ID
```

### Order history

```text
Time
Instrument
Status
Order Type
Action
Side
Amount (Lots)
Target Price
Bid
Ask
Filled Price
Order ID
Position ID
```

These are UI projections, not decoder schemas. They do verify that Northstar's
Ledger must retain order and position identities as separate columns and must
model balance changes, fees, swaps, fills, and final order outcomes as distinct
evidence. One closed-position row may summarize several venue facts; Northstar
must preserve the underlying receipts rather than ingesting the visual row as a
single source of truth.

## Official REST contract verified

The official TradeLocker documentation currently establishes:

- LIVE and demo REST environments use separate base URLs;
- JWT authentication returns access and refresh tokens;
- `accountId` identifies the account while the `accNum` header selects an
  account for every `/trade/*` request;
- `/trade/config` is the runtime schema, row-limit, and route-rate-limit
  contract;
- instrument discovery returns account-scoped instruments and distinct `INFO`
  and `TRADE` routes;
- quotes, daily bar, and history use the `INFO` route;
- instrument details expose lot steps/sizes, quoting currency, and trade
  session identity;
- account state, positions, non-final orders, final order history, and account
  details are separate request/response resources;
- history requests are limited to 20,000 bars and document `1m`, `5m`, `15m`,
  `30m`, `1H`, `4H`, `1D`, `1W`, and `1M` resolutions;
- daily bars distinguish `ASK`, `BID`, and `TRADE` sources;
- final order history is the documented bridge from `orderId` to the distinct
  `positionId` created after a fill;
- `strategyId` is limited to 31 characters and is visible across orders,
  order history, and positions.

The public contract is REST request/response. It does not document the old
prototype's `SyncEnd` stream assumption. Northstar's production gate must be
named after the truth it owns:

```text
TradeLocker coherent epoch committed
```

not:

```text
TradeLocker SyncEnd received
```

## Required account-scoped capture order

The first authenticated Northstar capture must be read-only and run in this
order:

1. resolve the LIVE environment and selected account through a protected secret
   handle;
2. capture `/trade/config` before decoding any dynamic table;
3. capture selected account details and the full instrument catalog;
4. capture details plus INFO/TRADE routes and session state for `NAS100`,
   `SPX500`, `US30`, `DE40`, `UK100`, and `JP225`;
5. capture quote, daily-bar, and bounded history examples for each binding;
6. capture account state, positions, non-final orders, final order history, and
   fill/trade history where the account exposes it;
7. capture representative 401, 403, 404, no-data, rate-limit, stale-session,
   and malformed-response failures without provoking account mutations;
8. strip authorization headers and secret material before any durable L0
   receipt is admitted;
9. freeze golden fixtures only after source rights permit retention and internal
   analysis.

No response-body shape is guessed before this capture. `/trade/config` columns
must be mapped by name to typed decoders; numeric array offsets are never
hard-coded without the matching configuration receipt.

## Coherent reconciliation epoch

One publishable epoch is a bounded set of receipts, not a series of independent
UI updates:

```text
config version
account details/state
instrument contracts and session status
positions
non-final orders
final order history needed to link open positions
fill/trade history watermark
request start/end and receipt sequence range
```

Northstar publishes the epoch only if every required call succeeds under one
account selection and the total age is within policy. Partial success leaves the
last complete epoch visible as stale and emits an incident. It never combines
new account state with old positions or silently treats an empty failed response
as a flat account.

## Audit of Northstar's current seam

What is already correct and must be reused:

- `VenueInstrumentContract` already separates environment, account,
  instrument, INFO route, TRADE route, session, quantity terms, price tick,
  precision, version, and effective interval;
- `CanonicalEvent::venue_quote` and the Market projector already preserve
  bid/ask/midpoint as venue truth and publish it independently of reference
  values;
- `VenueOrderId` and `VenuePositionId` are distinct typed namespaces;
- the Ledger hot record already carries account/instrument identity, stable
  source keys, correlation, canonical sequence ranges, and a cold evidence body;
- GPUI already accepts immutable segmented snapshots and explicit unavailable
  states.

What must be added or corrected:

- a read-only TradeLocker source adapter and protected token lifecycle;
- a config-driven table decoder rather than fixed response indexes;
- typed account, contract, session, quote, history, order, fill, position, and
  balance-change records;
- a rate-budget scheduler derived from `/trade/config`;
- a reconciliation builder that stages all resources before atomic publish;
- explicit unknown venue order/position detection;
- Ledger reducers for order lifecycle, fills, positions, fees, swaps, realized
  P&L, balance changes, and reconciliation incidents;
- replacement of every `SyncEnd` label with coherent-epoch language;
- account snapshots and venue health published to Fund, Systems, Desk, and
  Ledger without rebuilding unrelated domains.

The current `ExecutionReconciler` is a useful identity test seam, not yet the
production snapshot model. It assumes an opening order is already present for
each position and has no account, instrument, price, side, timestamps, fees,
swap, realized P&L, route, or receipt-range fields. Production reconciliation
must stage final order history alongside positions before validating links.

## Exit test for the activation leg

> With order transmission disabled, Northstar authenticates through a protected
> runtime secret handle, captures the HeroFX config and six index contracts,
> publishes one coherent account epoch, journals every admitted receipt without
> secrets, and reproduces the identical Desk, Fund, Systems, and Ledger state
> after replay. A missing, stale, reordered, duplicated, rate-limited, or
> conflicting resource fails closed.

The account is available; authenticated payload capture is now the immediate
frontier. The MT5 bridge, Nautilus, strategies, indicators, Markov/HSMM models,
arming, and order transmission remain deferred.

## Official references

- <https://public-api.tradelocker.com/docs/getting-started>
- <https://public-api.tradelocker.com/reference/getconfigusingget>
- <https://public-api.tradelocker.com/reference/getaccounts>
- <https://public-api.tradelocker.com/reference/getinstruments>
- <https://public-api.tradelocker.com/reference/getinstrumentdetails>
- <https://public-api.tradelocker.com/reference/getsessionstatususingget>
- <https://public-api.tradelocker.com/reference/getquotes>
- <https://public-api.tradelocker.com/reference/getdailybar>
- <https://public-api.tradelocker.com/reference/gethistory>
- <https://public-api.tradelocker.com/reference/getstate>
- <https://public-api.tradelocker.com/reference/getpositions>
- <https://public-api.tradelocker.com/reference/getorders>
- <https://public-api.tradelocker.com/reference/getordershistory>
- <https://public-api.tradelocker.com/docs/difference-between-orderid-and-positionid>
