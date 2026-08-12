# Massive real-data runtime

> Live entitlement result: the supplied individual key is valid, but the
> snapshot endpoint is not entitled and the current market-data agreement is
> display-only unless broader rights are separately granted. See
> [MASSIVE_ENTITLEMENT_AUDIT_2026-08-10.md](MASSIVE_ENTITLEMENT_AUDIT_2026-08-10.md).

## Status

Northstar now has an operational, credential-safe Massive REST bootstrap from
provider bytes to the GPUI Desk snapshot boundary. The code path is active only
after a complete reviewed catalog and a REST API credential are available.

| Capability | Status |
|---|---|
| Verified-subset catalog schema and validation | Implemented |
| License and durable-retention gate | Implemented |
| Environment-only REST API credential | Implemented |
| Redacted request metadata | Implemented |
| Exact-ticker REST snapshot transport | Implemented |
| Six L0 receipts to one L1 batch | Implemented |
| Atomic entitlement/completeness gate | Implemented |
| Delayed versus real-time truth | Implemented |
| Duplicate L0 and L1 identity rejection | Implemented |
| Restart replay into Desk state | Implemented |
| Bounded 60-second bootstrap supervisor | Implemented |
| Massive MCP catalog discovery | Live and audited |
| Production manifest | Blocked by current display-only rights |
| Historical aggregate canonical decoder | Implemented; activation license-gated |
| WebSocket value stream | After exact entitlement and endpoint verification |
| TradeLocker venue quote | LIVE desktop verified; sanitized REST capture pending |

The current implementation does not use guessed index symbols. `US100`,
`US500`, `US30`, `DE40`, `UK100`, and `JP225` are permanent Northstar IDs. A
Massive manifest may own a reviewed subset; absent instruments remain under no
Massive authority.

## Runtime contract

```text
N verified exact-ticker HTTPS responses
        |
        v
N credential-free L0 raw receipts
        |
        v
complete-set decoder
  - exactly the reviewed subset of identities
  - no duplicates or unknowns
  - no NOT_FOUND / NOT_ENTITLED
  - recency equals reviewed entitlement
  - active Northstar authority for every index
        |
        v
one durable N-event L1 batch
        |
        v
one MarketProjector application
        |
        v
one immutable Desk snapshot / one GPUI notice
```

The transport deliberately issues one documented exact `ticker` request per
index. It does not depend on an assumed comma-list or `any_of` behavior. All N
responses are accumulated off the GPUI and state-writer threads. Network
failure before handoff publishes no partial refresh.

Every complete HTTP response is retained before decoding. If one response is
not entitled, malformed, or incomplete, that refresh's canonical values do not
publish. If a later poll repeats the same provider timestamp and value, the raw
evidence remains retained while the canonical duplicate is rejected and no
Desk redraw occurs.

## Catalog activation

The production file is:

```text
%LOCALAPPDATA%\Northstar\ledger-v1\massive.catalog.json
```

The V1 manifest contains:

```text
schema and catalog version
Massive source identity
capture timestamp
reviewed license class

for each Northstar instrument Massive is reviewed to own:
    permanent Northstar instrument ID and symbol
    display name
    verified Massive ticker and provider name
    exact-benchmark or context-proxy role
    human-reviewable binding basis
    fixed-point price scale
    identity effective time
    verification time
    DELAYED or REAL-TIME entitlement
```

Activation fails closed when the set is empty, a ticker or instrument is
duplicated, a ticker lacks the `I:` namespace, verification time is invalid, or
the reviewed license does not permit canonical retention.

The application starts without Massive when the file is absent. It reports
`Not configured`; it does not fall back to fixtures or ETF/CFD proxies.

## Credential boundary

Northstar reads the Massive REST API key only from:

```text
MASSIVE_API_KEY
```

It authenticates with the documented `Authorization: Bearer ...` header. The
key type cannot be serialized and its `Debug` representation is always
`<redacted>`. Receipt metadata records only method, path, ticker, and a redacted
credential marker. Query URLs never contain `apiKey`.

The S3 `Access Key ID` and `Secret Access Key` shown on Massive's Flat Files tab
are a separate credential pair. They are not the REST API key and Northstar's
REST runtime does not accept or store them.

Any credential pasted into chat or captured in a screenshot should be rotated
before production use.

## Time and availability

REST `last_updated` is treated as provider event time in Unix nanoseconds.
Northstar receipt time remains distinct.

- `REAL-TIME` becomes `TimeQuality::ObservedLive`.
- `DELAYED` becomes `TimeQuality::ProviderDelayed`.
- Both become observable to replay only at `ts_received`; delayed data is never
  backdated into a time when Northstar had not received it.
- The Desk renders provider-delayed values as `Delayed`, never `Live`.
- A changed entitlement recency invalidates the refresh until the catalog is
  reviewed and versioned again.

The REST bootstrap polls no more than once per 60 seconds. This is not the final
live stream. A verified WebSocket adapter will later replace high-frequency REST
polling while using the same decoder/journal/projector boundary.

## Storage and replay

Configured runtime files are:

```text
massive.raw          exact provider responses and sanitized metadata
massive.canonical    fixed 128-byte Northstar index-value events
```

Both are append-only and checksummed. Raw receipt IDs and canonical source
event IDs are duplicate-checked before writes. On restart the mapped canonical
journal replays through the same `MarketProjector` used live; the recovered
Desk snapshot is published before the next network poll.

Historical bars are intentionally not fabricated from snapshots. Until the
historical aggregate endpoint and deterministic per-index session calendars are
implemented, reference values can be real while charts honestly await history.

## Failure behavior

| Condition | Desk state | Publication |
|---|---|---|
| catalog or REST key absent | Not configured | one startup state |
| retention not licensed | Not entitled | one startup state |
| one ticker not entitled | Not entitled | no partial values |
| 429 response | Stale | last durable values retained |
| network disconnect | Disconnected | last durable values retained |
| malformed/schema/identity drift | Invalid | last durable values retained |
| unchanged canonical snapshot | unchanged | no redraw |
| successful complete refresh | Live or Delayed | exactly one Desk generation |

## Verification evidence

The current slice is covered by:

- catalog completeness, duplicate, ticker, time, and retention tests;
- secret-redaction and missing-credential tests;
- documented WebSocket and REST snapshot decoder tests;
- entitlement, missing-record, and recency-drift rejection tests;
- raw receipt duplicate rejection before write and after restart;
- an integration test proving six receipts commit as one six-event batch and
  one delayed Desk generation;
- duplicate refresh proof: twelve raw receipts, one canonical batch, no second
  Desk generation;
- full `cargo test --all-targets` and strict all-feature clippy.

Official contracts used:

- <https://massive.com/docs/rest/quickstart>
- <https://massive.com/docs/rest/indices/overview>
- <https://massive.com/docs/rest/indices/snapshots/indices-snapshot>
- <https://massive.com/docs/rest/indices/tickers/all-tickers>
- <https://massive.com/docs/websocket/indices/value>

## Next slice

1. Obtain explicit non-display, strategy, derived-work, and retention rights.
2. Upgrade or authorize the required snapshot/stream and global-index feeds.
3. Select no unproven proxy as an executable or contract-agreement authority.
4. Write the first production catalog manifest only from licensed results.
5. Capture sanitized real REST fixtures and run an isolated end-to-end Desk QA.
6. Attach the implemented aggregate decoder to paced receipts and versioned
   session calendars so the native canvas receives licensed real bars.
7. Add the entitled delayed or real-time WebSocket value stream with reconnect,
   sequence, stale, and backfill behavior.

TradeLocker, Nautilus, indicators, models, and order transmission remain outside
this slice.
