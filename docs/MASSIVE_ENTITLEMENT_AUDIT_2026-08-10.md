# Massive entitlement audit — 2026-08-10

Status: **verified, fail closed**

This audit records what the supplied REST credential and the current Massive
account actually permit. It contains no credential material, response payloads,
or reusable authorization headers.

## Outcome

The credential is valid. It can use the indices reference catalog and entitled
historical aggregates. It is not entitled to `GET /v3/snapshot/indices`.

The current individual market-data terms describe the feed as display-only
unless another agreement grants broader rights. They also restrict non-display
use and creation of investment strategies or related derived works. Northstar
therefore does **not** publish a durable Massive manifest, retain these live
responses, populate the canonical journal, train from them, or use them for
execution under this account contract.

This is an entitlement result, not a transport failure.

## Live evidence

| Check | Result | Northstar consequence |
|---|---|---|
| REST Bearer credential | `OK` | credential format and account are valid |
| Massive MCP | callable | endpoint discovery and entitlement audit available |
| `/v3/reference/tickers` | `OK` | instrument discovery is available |
| `/v3/snapshot/indices` | `NOT_ENTITLED` | production snapshot supervisor stays disabled |
| `I:NDX` daily aggregates | `OK` | historical aggregate capability exists |
| `I:NDX` minute aggregates | `OK` | intraday historical bars exist for this feed |
| `I:BDE40P` aggregates | `NOT_ENTITLED` | Germany proxy candidate cannot supply bars |
| `I:BUK100P` aggregates | `NOT_ENTITLED` | UK proxy candidate cannot supply bars |
| request rate | plan limit observed | any future adapter must pace and back off |

The credential was held only in transient process memory for the direct
validation request. It was not written to the repository, Northstar data
directory, Codex configuration, a manifest, or an executable.

## Instrument evidence

### Exact benchmark evidence

| Northstar ID | Massive ticker | Provider name | Status |
|---|---|---|---|
| `US100` | `I:NDX` | `NASDAQ-100` | active exact benchmark |
| `US500` | `I:SPX` | `Standard & Poor's 500` | active exact benchmark |
| `US30` | `I:DJI` | `Dow Jones Industrial Average` | active exact benchmark |

### Global desk evidence

| Northstar ID | Discovery result | Status |
|---|---|---|
| `DE40` | `I:BDE40P`, `Cboe Germany 40` | active reference-only proxy candidate; not proven DAX-equivalent; aggregates not entitled |
| `UK100` | `I:BUK100P`, `Cboe UK 100` | active reference-only proxy candidate; not proven FTSE-100-equivalent; aggregates not entitled |
| `JP225` | no exact result for `Nikkei`, `Nikkei 225`, or `Japan 225` | no Massive binding |

Northstar must not turn a name resemblance into contract identity. A proxy can
be shown as context after methodology review, but cannot satisfy venue contract
agreement or become executable price truth.

## Code contract resulting from the audit

The provider catalog now accepts a verified subset of the six-index desk. One
provider is no longer required to own every instrument. Each binding carries:

```text
Northstar instrument ID
Massive ticker and provider name
exact benchmark or reference proxy role
human-reviewable binding basis
price scale
effective and verification times
entitlement recency
license class
```

Display-only manifests still fail before a canonical catalog, journal, replay,
or derived-bar pipeline can be constructed.

Massive aggregate responses now have a strict canonical decoder for a future
licensed contract. It validates source, HTTP status, ticker authority, result
count, duplicate timestamps, timestamp range, fixed-point scaling, OHLC
consistency, and stable event identity. The projector treats historical bars as
`STALE`, never `LIVE`, and inserts them directly into the requested M4, M20, H2,
or H4 timeline without fabricating empty intervals.

## Required production rights

Durable Massive activation requires evidence for all of the following:

1. non-display use is permitted for Northstar;
2. local raw and canonical retention is permitted;
3. deterministic replay and internal derived bars are permitted;
4. use in the user's personal trading/strategy workflow is permitted;
5. the selected plan is entitled to the required tickers and recency;
6. exact benchmark versus context-proxy mappings are reviewed;
7. termination/deletion obligations are represented operationally.

Until those are true, the honest production state is:

```text
Massive credential valid
reference discovery available
some aggregate history available
snapshot feed not entitled
canonical retention not licensed
production ingestion disabled
```

## Next data path

The next safe market-data authority is either:

- a Massive business/non-display agreement that explicitly permits the above;
- TradeLocker venue quotes and contract metadata after the verified LIVE account
  completes sanitized REST capture;
- or another licensed provider for DE40, UK100, and JP225.

Massive can still remain the exact US benchmark source if the appropriate
rights and plan entitlements are added. TradeLocker remains the eventual venue
and account truth; a Massive reference value can never replace it.

## Authoritative provider pages

- <https://massive.com/docs/rest/quickstart>
- <https://massive.com/docs/rest/indices/tickers>
- <https://massive.com/docs/rest/indices/snapshots/indices-snapshot>
- <https://massive.com/docs/rest/indices/aggregates/custom-bars>
- <https://massive.com/legal/market-data-terms-of-service>
- <https://massive.com/individuals-terms-of-service>
