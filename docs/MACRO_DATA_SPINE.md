# Indices-only macro data spine

This provider boundary now has an executable regional spine: BLS, BEA, Census,
FRED, CFTC TFF, Eurostat, ECB, ONS, Bank of England, and the Bank of Japan feed
the shared raw-receipt, canonical-journal, deterministic-replay, Ledger, Macro,
and Desk snapshot path. e-Stat remains credential-gated. The slice deliberately
excludes indicators, macro scoring, nowcasting, strategy logic, and Markov/HSMM
models.

The detailed system architecture and UI consumption plan continue in
`GLOBAL_MACRO_SYSTEM_PLAN.md` and `PAGE_BLUEPRINTS_V2.md`.

## Product rule

Northstar should ingest a small, reproducible set of official observations and
derive any later bias locally. It should not reproduce a wide terminal table,
depend on scraping, label proprietary consensus as truth, or use crowd
sentiment where auditable institutional positioning exists.

```text
official releases + CFTC positioning + official policy news + GDELT
                              |
                    normalized observations
                              |
                 immutable revision-aware history
                              |
              future features and models (not this slice)
```

## Source coverage

| Coverage | Primary sources | Initial series families |
| --- | --- | --- |
| All indices | Massive | price history and seasonality inputs |
| US100 / US500 / US30 | BLS, BEA, Census, FRED | inflation, labor, GDP/PCE, retail/activity, rates/yields |
| US index futures | CFTC TFF | asset manager, leveraged money, dealer positioning and weekly changes |
| DE40 | Eurostat, ECB | GDP, HICP, labor, retail/industry, policy rate, yields and EUR conditions |
| UK100 | ONS, Bank of England | GDP, CPI, labor/wages, retail, policy rate and yields |
| JP225 | Bank of Japan implemented; e-Stat credential-gated | BOJ call rate and Tankan now; GDP, CPI, industry, labor, and retail after exact e-Stat identities are frozen |
| Policy news | Fed, ECB, BoE and BOJ feeds | releases, decisions, speeches and statistics notices |
| Broad news | GDELT | headline metadata and later category classification |

PMI and paid economist consensus are deferred. ADP is omitted initially because
the official US labor spine already contains payrolls, claims, unemployment,
wages, and JOLTS.

## Normalized observation contract

Every release keeps source truth and revision lineage. A future packed record
should contain stable integer IDs and numeric values rather than source strings
in the hot path.

```text
MacroObservation
  series_id
  region_id
  release_time_utc
  period_id
  actual
  previous
  revised_previous
  source_revision
  ingestion_time_utc
  quality_flags
```

The durable archive is append-only and revision-aware. The operating projector
publishes compact immutable latest-value snapshots from that journal. Network
buffers are parsed in source-atomic batches into dense canonical records;
source names, units, and provenance stay out of the hot paint path.

`our_expected` and `our_surprise` belong to a later derived layer, never the raw
observation. When added, they must be timestamp-causal and reproducible:

```text
DerivedExpectation
  series_id
  release_time_utc
  model_version
  expected_value
  historical_scale
  surprise_z = (actual - expected_value) / historical_scale
```

The UI must label this value `our expected`, not `consensus`.

## Positioning contract

Replace crowd sentiment with CFTC TFF positioning for the relevant equity-index
futures. Preserve the reported categories and derive nothing during ingestion:

```text
PositioningObservation
  contract_id
  report_date
  asset_manager_net
  leveraged_money_net
  dealer_net
  other_reportable_net
  non_reportable_net
  source_revision
```

Weekly delta and historical percentile are future deterministic transforms.
Execution remains in TradeLocker CFDs; futures positioning is context only.

## Provider and publication boundary

Each source adapter owns transport, rate limits, raw-response receipts, source
timestamps, and retry policy. Northstar owns canonical IDs, validation,
revision lineage, release ordering, stale-state policy, and publication.

```text
fetch -> validate -> normalize -> append receipt -> compact -> atomic publish
```

Publication must fail closed on malformed units, time-zone ambiguity, impossible
revision order, or an incomplete batch. A later macro view consumes only an
immutable snapshot; it never calls provider APIs while painting.

## Deferred capability gates

No scoring or strategy use is allowed until the data layer proves:

- point-in-time revision correctness and no future leakage;
- deterministic replay from raw receipts;
- stable unit and seasonal-adjustment semantics;
- explicit release-calendar and late/missing-release behavior;
- bounded provider latency and retry/backoff behavior;
- sufficient history per index and factor;
- measured incremental value out of sample.

Only after those gates should Northstar consider factor weights, expectation
models, headline classification, or regime models.
