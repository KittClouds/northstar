# Global macro truth system plan

## Scope

This phase builds the systems required to render an auditable global macro
office. It does not implement strategy signals, technical indicators,
proprietary consensus, Markov/HSMM models, or an opaque macro score.

The first product is trusted macro truth with release timing, revisions,
positioning, provenance, freshness, and deterministic replay.

## Authority model

```text
official statistical agencies ─┐
central banks ─────────────────┼─> macro truth plane
CFTC TFF ──────────────────────┤
official policy feeds ─────────┤
GDELT broad-news metadata ─────┘

Massive ─────────────────────────> price/reference plane
TradeLocker ─────────────────────> execution/account plane
NautilusTrader ──────────────────> trading lifecycle plane
Northstar ───────────────────────> canonical identity, publication, UI, policy
```

Macro inputs are context. They never override execution truth or bypass the
existing arming, risk, strategy, and policy vetoes.

## Region and index coverage

| Index | Macro regions | Context markets | Positioning proxy |
| --- | --- | --- | --- |
| US100 | United States | Fed path, US 2Y/10Y, USD | Nasdaq-100/equity-index TFF mapping |
| US500 | United States | Fed path, US 2Y/10Y, USD | S&P 500 TFF mapping |
| US30 | United States | Fed path, US 2Y/10Y, USD | Dow/equity-index TFF mapping |
| DE40 | Germany + euro area | ECB path, Bund yields, EUR | DAX/European index mapping where supported |
| UK100 | United Kingdom | BoE path, gilt yields, GBP | FTSE mapping where supported |
| JP225 | Japan | BOJ path, JGB yields, JPY | Nikkei mapping where supported |

Mappings are explicit, versioned records. Missing positioning coverage is shown
as unavailable; Northstar must not synthesize a proxy silently.

## System decomposition

The implementation should remain a set of package-ready crates/modules rather
than mixing provider calls into the GPUI page.

```text
northstar-macro-domain
  stable IDs, units, calendars, packed records, validation

northstar-macro-adapters
  BLS / BEA / Census / FRED / CFTC / Eurostat / ECB / ONS / BoE / e-Stat / BOJ

northstar-macro-store
  raw receipts, append log, revision index, immutable snapshots

northstar-macro-replay
  point-in-time reconstruction, release clock, deterministic fixtures

northstar-macro-publish
  compact MacroOfficeSnapshot and atomic publication

index-fund-prototype
  GPUI consumers only; no provider networking while rendering
```

Adapters may use dynamic dispatch at orchestration boundaries. Parsing,
normalization, indexing, and snapshot assembly should use monomorphic batched
kernels over dense slices.

## Canonical identities

String labels are metadata, never joins.

```text
SeriesId(u32)
ReleaseId(u64)
RegionId(u8)
ProviderId(u8)
UnitId(u16)
FrequencyId(u8)
VintageId(u64)
DocumentId(u64)
PositioningContractId(u32)
```

The catalog owns:

- provider series code;
- canonical series ID;
- region and category;
- unit and scale;
- frequency;
- seasonally adjusted status;
- observation period semantics;
- expected release timezone/calendar;
- revision policy;
- source URL template;
- active/deprecated state.

Catalog changes are versioned and require deterministic migration tests.

## Hot and cold records

### Hot observation record

A candidate fixed-width record:

```text
ObservationRecord
  series_id: u32
  flags: u32
  period_key: i64
  release_time_ns: i64
  observed_time_ns: i64
  actual: f64
  previous: f64
  revised_previous: f64
  vintage_id: u64
```

Hot records contain numeric truth and IDs only. They should be suitable for
`bytemuck`/`zerocopy` validation and read-only mmap traversal after schema
freeze.

### Cold metadata

Provider response receipts, labels, release titles, URLs, units, explanatory
notes, and document text stay in separate cold stores keyed by stable IDs.

The UI receives labels only for visible rows. It does not inflate every hot
record with repeated strings.

## Ingestion pipeline

```text
schedule batch
   -> fetch provider response
   -> hash + persist raw receipt
   -> parse into provider records
   -> validate units / time / series identity
   -> normalize into dense canonical batch
   -> reconcile revisions and vintages
   -> append observation log
   -> update compact indexes
   -> assemble immutable snapshot
   -> atomic publish
```

No partial provider batch becomes visible. Publication is all-or-nothing per
coherent release group.

### Allocation and throughput policy

- Reuse response and parse buffers.
- Batch series requests within provider limits.
- Use `memchr` for delimiter-heavy formats where appropriate.
- Use `hashbrown` for construction indexes with benchmarked internal hashers.
- Freeze stable vectors into `Box<[T]>` or `Arc<[T]>`.
- Use `memmap2` for immutable archives and snapshots.
- Keep raw JSON/XML parsing outside UI threads.
- Use bounded crossbeam channels between owned stages if asynchronous handoff
  is required.
- Introduce Rayon only for measured independent batch work.
- Add SIMD only to measured numeric transforms; provider I/O is not a SIMD
  problem.

## Release calendar

The calendar is a first-class truth system, not a UI convenience.

`ReleaseEvent` should contain:

```text
release_id
series_group
scheduled_time_utc
expected_timezone
status: scheduled | due | received | late | canceled | revised
importance: operating | context
affected_regions
affected_indices
source_document_id
```

Provider schedule changes create a new calendar version. A release that is late
must remain visibly late; it must not be marked stale using an arbitrary market
data timeout.

## Revision and vintage semantics

Northstar stores at least four distinct times:

1. observation period;
2. scheduled release time;
3. source publication time;
4. Northstar ingestion time.

Every overwrite-looking source update becomes a new vintage. The system must be
able to answer:

- What value is current now?
- What value was known at a historical decision time?
- Which prior value was revised?
- Which raw receipt proved that revision?

Backtests and receipt replays must query point-in-time truth, never the latest
revised database by accident.

## Positioning truth

CFTC TFF ingestion preserves reported categories before derivation:

```text
dealer long / short
asset manager long / short
leveraged money long / short
other reportable long / short
non-reportable long / short
open interest
report date
publication date
```

Later deterministic views may calculate net, weekly delta, and historical
percentile. The raw report and its exact contract mapping remain inspectable.

The UI calls this **Positioning**, not crowd sentiment.

## Policy and document events

Official releases, central-bank statements, speeches, and minutes are stored as
source documents with immutable receipts.

V1 exposes:

- headline/title;
- institution;
- published time;
- document type;
- affected region/index mapping;
- source URL and receipt hash;
- read/unread and pinned state.

Automatic topic classification and sentiment are deferred until they have a
separate measured quality contract. GDELT remains a broad antenna, visually
separated from official sources.

## Derived data boundary

Raw observations and derived views never share authority.

```text
RawMacroSnapshot
  source-faithful observations and positioning

DerivedMacroView
  deterministic deltas, percentiles, transforms, and later expectations
```

The first allowed deterministic transforms are:

- change from previous release;
- revision magnitude;
- distance from trailing range;
- CFTC net and weekly delta;
- CFTC historical percentile after minimum-history gates;
- time until/since release;
- rate/yield and currency change over explicit windows.

An `our expected` or `our surprise` field is a later versioned layer. It must be
timestamp-causal, reproducible, and clearly labeled. It is never presented as
economist consensus.

## Snapshot published to GPUI

```text
MacroOfficeSnapshot
  snapshot_id
  catalog_version
  as_of_time_utc
  provider_health[]
  regions[]
  index_context[]
  upcoming_releases[]
  recent_observations[]
  positioning[]
  official_documents[]
  broad_news_metadata[]
  stale_or_missing[]
```

The snapshot is immutable. GPUI selection state references stable IDs into its
dense arrays. Page painting performs no networking, parsing, database queries,
or large allocations.

## Systems-page observability

The Systems page must make the macro pipeline inspectable:

- provider connectivity and last success;
- request quota/backoff state;
- catalog coverage by region;
- next scheduled release;
- latest raw receipt hash;
- parse and validation counts;
- revision/vintage count;
- late or missing releases;
- snapshot publication sequence;
- replay determinism status;
- storage size and compaction generation;
- clock skew and timezone database version.

Healthy networking alone is never displayed as `all good`. Catalog, calendar,
revision, and publication truth have independent gates.

## Failure behavior

| Failure | Required behavior |
| --- | --- |
| Provider unavailable | retain last snapshot, mark affected series stale |
| Rate limited | honor provider backoff, show next retry |
| Unknown series code | quarantine batch; do not auto-map by label |
| Unit changed | block publication pending catalog review |
| Timezone ambiguous | quarantine release |
| Revision order impossible | preserve receipts and fail publication |
| Partial batch | no snapshot publication |
| Calendar event late | mark late, do not invent a value |
| mmap/schema mismatch | fail closed and rebuild from receipts |

## Testing and measurement gates

### Correctness

- Golden fixtures for every provider format.
- Unit, scale, seasonal-adjustment, and timezone tests.
- Original and revised release fixtures.
- Duplicate, late, missing, and out-of-order publication fixtures.
- Point-in-time replay tests proving no future leakage.
- Catalog migration and deprecated-series tests.
- CFTC contract-roll and mapping tests.
- Atomic publication crash/recovery tests.

### Performance

- Allocation counts per 10,000 observations.
- Batch parse throughput by provider format.
- Snapshot assembly wall time and peak heap.
- mmap open/validation time.
- Point-in-time replay throughput.
- GPUI snapshot swap and visible-row materialization time.

Initial desktop targets:

```text
snapshot swap                         < 2 ms p95
visible macro table materialization   < 1 ms p95
UI snapshot-to-paint                  < 16.7 ms p95
steady-state UI allocations           approximately zero per unchanged frame
```

Criterion, Iai-Callgrind where supported, and DHAT allocation gates should be
added before provider scale claims.

## Implementation sequence

1. Freeze domain IDs, units, time semantics, catalog schema, and fixture format.
2. Implement append-only observation/vintage storage and deterministic replay.
3. Build the release calendar and publication state machine.
4. Add one complete US vertical slice using fixtures before live networking.
5. Publish `MacroOfficeSnapshot` into a fixture-only Macro page.
6. Add official adapters one at a time with golden receipts.
7. Add CFTC TFF positioning and explicit contract mappings.
8. Add Europe, UK, and Japan vertical slices.
9. Add official document feeds, then GDELT as a separately labeled antenna.
10. Only after data-quality gates, consider transparent derived expectations or
    factor views as a new reviewed phase.
