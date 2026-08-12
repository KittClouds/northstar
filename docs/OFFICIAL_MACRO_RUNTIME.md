# Northstar official macro runtime

## Implemented operating cut

Northstar now has production paths from BLS, the BLS release calendar, BEA,
Census MARTS, FRED, CFTC TFF Futures Only, Eurostat, ECB, ONS, the Bank of
England, and the Bank of Japan to the same durable canonical journal and GPUI
Macro office. Credentialed sources are activated only when their environment
secret exists; the secret never enters receipts, journal events, Ledger bodies,
logs, or UI state. e-Stat is explicitly not configured until an application ID
and exact table/dimension contracts are available.

```text
BLS public API
  -> bounded source supervisor
  -> four exact L0 receipts
  -> pure borrowing decoder
  -> one committed canonical batch
  -> deterministic Macro projector
  -> immutable MacroSnapshot
  -> coalesced GPUI domain notification

CFTC public reporting API
  -> bounded daily source supervisor
  -> one exact L0 receipt
  -> pure borrowing TFF decoder
  -> typed participant observations in the shared canonical journal
  -> deterministic positioning projector
  -> immutable MacroSnapshot
  -> coalesced GPUI domain notification

Eurostat dissemination API + ECB Data Portal
  -> bounded daily source supervisor
  -> five exact JSON-stat 2.0 receipts + two ECB CSV receipts
  -> pure borrowing dimension-aware decoder
  -> one committed canonical batch in the shared journal
  -> deterministic Macro projector
  -> immutable MacroSnapshot
  -> coalesced GPUI domain notification

ONS downloads + BoE database + BOJ Time-Series API
  -> exact, frozen series and database identities
  -> source release/update metadata preserved as provenance
  -> dataset snapshots emitted as canonical SourceDocument events
  -> observations and provenance split into typed Ledger material receipts
  -> replay-safe regional facts consumed directly by Desk

BLS official iCalendar + BEA NIPA + Census MARTS + FRED
  -> bounded source-specific supervisors
  -> exact L0 receipts with sanitized request metadata
  -> frozen identity decoders
  -> shared canonical observations / release / document events
  -> deterministic replay, Ledger material, and MacroSnapshot
```

This cut implements data plumbing and presentation truth. It does not implement
macro scoring, consensus forecasts, signals, indicators, strategies, Markov or
HSMM models, Nautilus, or order transmission.

## Frozen BLS series identities

| Northstar ID | BLS series | Meaning | Adjustment | Unit |
|---:|---|---|---|---|
| 1001 | `CUUR0000SA0` | CPI / all items | NSA | index |
| 1002 | `LNS14000000` | unemployment rate | SA | percent |
| 1003 | `CES0000000001` | total nonfarm payrolls | SA | thousands |
| 1004 | `CES0500000003` | average hourly earnings | SA | dollars/hour |

Provider strings terminate at the adapter and decoder. Canonical events and
projectors join on compact `SeriesId` values.

Official API references:

- <https://www.bls.gov/developers/>
- <https://www.bls.gov/developers/api_signature_v1.htm>
- <https://www.bls.gov/developers/api_faqs.htm>

The unregistered v1 API is deliberately used for this first four-series cut.
Northstar performs four single-series requests and, after success, waits 24
hours before polling again. A failed complete refresh retries after six hours,
keeping the maximum planned request rate below the documented v1 daily limit.

The official BLS iCalendar is a separate provenance stream. It currently
materializes CPI and Employment Situation schedules plus the source document
hash. Scheduled time is preserved as event time, while `ts_effective` remains
the moment Northstar received the calendar, preventing a newly downloaded
calendar from leaking into an earlier replay.

## Frozen BEA, Census, and FRED identities

| Northstar ID | Provider identity | Meaning | Unit / measure |
|---:|---|---|---|
| 1101 | FRED `DFF` | effective federal funds rate | percent / NSA |
| 1102 | FRED `DGS2` | U.S. Treasury 2-year | percent / NSA |
| 1103 | FRED `DGS10` | U.S. Treasury 10-year | percent / NSA |
| 1201 | BEA `T10101`, line `1`, quarterly | real GDP growth | percent / QoQ SAAR |
| 1202 | BEA `T20804`, line `6`, monthly | core PCE price index | index / SA |
| 1203 | Census MARTS `SM`, `44X72`, `yes` | advance retail and food-services sales | millions of dollars / SA |

FRED is enabled with `NORTHSTAR_FRED_API_KEY`, BEA with
`NORTHSTAR_BEA_API_KEY`, and Census with `NORTHSTAR_CENSUS_API_KEY`. Missing
keys are an explicit `NotConfigured` state, not a startup failure and never a
reason to substitute fixture data. FRED real-time ranges participate in vintage
identity. BEA table/line/frequency and Census data-type/category/adjustment are
validated on every decode; unexpected identities fail closed.

Official references:

- <https://fred.stlouisfed.org/docs/api/fred/series_observations.html>
- <https://apps.bea.gov/api/_pdf/bea_web_service_api_user_guide.pdf>
- <https://www.census.gov/data/developers/data-sets/economic-indicators.html>
- <https://api.census.gov/data/timeseries/eits/marts/variables.html>

## Frozen CFTC contract identities

Northstar uses the official TFF Futures Only dataset `gpe5-46if` and freezes
contract-market-code bindings at the adapter boundary:

| Northstar instrument | CFTC contract code | Official contract |
|---|---|---|
| US100 | `20974+` | NASDAQ-100 Consolidated |
| US500 | `13874+` | S&P 500 Consolidated |
| US30 | `124603` | DJIA x $5 |
| JP225 | `240743` | Nikkei Stock Average Yen Denom |

The TFF report has no defensible DE40 or UK100 contract binding. Those two
markets remain explicitly unsupported in the UI instead of receiving a proxy
or fabricated score. Official references:

- <https://publicreporting.cftc.gov/Commitments-of-Traders/TFF-Futures-Only/gpe5-46if>
- <https://www.cftc.gov/MarketReports/CommitmentsofTraders/index.htm>

Each report row yields typed Dealer, Asset Manager, Leveraged Funds, Other
Reportables, and Non-Reportable observations. Northstar retains long, short,
spread, open interest, report date, receipt time, and source identity. It
derives objective net, week-over-week net change, and a bounded 104-week net
percentile. It does not convert positioning into a bullish/bearish score.

## Frozen Eurostat series identities

Northstar uses the keyless Eurostat dissemination API with exact frozen
dimensions and a bounded `lastTimePeriod=36` request:

| Northstar ID | Eurostat dataset | Frozen dimensions | Meaning | Unit |
|---:|---|---|---|---|
| 2001 | `prc_hicp_minr` | `freq=M`, `unit=RCH_A`, `coicop18=TOTAL`, `geo=DE` | Germany all-items HICP annual rate | percent |
| 2002 | `une_rt_m` | `freq=M`, `s_adj=TC`, `age=TOTAL`, `unit=PC_ACT`, `sex=T`, `geo=DE` | Germany total unemployment trend-cycle rate | percent |
| 2003 | `namq_10_gdp` | `freq=Q`, `unit=CLV_PCH_PRE`, `s_adj=SCA`, `na_item=B1GQ`, `geo=EA20` | euro-area real GDP QoQ | percent |
| 2004 | `sts_trtu_m` | `freq=M`, `indic_bt=VOL_SLS`, `nace_r2=G47`, `s_adj=SCA`, `unit=I21`, `geo=DE` | Germany retail volume | index |
| 2005 | `sts_inpr_m` | `freq=M`, `indic_bt=PRD`, `nace_r2=B-D`, `s_adj=SCA`, `unit=I21`, `geo=DE` | Germany industrial production | index |
| 2101 | ECB `FM.D.U2.EUR.4F.KR.DFR.LEV` | frozen eight-dimension SDMX identity | deposit facility rate | percent |
| 2102 | ECB `FM.D.U2.EUR.4F.KR.MRR_RT.LEV` | frozen eight-dimension SDMX identity | main refinancing rate | percent |

The trend-cycle unemployment measure is deliberate: Eurostat's official
metadata identifies Germany's published monthly headline as trend data because
of monthly volatility. No seasonal-adjustment proxy is substituted. The old
`prc_hicp_midx` identity is not used because Eurostat's 2026 HICP migration moved
current classifications to ECOICOP version 2 datasets.

Official references:

- <https://ec.europa.eu/eurostat/web/user-guides/data-browser/api-data-access/api-introduction>
- <https://ec.europa.eu/eurostat/web/user-guides/data-browser/api-data-access/api-getting-started/api>
- <https://ec.europa.eu/eurostat/web/hicp/information-data>
- <https://ec.europa.eu/eurostat/cache/metadata/en/une_rt_m_esms.htm>

The decoder validates the complete JSON-stat identity and frozen one-cell
dimensions, reconstructs time ordering from category indices, preserves sparse
missing observations without manufacturing zeroes, and maps estimated or
provisional statuses to a provisional canonical observation. The dataset-wide
`updated` timestamp remains in the raw receipt but is excluded from the
observation vintage hash, so unrelated dataset republication cannot fabricate a
correction.

## Frozen UK and Japan identities

| Northstar ID | Official identity | Meaning | Unit / measure |
|---:|---|---|---|
| 3001 | ONS `D7G7/MM23` | UK CPI all-items annual rate | percent / YoY |
| 3002 | ONS `IHYQ/QNA` | UK real GDP quarter-on-quarter growth | percent / QoQ |
| 3003 | ONS `MGSX/LMS` | UK unemployment rate, age 16+, seasonally adjusted | percent / SA |
| 3004 | ONS `J5EK/DRSI` | Great Britain retail volume, all retailers including fuel | index / SA |
| 3101 | BoE `IUDBEDR` | Official Bank Rate | percent / policy |
| 4101 | BOJ `FM01/STRDCLUCON` | uncollateralized overnight call rate, daily average | percent per annum |
| 4201 | BOJ `CO/TK99F1000601GCQ01000` | Tankan business conditions, large manufacturers, actual | percentage points / DI |

ONS uses its current official `/generator?format=csv&uri=...` downloads because
the former v0 time-series API is retired. Each response must match its CDID,
dataset ID, unit, frequency, and release-date schema. The release date is stored
as `publication_ns`, but `ts_effective` remains the receipt time so replay never
learns a download retroactively.

The BoE decoder freezes the two-column `DATE,IUDBEDR` export contract. The BOJ
decoder freezes the API envelope, database, series name, unit, frequency,
pagination state, last-update identity, and parallel date/value arrays. Null
BOJ values are unpublished days and never become zero observations.

Official references:

- <https://www.ons.gov.uk/economy/inflationandpriceindices/timeseries/d7g7/mm23>
- <https://www.ons.gov.uk/economy/grossdomesticproductgdp/timeseries/ihyq/qna>
- <https://www.ons.gov.uk/employmentandlabourmarket/peoplenotinwork/unemployment/timeseries/mgsx/lms>
- <https://www.ons.gov.uk/businessindustryandtrade/retailindustry/timeseries/j5ek/drsi>
- <https://www.bankofengland.co.uk/boeapps/database/Bank-Rate.asp>
- <https://www.stat-search.boj.or.jp/info/api_manual_en.pdf>
- <https://www.stat-search.boj.or.jp/ssi/mtshtml/co_q_1_en.html>
- <https://www.e-stat.go.jp/api/en/api-info>

## Time and vintage contract

BLS v1, BEA, Census, FRED current observations, Eurostat latest-only, ONS,
BoE, and BOJ
observations provide period/value data but do not by themselves prove when
Northstar first knew a historical value. Northstar therefore uses:

```text
period_start / period_end = provider observation period
ts_received                = exact response receipt time
ts_effective               = ts_received
time_quality               = ObservedLive
```

Historical observations are never backdated to a release time Northstar has not
verified. Eurostat does not expose past database versions through this endpoint,
so every received response is retained as a Northstar vintage. The BLS calendar
is retained as separate release/document evidence; it does not rewrite the
effective time of an observation downloaded later.

The decoder:

- accepts monthly `M01` through `M12` periods;
- ignores the BLS `M13` annual pseudo-period;
- treats `value: "-"` as missing instead of zero;
- rejects duplicate periods within one response;
- emits observations oldest to newest;
- derives stable source-event and vintage IDs with BLAKE3;
- preserves BLS preliminary footnotes;
- fails closed on unknown series, malformed numbers, schema drift, or time
  overflow.

When a later receipt changes the value for the same series and period, the
canonicalizer marks a correction and records the superseded vintage. The raw
receipt and prior canonical event remain immutable.

## Storage and atomic publication

The operating data root now contains:

```text
macro.raw        exact response bytes, sanitized endpoint metadata, hashes
macro.canonical  fixed-size canonical events, batch checksum, commit marker
```

A source refresh is publishable only when it contains exactly one successful
response for each frozen series in that source batch: four for BLS, one bounded
multi-market response for CFTC, five for Eurostat, three for FRED, two for BEA,
one Census MARTS response, two ECB responses, four ONS responses, one BoE
response, or two BOJ responses. Partial multi-response refreshes produce no
Macro snapshot. Raw writes are append-only evidence; one canonical commit and
one domain publication make the completed source batch visible.

Repeated provider responses remain valid raw evidence but duplicate canonical
source events are filtered before the journal. Startup opens the mmap journal,
replays every committed batch through the same projector, and schedules the next
network poll from the last received time. A current restart performs no eager
duplicate fetch.

The displayed raw-receipt count comes from the L0 store, not from canonical
event provenance. Consequently, an exact duplicate refresh increases raw
evidence without increasing the deduplicated canonical event count.

## Snapshot and UI contract

`MacroSnapshot` contains:

- domain generation, availability, health, and next refresh;
- last canonical sequence and receipt/event counts;
- all cataloged typed series snapshots, including the regional UK/Japan cut;
- four typed positioning-market snapshots plus explicit unsupported markets;
- latest and previous visible vintages;
- bounded 36-period presentation history;
- bounded 104-week participant positioning history;
- period, received/effective time, receipt, sequence, vintage, preliminary,
  correction, and time-quality provenance.

The GPUI Macro office renders current values, receipt-derived mini histories,
period and preliminary/final state, raw/canonical counts, journal sequence, last
receipt time, the explicit time contract, and the official-source contract.
Long audit panels scroll internally at the default 1440 by 780 window size.

Provider I/O, receipt persistence, JSON decoding, canonical append, and replay
occur outside the GPUI thread. Only the single state writer owns Macro and
Ledger publication, preventing duplicate authority and lost-domain updates.

## Verified evidence

The following live counts are the retained pre-BEA/Census/FRED QA baseline; they
must not be read as a claim that credentialed providers were live-tested without
their keys:

The combined isolated live QA run on 2026-08-10 produced:

```text
raw receipts         7
canonical events  2,273
last sequence     2,273
macro.raw          294,160 bytes
macro.canonical    291,296 bytes
```

A fresh process reopened the same data root with networking disabled,
reconstructed all 2,273 events, reported Macro live at sequence 2,273, and left
all four journal files byte-for-byte unchanged. The 125%-scaled maximized 1080p
Macro page was visually inspected with live BLS and Eurostat observations, real
CFTC positioning, objective percentiles, source health, provenance, and honest
unsupported states. The adaptive layout uses three cards per row at this
breakpoint so the evidence panels remain operable.

The restart proof retained these SHA-256 identities:

```text
macro.raw        38B1F518FD5995E8F5CF39B92FC10290A575BE3F7E12E3FA79BA877F4F4469AA
macro.canonical  20379C06951A13C2409972A4B1732665F97C5EC216A8713C92017E6A67E6D574
ledger.hot       787F4981D756427D834DB4483EB51D644F7987D7E255F242B3E426A08249FD61
ledger.body      7E2BD247DDA97E00EDC26426F27D0ADCCD713FCBDBDDA685679A15A32152D2C6
```

The completed regional adapter cut passed decoder, atomicity,
duplicate-suppression, durable replay, document-provenance, and Desk publication
tests. Verification on the isolated target
`D:\phoenix-target-northstar-regional-macro` passed:

- 96 no-default-feature library tests;
- 127 desktop/all-feature unit tests and three integration smoke tests;
- strict all-target/all-feature Clippy with warnings denied;
- formatting check.

Four manual release-mode performance gates remain intentionally ignored in the
ordinary debug suite. Live BEA, Census, FRED, and e-Stat transport activation
still requires their credentials.

Each committed canonical source batch now also reduces to one idempotent Ledger
machine receipt linked to its exact sequence range. Startup reconciles missing
receipts from canonical history without duplicating observations into Ledger.

## Next provider cuts

1. Obtain an e-Stat application ID, freeze exact table and dimension identities,
   and add the missing Japan CPI/GDP/labor/retail observations.
2. Extend release-calendar and official-document coverage to policy decisions
   and statistical releases without weakening receipt-time causality.
3. Add no scores, strategies, Markov models, Nautilus, or live orders in this
   data-plane phase.

HeroFX MT5 and TradeLocker now take priority over Massive for actual CFD signal
and execution truth. Their account capture remains deferred until the accounts
exist; `HERO_FX_VENUE_DATA_PLAN.md` owns that boundary. Massive is an optional
licensed independent reference rather than the primary activation path.
Nautilus and all model/strategy work remain later phases.

## Feed-truth cleanup — 2026-08-12

Missing observations now preserve their actual feed state instead of collapsing
to `awaiting receipt`:

```text
AWAITING         no transport receipt yet
CURRENT          canonical publication exists
NOT CONFIGURED   required credential is absent
REJECTED         transport or canonical ingestion failed
UNSUPPORTED      no canonical series exists in the frozen catalog
```

Each supervisor now waits for state-writer acknowledgement of canonical
ingestion. A successful HTTP response followed by schema or publication failure
uses the bounded failure-retry cadence rather than the normal daily interval.

The ONS decoder accepts the provider's official empty metadata rows such as
`"Important notes",` while retaining strict validation for identity, unit,
release date, period, duplicates, and numeric observations. An isolated clean
run published all four ONS series. The resulting journal contained 6,636 events
and 14 of 24 catalogued macro series:

```text
ONS CPI           450 observations
ONS GDP           284 observations
ONS unemployment  663 observations
ONS retail        366 observations
```

The same visual proof showed the BLS observation feed as rejected after the
anonymous API quota response, the BLS release calendar as rejected after HTTP
403, FRED/BEA/Census as not configured, and uncatalogued Japanese inflation and
labor as unsupported. No credential was added during this pass.
