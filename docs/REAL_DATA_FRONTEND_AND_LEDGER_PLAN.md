# Northstar real-data frontend and trade-ledger plan

## Status and objective

This document is the cutover plan from the deterministic GPUI prototype to an
operating, indices-only Northstar desk. It covers the path from external data to
visible GPUI state and defines Ledger as a durable machine-and-operator trade
journal.

The objective is not to make the fixtures move. The objective is:

> Every visible value comes from a named authority, carries time and freshness,
> can be traced to durable evidence, survives restart where appropriate, and
> has an honest unavailable state.

This phase may make the Desk, Macro, Fund, Systems, and Ledger pages fully useful
for live observation, reconciliation, journaling, review, and replay. It does
not authorize order transmission. Nautilus, strategy execution, indicators,
Markov/HSMM models, and live arming remain deferred.

## Audit of the current seam

### What exists now and must be reused

- The native GPUI shell and chart canvas already render the five-page office.
- The chart owns viewport, pan, zoom, crosshair, visible-range planning, and
  layered invalidation.
- L0 raw receipts preserve provider payloads with hashes and commit framing.
- L1 stores fixed 128-byte causal events in an append-only mmap journal.
- Canonical events distinguish event, receive, and effective time.
- Live and replay already publish the same `EventBatchRef` interface.
- The M4/M20/H2/H4 bar builder is deterministic and revision-aware.
- Catalog, calendar, authority, venue-contract, export, and licensing boundaries
  already exist.
- The current pages establish a useful visual hierarchy and interaction model.

### Deliberately retained reference shell

The fixture-only `northstar-index-fund` binary remains a frozen visual reference.
The separate `northstar-operating` binary owns the production composition root,
domain snapshots, supervisors, durable replay, and writable Ledger. Fixtures do
not enter that path. TradeLocker account truth, attachment storage, search
indexes, review analytics, and live execution remain explicit later gates.

## Current external-contract findings

Provider behavior is discovered and captured, never inferred from the mockup.
### Massive

Massive is a reference-index authority, never venue bid/ask authority. The MCP
and supplied REST credential were audited live. The key is valid, historical
`I:NDX` aggregates are entitled, and snapshots are not. The current individual
terms are display-only absent a broader agreement, so no production manifest,
receipt retention, canonical replay, strategy use, or derived-bar activation is
claimed. Catalogs now support verified subsets and distinguish exact benchmarks
from context proxies. Full evidence lives in
`MASSIVE_ENTITLEMENT_AUDIT_2026-08-10.md` and `MASSIVE_REAL_DATA_RUNTIME.md`.
### TradeLocker

TradeLocker is venue and account authority. Its public contract is REST
request/response. Authentication is JWT based. `/trade/*` calls require the
account selector, instrument routes distinguish `INFO` and `TRADE`, and
`/trade/config` supplies dynamic response columns, row limits, and route rate
limits. Quotes, history, account state, positions, active orders, and order
history are available, but the public documentation does not establish a
streaming `SyncEnd` event.

Northstar must therefore produce coherent reconciliation epochs from bounded,
rate-aware REST snapshots. Order ID and position ID stay distinct throughout
the system.

The HeroFX LIVE TradeLocker account and desktop venue schemas were verified
read-only on 2026-08-11. The account dependency is therefore resolved.
Authenticated REST payload capture, `/trade/config`-driven decoders, exact
instrument bindings, protected token handling, and reconciliation proof remain
open. No guessed response body is promoted to a production contract. See
`TRADELOCKER_HEROFX_ACTIVATION_AUDIT_2026-08-11.md`.

Official references:

- <https://public-api.tradelocker.com/docs/getting-started>
- <https://public-api.tradelocker.com/reference/getquotes>
- <https://public-api.tradelocker.com/reference/gethistory>
- <https://public-api.tradelocker.com/reference/getstate>
- <https://public-api.tradelocker.com/reference/getpositions>
- <https://public-api.tradelocker.com/reference/getorders>
- <https://public-api.tradelocker.com/reference/getordershistory>
- <https://public-api.tradelocker.com/reference/getconfigusingget>
- [HeroFX TradeLocker activation audit](TRADELOCKER_HEROFX_ACTIVATION_AUDIT_2026-08-11.md)

### Official macro and positioning sources

The source list, identities, vintages, publication rules, and regional rollout
remain owned by `GLOBAL_MACRO_SYSTEM_PLAN.md` and `MACRO_DATA_SPINE.md`. This
plan does not create a competing macro architecture. It connects their
canonical observations, releases, positioning reports, and source documents to
the GPUI snapshot plane.

## Authority table

| Capability | Owner | Source of truth | Frontend consequence |
|---|---|---|---|
| Reference index value | Optional licensed reference adapter | canonical `IndexValue` | Independent benchmark context |
| Signal-market quote | HeroFX MT5 sensor | canonical source-preserving quote | Signal chart and broker-CFD history |
| Execution quote | HeroFX TradeLocker adapter | canonical `VenueQuote` | Executable bid, ask, midpoint, spread |
| Instrument terms | HeroFX TradeLocker | versioned venue contract | Precision, tick, size, route, session |
| Account and positions | HeroFX TradeLocker | reconciliation epoch | Fund and exposure state |
| Macro observations | Northstar | official-source canonical events | Macro tables and release detail |
| Positioning | Northstar | canonical CFTC observations | Positioning panels |
| Raw and canonical history | Northstar | L0/L1 journals | Replay and evidence links |
| Derived bars and sessions | Northstar | versioned L2 projectors | Chart and session state |
| UI snapshots | Northstar | immutable domain snapshots | Only input accepted by GPUI pages |
| Trade narrative | Northstar Ledger | append-only ledger journal | Trade Book, notes, reviews, reports |
| Portfolio/execution semantics | Nautilus | deferred integration | No live transmission in this phase |
| Venue transmission | TradeLocker | deferred live phase | Controls remain disabled/read-only |

Formal invariant:

> No two modules may own the same truth. A derived view names its upstream
> authority and derivation version; it never silently becomes a second source.

## End-to-end operating architecture

```text
HeroFX MT5 pipe    HeroFX TradeLocker REST    official macro/CFTC
       |                     |                         |
       +---------- source adapters and L0 receipts ----+
                             |
                     pure canonical decoders
                             |
                  committed L1 canonical journal
                             |
                one live/replay batch publication API
                             |
       +---------------------+-------------------------+
       |                     |                         |
 market projector      venue/fund projector      macro projector
       |                     |                         |
       +---------- system and ledger projectors -------+
                             |
                 immutable domain snapshot store
                             |
                    bounded change notification
                             |
                 GPUI view model and native canvas
```

Optional licensed Massive data enters as independent reference observations;
it does not own the HeroFX signal or execution market. See
`HERO_FX_VENUE_DATA_PLAN.md` for the frozen three-price-role contract.

Provider I/O, decoding, storage, projection, and painting are separate stages.
The GPUI render path performs no network calls, disk reads, journal scans,
reconciliation, or authoritative calculations.

## Presentation truth contract

Every operational value needs more than a number. Coarse presentation fields
use a contract equivalent to:

```rust
enum DataState {
    Live,
    Delayed,
    Stale,
    Replaying,
    AwaitingFirstReceipt,
    NotConfigured,
    NotEntitled,
    Disconnected,
    Invalid,
}

struct ValueMeta {
    state: DataState,
    source: SourceId,
    as_of_ns: i64,
    received_ns: i64,
    journal_sequence: JournalSequence,
    derivation_version: u32,
}
```

Hot series do not wrap every sample in a heap-heavy object. They retain dense
numeric lanes, validity bitsets, and shared series metadata. `ValueMeta` is used
at snapshot and inspector boundaries where lineage is visible and useful.

UI rules:

- Never show zero for unknown.
- Never keep a stale value green.
- Never replace a failed real source with fixture data.
- Preserve the last valid value only when it is visibly marked stale with age.
- Give each page an `as of`, mode, and publication-sequence header.
- Let the operator open the supporting receipt or reconciliation epoch.
- Use `Not configured` and `Awaiting first receipt` as finished product states,
  not developer placeholders.

## Production runtime and snapshot plane

### Composition root

Introduce `NorthstarRuntime` as the production composition root. It owns:

- validated configuration and secret handles;
- catalog/calendar versions;
- receipt and canonical journal writers/readers;
- live/replay publisher selection;
- provider task supervision;
- L2 projectors;
- immutable snapshot store;
- Ledger command and query ports;
- health and incident publication.

`PrototypeRuntime` moves behind an explicit `fixtures` feature and remains
available only to visual tests, examples, and deterministic screenshots. The
default desktop binary opens `NorthstarRuntime`. If configuration is absent it
opens a real setup state, not a demo account.

### Segmented snapshots

Do not rebuild or clone one office-wide object for every quote. Publish domains
independently:

```rust
struct OfficeSnapshot {
    desk: Arc<DeskSnapshot>,
    macro_office: Arc<MacroSnapshot>,
    fund: Arc<FundSnapshot>,
    systems: Arc<SystemsSnapshot>,
    ledger: Arc<LedgerSnapshot>,
    generation: OfficeGeneration,
}
```

Each projector advances only its domain generation. Within Desk, each index can
own an `Arc<IndexMarketSnapshot>` so a US100 update does not copy DE40 history.
Cold documents and evidence remain behind typed IDs and are loaded on demand.

### Subscription boundary

Replace the concrete runtime dependency with narrow ports:

```rust
trait OfficeSnapshotPort: Send + Sync {
    fn current(&self) -> Arc<OfficeSnapshot>;
    fn subscribe(&self, sink: SnapshotSink) -> Subscription;
}

trait LedgerCommandPort: Send + Sync {
    fn append_operator_entry(&self, command: OperatorEntryCommand)
        -> Result<LedgerEntryId, LedgerError>;
}
```

The publisher sends a compact changed-domain bitset and generation. GPUI swaps
the affected `Arc`, repairs selection by stable ID, marks the required damage
layer, and calls `cx.notify()` once. A quiet system causes no redraws.

Suggested coalescing policy:

- reference/venue value labels: latest value per UI frame;
- active candle geometry: at most one chart-series update per frame;
- fund/account state: publish per coherent reconciliation epoch;
- systems counters: 2-4 Hz maximum unless state changes severity;
- crosshair: local overlay invalidation only;
- ledger rows: append batches, not one notify per receipt.

## Page-by-page placeholder eradication

### Desk

| Visible field | Real producer | Empty/failure behavior |
|---|---|---|
| Index identity | versioned catalog binding | block symbol, show catalog error |
| Reference value/change | latest Massive `IndexValue` plus prior close | stale age or not entitled |
| Native chart | canonical bar projector with REST backfill | honest gaps, loading range |
| Bid/ask/mid/spread | TradeLocker venue quote | `Venue unavailable`; no fabricated spread |
| Reference/venue basis | versioned derivation over synchronized observations | hide when timestamps exceed tolerance |
| Session state | canonical calendar plus venue session contract | disagreement incident and blocked gate |
| Positions/exposure | latest complete TradeLocker reconciliation epoch | retain stale epoch with warning |
| Macro next event | Macro release projector | `No scheduled release in window` |
| Decision/system rows | later Strategy runtime | `No strategy runtime configured` |

Chart backfill, live values, and venue quotes retain separate identities. A
Massive index value must never be labeled executable. TradeLocker history may
be displayed as venue context but does not replace the reference series without
an explicit operator-selected chart mode.

### Macro

- Region/family values come from canonical observations selected as-of the
  macro snapshot's publication time.
- Revisions show original, revised, publication, received, and superseded
  vintage IDs.
- Release rows show actual, previous, Northstar expectation only when that
  expectation has a named reproducible method. It is never called consensus.
- CFTC positioning replaces crowd sentiment.
- Source documents link to stored receipt metadata and official origin.
- Deterministic category context may be displayed only with a visible formula
  version and component inspector. Until approved, show observations rather
  than a fabricated bullish/bearish score.
- Late, missing, stale, and estimated-publication observations remain visible
  data-quality states.

### Fund

| Visible field | Authority/derivation |
|---|---|
| Balance, equity, available funds, margin, floating P&L | TradeLocker account-state epoch |
| Open positions and gross exposure | TradeLocker positions plus contract conversion |
| Realized P&L, fees, financing | final orders/fills and account history when supported |
| Equity curve | durable sequence of reconciled account snapshots |
| Daily loss capacity | versioned Northstar policy applied to venue equity |
| Drawdown | derived from reconciled equity history |
| Stress rows | versioned deterministic scenarios; otherwise absent |
| Strategy attribution | absent until stable strategy IDs exist |

Every total exposes its account, currency, reconciliation epoch, and as-of time.
Unknown fees or financing remain unknown and do not get coerced to zero.

### Systems

Systems becomes live observability rather than a decorative health board:

- adapter connection/auth state and next retry;
- raw receipt, canonical batch, and journal sequence rates;
- feed age, late-event count, gaps, duplicates, and decoder failures;
- catalog, calendar, contract, and derivation versions;
- disk capacity, journal integrity, recovery, and replay hash;
- clock offset and sampling age;
- TradeLocker REST budget and last coherent reconciliation epoch;
- unknown orders/positions and instrument-contract disagreements;
- snapshot publication age and UI subscriber lag;
- durable incidents with acknowledgement/resolution receipts.

Nautilus displays `Deferred / not installed`, never a green reconciled fixture.
Strategy Runtime remains empty until that later system exists.

### Ledger

Ledger is specified separately below because it owns much more than a table.

## Ledger: Northstar's institutional memory

### Product contract

Ledger serves two related views of one history:

1. **Machine truth** records what Northstar and the venue observed or did.
2. **Operator memory** records plans, context, notes, review, and lessons.

Machine receipts are immutable. Operator content is amendable only by appending
a new event that supersedes or redacts a previous operator event. No screen
silently edits history.

### Trade case: the primary journal object

A `TradeCase` is the durable story of one idea. It can begin before an order and
can remain manual-only. Later it may link to many intents, orders, fills,
positions, and exits.

```text
TradeCase
  case_id
  account_id
  instrument_id
  direction
  lifecycle_state
  opened_at / closed_at
  plan_entry_id?
  strategy_id/revision?
  session and macro context refs
  order_ids[]
  position_ids[]
  operator tags[]
  latest review id?
```

This prevents the common mistake of treating one broker order as one trade. A
single case may scale in, partially exit, replace stops, or span several venue
orders while retaining one thesis and review.

### Append-only ledger records

Use a compact hot header and cold typed payload:

```text
LedgerEventHeader
  entry_id
  case_id?
  parent_entry_id?
  correlation_id
  actor_id
  event_time
  received_time
  recorded_time
  kind
  status
  instrument/account IDs
  canonical sequence range?
  payload offset/length/hash
  schema version
```

Hot headers live in an append-only, mmap-readable Ledger journal. Rich text,
checklists, serialized typed bodies, and report material live in a checksummed
cold pack. Attachments are content-addressed by BLAKE3 and referenced by ID.
Derived indexes are rebuildable and never become authority.

Ledger does not duplicate every quote or bar. It links to canonical sequence
ranges and promotes only operationally meaningful events.

### Automatic entries

Northstar appends entries for:

- session open/close and operator-selected session plan;
- macro releases, revisions, and high-impact release windows;
- data gaps, stale feeds, clock faults, catalog/contract changes, and recovery;
- reconciliation start/completion/failure and unknown venue objects;
- account baseline, daily loss threshold changes, and risk-policy changes;
- strategy observation/candidate/risk/intent later, when those systems exist;
- order request, venue acknowledgement/rejection/cancel/replace;
- partial/full fill, fees, slippage, and duplicate-fill suppression;
- position open/resize/stop-target change/close;
- realized and unrealized P&L attribution snapshots;
- manual intervention, emergency action, incident, and resolution.

Automatic ingestion is idempotent by source identity. Reconciliation may append
new evidence or a correction, but it never rewrites the prior receipt.

### Operator entries

The quick composer and full editor support:

**Before the trade**

- thesis and catalyst;
- intended entry zone, invalidation, targets, and maximum risk;
- setup/playbook, direction, session, and expected holding time;
- pre-trade checklist, confidence, and conditions that cancel the idea;
- screenshot, document, URL, and linked macro release.

**During the trade**

- timestamped note or screenshot;
- observed market change;
- stop/target rationale;
- deviation from plan and intervention reason;
- emotion/focus tag without contaminating machine evidence.

**After the trade**

- exit reason and whether it matched the plan;
- planned versus actual entry, stop, size, and target;
- R multiple, realized P&L, fees, slippage, MFE, MAE, and duration;
- execution quality and process-adherence ratings;
- mistakes, strengths, lessons, and follow-up tasks;
- reusable review template and playbook tag updates.

Autosave creates local drafts. Publishing a note creates the immutable event.
Amendment history remains visible. A redaction requires a reason and preserves
the hash and audit link while hiding sensitive body content in normal views.

### Ledger pages

Ledger becomes five first-class workspaces:

1. **Trade Book** — one row per `TradeCase`, with state, index, direction,
   plan, position, realized/unrealized P&L, R, MFE/MAE, duration, adherence,
   review state, and tags.
2. **Timeline** — the current causal lifecycle view with Decisions, Orders,
   Fills, Positions, Reconciliation, and Incidents as fast filters.
3. **Review Queue** — trades missing a plan, note, screenshot, classification,
   or post-trade review; unknown venue outcomes stay pinned above this queue.
4. **Calendar** — session/day P&L and R with click-through into that day's cases,
   macro releases, incidents, and operator notes.
5. **Analytics** — descriptive, filterable journal statistics only: expectancy,
   hit rate, profit factor, average R, drawdown, MFE/MAE capture, slippage,
   duration, time-of-day, setup, index, direction, and adherence. No predictive
   model or Markov score enters this phase.

### Ledger working layout

```text
+--------------------------------------------------------------------------+
| mode | account | date/session | index | tags | review | search | export   |
+----------------------+--------------------------------+------------------+
| facets and calendar  | trade book or event timeline   | case inspector   |
| saved views          | virtualized dense rows         | causal chain     |
| review queue         | honest empty/loading states    | evidence + notes |
| daily totals         |                                | quick composer   |
+----------------------+--------------------------------+------------------+
```

The inspector contains Summary, Plan, Timeline, Execution, Risk, Macro Context,
Attachments, Review, Provenance, and Amendment History. Links from Desk, Macro,
Fund, and Systems open Ledger with a stable case, entry, receipt, order,
position, release, or incident ID—not a fragile row number.

### Search, filters, and storage

- Dense headers and timestamps remain contiguous.
- Stable typed integer IDs address hot records.
- `hashbrown` maps external IDs at ingestion boundaries.
- Roaring bitmaps back multi-select facets such as index, setup, status, and
  tag when history warrants them.
- Normalized UTF-8 text is indexed out of the render path using `memchr` for
  scanning; query results return entry IDs, not copied formatted rows.
- Mmap-backed sealed segments support zero-copy historical traversal.
- The active segment remains single-writer and publishes immutable batches.
- Attachments are size-limited, MIME-validated, deduplicated, and never loaded
  until the inspector asks for them.

### Security, recovery, and export

- API credentials and refresh tokens live in Windows-protected secret storage,
  never in receipts, logs, exports, or screenshots.
- Journal notes may contain sensitive information; cold bodies and attachments
  require encrypted-at-rest storage before live-account use.
- Crash recovery truncates only an incomplete tail and records the recovery.
- Backup is incremental and hash-verified.
- JSON exports preserve schema, IDs, provenance, and amendment history.
- CSV exports are flat filtered views, not authoritative re-import formats.
- Markdown trade reports combine human-readable narrative with evidence links.
- Export policy checks provider licensing before including source-derived data.

## Configuration and operating modes

The first-run/settings workflow configures:

- Northstar data and backup directories;
- Massive entitlement and API-key secret handle;
- TradeLocker demo/live environment, server, account selection, and secret
  handle;
- six canonical index bindings and TradeLocker venue instruments;
- official macro keys and enabled regional adapters;
- currency/time-zone display preferences;
- data retention and export policy;
- journal encryption and backup recovery material.

Modes are explicit:

- **Offline Replay** — no external I/O; historical journal drives the same UI.
- **Live Data** — real market/macro and optional read-only venue/account state.
- **Paper** — future Nautilus-backed simulated execution.
- **Live** — future, unavailable until all arming and execution phases pass.

Mode is always visible in the top bar and every ledger record.

## Implementation sequence

### Slice 0 — freeze contracts and remove ambiguity

Build:

1. `DataState`, value/series metadata, domain generations, and stable IDs.
2. `OfficeSnapshotPort`, domain snapshot shapes, and change bitset.
3. configuration schema, secret-handle interface, and operating-mode type.
4. fixture feature boundary and production empty/setup snapshots. **Implemented.**
5. source-to-visible-field inventory as an executable coverage test. **Pending.**

Exit test:

> The production binary can start with no credentials, renders no fabricated
> value, and explains exactly which configuration or first receipt is missing.

### Slice 1 — HeroFX signal and execution market to the live Desk

Build:

1. confirm and record HeroFX/MT5/TradeLocker licensing, retention, and internal
   research rights — **architecture is fail-closed; the TradeLocker desktop
   account is verified but durable-retention rights remain pending**;
2. discover separate MT5 and TradeLocker index bindings and contract versions —
   **TradeLocker candidate symbols are observed; API IDs/routes/contracts remain
   pending and MT5 stays deferred**;
3. implement the sensor-only MQL5 to Rust fixed-frame named-pipe bridge;
4. capture golden MT5 ticks/history and TradeLocker config, quote, history,
   error, entitlement, and reconnect responses;
5. detect gaps, duplicates, stale values, reconnects, and entitlement failures —
   **generic raw/canonical duplicates and stale/disconnected states implemented;
   HeroFX-specific proof awaits sanitized REST capture**;
6. preserve reference, signal, and execution prices as separate authorities;
7. implement Market projector and segmented Desk snapshots. **Implemented.**
8. bind GPUI labels and native chart to live domain generations. **Implemented:
   direct chunk traversal, visible-range search, gap-preserving timeframe slots,
   zoom/pan/crosshair, and event-driven invalidation; provider activation awaits
   licensed HeroFX REST evidence.**

Massive remains an optional independent reference sub-slice. Its decoder,
catalog audit, and entitlement gate are implemented, but its current account is
not permitted to become Northstar's durable signal source.

Exit test:

> Licensed live and replayed HeroFX MT5/TradeLocker sessions produce the same
> separated signal/execution Desk state; disagreements and gaps remain visible,
> and disconnect becomes stale without fixture fallback.

### Slice 2 — HeroFX TradeLocker read-only venue and fund truth

Build:

1. Windows-protected JWT/refresh-token handling;
2. account selection and `/trade/config` capability capture — **desktop LIVE
   selection verified; protected REST capture pending**;
3. instrument, INFO/TRADE route, session, precision, lot-step, and sizing
   contract discovery;
4. typed golden decoders for quote, history, account state, positions, active
   orders, final orders, and errors;
5. adaptive scheduler honoring discovered route limits;
6. coherent reconciliation epoch with per-call receipt range;
7. unknown order/position detection and resolution workflow;
8. venue, Fund, exposure, and Systems snapshot projectors.

Exit test:

> After restart, Northstar reconstructs the same last reconciled account state,
> identifies every venue object by its correct ID type, and marks an incomplete
> epoch stale rather than publishing a mixed snapshot.

### Slice 3 — official macro to Macro and Desk context

Build in the regional order defined by `GLOBAL_MACRO_SYSTEM_PLAN.md`:

1. US official observations/releases and vintages — **BLS CPI and core labor
   observation path implemented; BLS CPI and Employment Situation schedule
   provenance, FRED rates, BEA GDP/core PCE, and Census MARTS retail sales now
   share raw receipts, canonical events, durable replay, and Ledger material;
   live credentialed transport proof remains pending keys**;
2. CFTC TFF positioning — **implemented for US100, US500, US30, and JP225;
   DE40 and UK100 explicitly report no CFTC TFF contract**;
3. Europe, UK, and Japan adapters — **Eurostat HICP, unemployment, euro-area
   GDP, German retail and industry; ECB deposit and refinancing rates; ONS CPI,
   GDP, unemployment and retail; BoE Bank Rate; and BOJ call rate and Tankan are
   implemented through source-atomic raw/canonical publication. e-Stat remains
   credential-gated until an application ID and exact table dimensions exist**;
4. official source-document receipts — **BLS release calendars and ONS, BoE,
   and BOJ dataset snapshots emit canonical provenance events and separate
   typed Ledger material receipts**;
5. release calendar and late/missing/revision states;
6. Macro projector and Desk next-event/context slice — **typed Macro projector,
   immutable snapshot, live GPUI page, restart replay, and regional factual Desk
   context are implemented. Desk and Macro consume the same snapshot authority;
   page subscriptions skip unrelated domain redraws**.

Exit test:

> An as-of replay never sees a later vintage, every displayed observation opens
> its provenance, and a missing or revised release is visible without changing
> historical truth.

### Slice 4 — Ledger foundation and automatic trade cases

Build:

1. stable ledger/case/actor/correlation identities — **implemented**;
2. hot headers, cold body pack, and recovery — **implemented; attachments pending**;
3. automatic material-event reducer — **command, case correlation, canonical
   ranges, and BLS/BEA/Census/FRED/CFTC/Eurostat backfill implemented;
   venue/reconciliation/risk
   reducers await their owning sources**;
4. TradeCase projection — **manual cases implemented; venue correlation pending**;
5. idempotence and correction/amendment semantics — **typed operator commands,
   linked amendments/redactions, restart recovery, and machine-evidence
   immutability implemented**;
6. mmap query and rebuildable indexes — **mmap reader implemented; indexes pending**;
7. Trade Book, Timeline, and inspector snapshots — **implemented for machine
   receipts and typed Plan/Observation/Intervention/Review entries**;
8. links from every other page — **pending stable cross-page identities**.

Exit test:

> A multi-order, partial-fill, partially closed trade reconstructs into one case
> with a complete causal timeline after restart and exact replay.

### Slice 5 — operator journal and review office

Build:

1. quick typed Plan/Observation/Intervention/Review composer — **implemented;
   richer structured plan/review forms pending**;
2. drafts, publish, amend, redact, tag, and attachment commands — **local drafts,
   publish, amend, and redact implemented; recovered drafts, tags, and files
   pending**;
3. Review Queue and Calendar;
4. deterministic planned-versus-actual, R, MFE/MAE, slippage, duration, and
   adherence calculations;
5. descriptive Analytics saved views;
6. JSON/CSV/Markdown export and verified backup/restore.

Exit test:

> An operator can create a plan before a trade, have venue activity attach
> automatically, add during-trade evidence, complete a review, find it after a
> restart, and export a self-contained report with amendment history.

### Slice 6 — finished-product cutover

Build:

1. remove fixture assumptions and row-index navigation from all pages;
2. virtualize large tables and lazy-load inspectors/documents;
3. complete settings, account switcher, notifications, reconnect, and recovery
   surfaces;
4. replace prototype banners with truthful mode/readiness status;
5. complete keyboard navigation, focus, empty states, errors, and accessibility;
6. perform live visual checks at 1600x900 and compact widths without stopping
   the separately running Phoenix app.

Exit test:

> The default binary contains no production path to `office_fixture`, every
> visible datum has lineage and state, all useful controls work, and the only
> disabled controls are explicitly deferred execution actions.

### Slice 7 — Nautilus and execution, later

Only after the preceding gates:

```text
Northstar candidate and TradeIntent
    -> Nautilus portfolio/risk/execution semantics
    -> TradeLocker command adapter
    -> venue reconciliation
    -> automatic Ledger receipts
```

This slice is deliberately not smuggled into the real-data frontend work.

## Verification and performance gates

### Correctness

- Golden provider payloads cover success, malformed, missing, duplicate, late,
  revised, disconnected, unauthorized, not-entitled, and rate-limited cases.
- Projectors are deterministic across live publication, exact replay, as-of
  replay, restart, and different batch boundaries.
- No future macro vintage or later venue result leaks into earlier replay.
- Incomplete reconciliation epochs never publish mixed account truth.
- Duplicate provider events and fills are idempotent with retained evidence.
- Every automatic ledger entry traces to canonical sequence or venue receipt.
- Manual amendments and redactions preserve immutable lineage.
- Export/import and backup/restore round trips are hash verified.

### UI behavior

- No provider or disk work occurs on the GPUI thread.
- No event means no application redraw.
- A quote update does not rebuild Macro, Fund, Systems, or Ledger snapshots.
- Crosshair movement invalidates only the overlay layer.
- App selection survives snapshot changes by stable ID.
- Stale and disconnected transitions are visible within one publication cycle.
- Empty, loading, unavailable, and invalid states have finished visual designs.
- Periodic computer-use checks verify the actual running UI and interaction,
  not screenshots generated from isolated fixtures alone.

### Initial measurable budgets

These are guardrails to benchmark and revise with recorded evidence:

- committed event to projected hot snapshot: under 10 ms p95 locally;
- snapshot notification to visible paint: under one 60 Hz frame p95 when the UI
  is otherwise idle;
- zero full-history chart copies during an active-candle update;
- zero allocations in the per-sample bar aggregation inner loop;
- Ledger filtered query: under 50 ms p95 at one million hot headers;
- Ledger scrolling: no cold-body or attachment loads without inspector demand;
- bounded provider queues with explicit overflow incidents, never silent loss;
- bounded RSS under a captured full-session and one-million-ledger-entry test.

Benchmarks cover decode, journal append, replay, bar projection, snapshot
publication, filter intersection, text query, and trade-case reconstruction.
Allocation profiles and flamegraphs are required before introducing lock-free or
allocator-specific complexity.

## Definition of “no longer a prototype”

The application earns that label only when all of these are true:

- the default composition root is `NorthstarRuntime`;
- fixtures are compile-time test/example assets only;
- title, mode bar, and readiness language describe actual operating state;
- Desk displays real reference data and honest venue availability;
- Fund displays one complete reconciled account epoch or a clear unavailable
  state;
- Macro displays official observations, vintages, and source documents;
- Systems reports real pipeline, storage, clock, contract, and reconciliation
  health;
- Ledger automatically builds durable trade stories and accepts durable manual
  plans, notes, evidence, and reviews;
- restart/replay recreate the same state;
- no stale value appears live, no unknown appears zero, and no failure reveals
  fixture data;
- the UI is quiet when state is quiet and updates only affected regions;
- execution remains disabled until its separate Nautilus and live-safety gates
  have been implemented and proven.

At that point Northstar is a real operating quant office even before it is
allowed to transmit an order: it observes real markets, knows venue/account
truth, preserves macro vintages, explains its own health, and remembers every
trade and operator decision.
