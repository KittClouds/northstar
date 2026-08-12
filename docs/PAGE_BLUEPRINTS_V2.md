# Northstar page blueprints V2

## Navigation and page ownership

```text
Desk | Macro | Fund | Systems | Ledger
```

Each page owns one operating concern. Cross-page summary cards link to detail;
they do not duplicate an entire page in miniature.

| Page | Primary question | Authoritative snapshot |
| --- | --- | --- |
| Desk | What needs my attention now? | market + position + guarded decision |
| Macro | What global context is known and what releases are next? | macro office |
| Fund | What capital and risk are committed? | portfolio/risk |
| Systems | Can I trust the machinery and its latest publication? | system health |
| Ledger | Why did anything happen, and can I prove it? | immutable receipts |

## 1. Desk — mission control

### Purpose

Desk remains the default page. It should support a session without requiring
the operator to jump between broker, chart, spreadsheet, and release-calendar
tabs.

### Layout

```text
+-----------------------------------------------------------------------+
| next event | Massive | venue | macro as-of | risk used | SIM/GUARDED   |
+-----------+---------------------------------------------+-------------+
| 6-index   | active index header + reference/execution  | risk and    |
| universe  +---------------------------------------------+ execution   |
|           |                                             | inspector   |
|           | native chart + decision/risk annotations    |             |
|           |                                             |             |
+-----------+---------------------------------------------+-------------+
| session plan / recent decisions / orders / positions / events         |
+-----------------------------------------------------------------------+
```

### Header

Show:

- selected index and friendly name;
- Massive reference price;
- TradeLocker executable price;
- basis and spread;
- market session state;
- local/exchange clock;
- next material release countdown;
- current mode.

Provider identity is visible but subordinate. The index and operating state are
the headline.

### Index universe rail

One row per supported index:

- symbol and session;
- reference price and daily change;
- basis status;
- open risk or `flat`;
- next local macro event;
- freshness marker.

Avoid mini scorecards. Selection is a deep-mint surface with a slim accent,
never a bright filled heatmap cell.

### Native chart

The chart remains the dominant surface and adds only operational overlays:

- reference candles/bars;
- execution/reference basis marker;
- position entry, stop, target, and average fill;
- open-risk band;
- decision markers linked to receipt IDs;
- macro release vertical markers;
- session boundaries;
- stale-data interruption marker.

No indicator menu is added in this phase. Overlays must be individually
toggleable and derived from existing authoritative snapshots.

### Right inspector

The rail is divided into three small sections:

1. **Position truth** — quantity, entry, stop, open P&L, max stopped loss,
   order/position IDs.
2. **Risk permission** — daily risk, portfolio heat, correlation contribution,
   event lockout status.
3. **Guarded controls** — review arming, pause new intents, global halt.

The six operational arming gates stay explicit. A seventh downstream section
shows strategy/risk/policy permission separately so green health never reads as
automatic trade permission.

### Bottom workspace

Tabs:

- **Session plan** — next events, operator note, risk posture, indices in focus.
- **Decisions** — latest decision receipts.
- **Orders** — working and recently completed orders.
- **Positions** — current venue positions.
- **Events** — macro releases, session changes, provider incidents.

The session plan makes the product feel like a home quant office rather than a
chart with broker controls.

### Desk acceptance

- The operator can identify next event, open risk, and live eligibility within
  five seconds.
- No critical truth is hidden behind hover.
- Broker IDs are one click away but do not dominate the page.
- The chart remains useful at the 1460 compact breakpoint.
- A stale feed visually interrupts the affected index without turning the whole
  window red.

## 2. Macro — global context room

### Purpose

Macro replaces the giant red/blue EdgeFinder grid with an auditable,
revision-aware context room. It exposes facts and transparent transforms; it
does not pretend one number can summarize the world.

### Primary layout

```text
+-----------------------------------------------------------------------+
| as-of | next release | late releases | source health | region filter  |
+-----------------------------------------------------------------------+
| region cards: United States | Euro area | United Kingdom | Japan      |
+----------------------+--------------------------------+---------------+
| index context matrix | release/event table            | inspector     |
| 6 rows x 6 families  | actual/prev/revised/freshness  | provenance    |
|                      |                                | revision trail|
+----------------------+--------------------------------+---------------+
| positioning | rates/yields/currency context | official documents     |
+-----------------------------------------------------------------------+
```

### Situation strip

- Snapshot as-of time and catalog version.
- Next scheduled operating release and countdown.
- Number of late/missing releases.
- Number of stale source families.
- Last successful atomic publication.
- Replay/live badge.

### Region cards

Four compact cards show United States, euro area/Germany, United Kingdom, and
Japan.

Each card contains:

- latest growth observation;
- latest inflation observation;
- latest labor observation;
- current policy rate;
- 2Y/10Y or local equivalents;
- local currency context;
- next release;
- oldest stale series.

Values have source badges and freshness. Cards are navigational summaries, not
scores.

### Index context matrix

Rows are only the six supported indices. Columns are grouped families:

```text
Index | Growth | Inflation | Labor | Rates | Positioning | Event risk
```

Each cell contains:

- a plain-language state such as `accelerating`, `cooling`, `tightening`,
  `extended`, or `unavailable`;
- the latest value/delta or positioning percentile;
- a tiny trend glyph;
- freshness/source marker.

Cell backgrounds stay graphite. Mint, lavender, amber, and coral appear as
small text, edge, or glyph accents. This preserves scanability without the ugly
full-cell red/blue treatment.

V1 does not display `Very Bullish 9`. If a future transparent composite is
approved, every contribution must be expandable and versioned.

### Release table

Default columns:

```text
Time | Region | Release | Actual | Previous | Revised | Change | Source | Freshness
```

Optional later columns:

```text
Our expected | Our surprise
```

They appear only when a timestamp-causal expectation artifact exists. They are
never labeled `Forecast` or `Consensus`.

Rows are grouped into:

- upcoming;
- released today;
- revisions;
- late/missing.

An expanded row shows period semantics, unit, seasonal adjustment, raw provider
code, receipt hash, publication time, ingestion time, and prior vintages.

### Positioning panel

The panel replaces crowd sentiment.

For the selected index/contract mapping show:

- asset manager net;
- leveraged money net;
- dealer net;
- weekly change;
- historical percentile;
- open interest;
- report date and publication lag;
- mapping confidence/status.

Use horizontal range bars and numeric labels, not a speedometer.

### Rates, yields, and currency context

This is a compact comparison chart, not another full trading chart.

Examples:

- US100 with US 2Y and USD context;
- DE40 with Bund/ECB and EUR;
- UK100 with gilts/BoE and GBP;
- JP225 with JGB/BOJ and JPY.

Every series has its own scale/unit label. Correlation or causal claims are not
implied merely because lines share a panel.

### Official documents

Show a restrained list of recent central-bank/statistical-agency releases:

- institution and document type;
- title;
- publication time;
- affected regions/indices;
- source/receipt badge;
- pinned/read state.

GDELT headlines live in a separately labeled `Broad antenna` sub-tab so official
and third-party material cannot be visually confused.

### Macro inspector

The inspector has four tabs:

- **Observation** — value, unit, period, source.
- **Revisions** — vintage timeline and deltas.
- **Provenance** — provider, raw receipt hash, catalog mapping.
- **Impact path** — affected indices and which page components consume it.

Impact path describes wiring, not predicted direction.

### Macro acceptance

- Every visible number can open source and revision provenance.
- Current and point-in-time values are clearly distinguishable.
- Missing data displays `unavailable`, never zero.
- Late releases are separate from stale market data.
- The six-index matrix fits at 1600 x 900 without horizontal scrolling.
- No raw provider request occurs on the GPUI thread.

## 3. Fund — capital and risk office

### Purpose

Fund answers how the office is performing, where risk is concentrated, and what
capacity remains. It avoids vanity portfolio metrics and exchange-style asset
allocation donuts.

### Layout

```text
+-----------------------------------------------------------------------+
| NLV | today R/$ | drawdown | portfolio heat | remaining daily risk    |
+-------------------------------------------+---------------------------+
| equity + drawdown + high-water mark       | limit ladder + stress     |
+---------------------+---------------------+---------------------------+
| index exposure      | system allocation   | session/calendar risk     |
+---------------------+---------------------+---------------------------+
| daily attribution and closed-trade ledger summary                     |
+-----------------------------------------------------------------------+
```

### Headline metrics

- net liquidation value;
- realized, unrealized, and total day P&L;
- day result in R;
- current/max drawdown;
- portfolio heat;
- remaining daily risk capacity.

Each value has an as-of timestamp and authority badge.

### Equity and drawdown chart

- equity and high-water mark;
- drawdown lane below, sharing time axis;
- deposit/withdrawal markers if they ever exist;
- daily stop/halt markers;
- selectable windows: 20 sessions, quarter, inception;
- tooltip with realized/unrealized split.

### Limit ladder

Show current/limit and permission for:

- daily loss;
- portfolio heat;
- per-index exposure;
- correlated US-index cluster exposure;
- gross/net exposure;
- open positions;
- event lockout;
- stale data or unknown venue truth.

Limits use calm progress bars until attention is required.

### Exposure views

Two small tables replace generic pies:

1. **By index** — notional, risk R, P&L, correlation cluster, session.
2. **By system** — allocated risk, used risk, realized R, open R, disposition.

Rows expand into their supporting positions and receipts.

### Stress card

Deterministic scenario checks only:

- all stops filled at defined slippage;
- correlated US-index shock;
- gap beyond stop by configured points;
- TradeLocker disconnect with positions open;
- reference/execution basis breach.

This is policy arithmetic, not probabilistic forecasting.

### Attribution

Daily and session attribution by index, system, realized/unrealized, fees, and
slippage. Every cell links to filtered Ledger receipts.

### Fund acceptance

- Dollar and R views always reconcile.
- Realized and unrealized values are never blended without labeling.
- Every exposure joins through venue position identity, not symbol text.
- Stress assumptions are visible beside their results.
- Limit cards link directly to the gate or receipt that set their status.

## 4. Systems — operational control room

### Purpose

Systems becomes the trust page for the entire office. Strategy evidence can
remain as a secondary tab, but the default view is data, engine, venue, clock,
storage, replay, and publication health.

### Layout

```text
+-----------------------------------------------------------------------+
| overall readiness | publication seq | clock | storage | replay        |
+-----------------------------------------------------------------------+
| pipeline map: sources -> normalize -> store -> snapshot -> engines     |
+----------------------+-----------------------------+------------------+
| providers/catalog   | event + incident timeline   | selected detail  |
+----------------------+-----------------------------+------------------+
| macro coverage | Massive gaps | Nautilus state | TradeLocker reconcile|
+-----------------------------------------------------------------------+
```

### Pipeline map

Nodes:

- Massive market data;
- official macro adapters;
- CFTC TFF;
- policy documents/GDELT;
- normalization/catalog;
- receipt and revision stores;
- snapshot publisher;
- Nautilus data/risk/portfolio/execution;
- TradeLocker execution/reconciliation;
- GPUI snapshot consumer.

Edges show last batch time, item count, and state. The map is fixed and
purpose-built; it is not an arbitrary node editor.

### Health groups

**Data**

- Massive latency/gaps;
- macro adapter last success;
- next scheduled fetch;
- late/missing releases;
- catalog coverage;
- latest snapshot ID.

**Engine**

- Nautilus node state;
- reconciliation epoch;
- cache/order/position counts;
- pending command outcomes;
- event backlog.

**Venue**

- TradeLocker stream and SyncEnd;
- account snapshot age;
- unknown orders/positions;
- instrument contract agreement;
- route/session state.

**Machine**

- clock drift;
- storage usage;
- mmap generation;
- background queue depth;
- last deterministic replay;
- build/schema versions.

### Incident timeline

Every transition is a receipt:

- provider disconnected/recovered;
- batch quarantined;
- publication skipped;
- release late;
- clock drift exceeded;
- venue resynchronized;
- contract changed;
- live eligibility removed;
- global halt invoked.

Rows open a structured incident drawer with cause, affected consumers, recovery,
and linked receipts.

### Data coverage matrix

Rows are region/source families; columns are cataloged, historical coverage,
latest release, freshness, revision health, and replay status. Use text and
small range bars, not red/blue cells.

### Strategy runtime secondary tab

Keep the useful parts of the current Signal Lab:

- loaded systems and exact revision;
- current disposition;
- latest decision trace;
- explicit promotion/evidence gates;
- shadow/simulation agreement.

Rename the tab `Strategy runtime`; do not make it the default Systems page.
Indicators and learned regime models remain excluded.

### Systems acceptance

- A green provider connection cannot mask a broken catalog or stale snapshot.
- Every readiness group has independent gates.
- The operator can locate the cause of a blocked live gate in two interactions.
- Pipeline visualization remains readable at 1600 x 900.
- Incident history survives restart through receipts.

## 5. Ledger — immutable causal record

### Purpose

Ledger becomes Northstar's institutional memory. It is not merely a transaction
history. It connects observations, decisions, risk, commands, venue reports,
positions, exits, P&L, and reconciliation.

### Receipt families

```text
Observation
Decision
RiskEvaluation
CommandIntent
VenueOrder
Fill
Position
Exit
Reconciliation
Incident
OperatorAction
```

Each receipt has a stable ID, event time, observed time, schema version, mode,
and links to parent/child receipts.

### Decision receipt V2

```text
DecisionReceiptV2
  receipt_id
  event_time_ns
  observed_time_ns
  index_key
  system_id + system_revision
  operating_mode
  market_snapshot_id
  macro_snapshot_id
  fund_snapshot_id
  trigger
  evidence_refs[]
  gate_results[]
  proposed_risk_r
  proposed_quantity
  proposed_entry / stop / target
  disposition
  route_reason
  command_intent_id?
  venue_order_id?
  venue_position_id?
  latency_breakdown
  provenance_hash
```

Hot receipt linkage uses compact IDs and small fixed collections. Large evidence
text and source documents remain cold artifacts.

### Layout

```text
+-----------------------------------------------------------------------+
| date | index | system | mode | receipt type | status | search/export  |
+-----------------------------------------------------------------------+
| summary: decisions / routed / blocked / fills / unknown / incidents   |
+------------------------------------------------------+----------------+
| dense lifecycle table                                | inspector      |
| time index type system decision risk venue outcome  | causal chain   |
|                                                      | evidence       |
|                                                      | raw IDs        |
+------------------------------------------------------+----------------+
```

### Ledger tabs

- **Lifecycle** — all receipt families in causal order.
- **Decisions** — decisions and gate results.
- **Orders** — command intent through venue order state.
- **Fills** — fill identity, quantity, price, fees, slippage.
- **Positions** — venue position lifecycle and P&L.
- **Reconciliation** — epochs, snapshots, unknown outcomes, resolutions.
- **Incidents** — provider, clock, contract, storage, and operator events.

### Default lifecycle columns

```text
Time | Index | Receipt | System/Source | Decision/Event | Risk | Venue | Status | Latency | ID
```

Columns are resizable within controlled limits and presets, not freely arbitrary.
The operator can save a compact and an audit preset.

### Causal inspector

The right drawer shows:

1. **Summary** — plain-language statement.
2. **Timeline** — observation → decision → risk → intent → order → fill →
   position → exit/reconcile.
3. **Evidence** — market/macro/fund snapshot references and gate outcomes.
4. **Venue truth** — distinct order and position IDs, status transitions, fills.
5. **Latency** — source, decision, risk, route, venue acknowledgment.
6. **Provenance** — schema, revision, hashes, raw/cold artifact links.

Unknown outcomes get an unmistakable coral edge and remain pinned until
resolved. Resolution creates a new receipt; it never mutates history silently.

### Filters and queries

- date/session range;
- index;
- system/revision;
- operating mode;
- receipt family;
- disposition/status;
- with/without order;
- unknown/unreconciled only;
- macro release window;
- receipt ID, intent ID, order ID, position ID.

Search runs over indexes, not by allocating and formatting every receipt.

### Operator notes

An operator may append a note or tag to a receipt. Notes are new receipts with
author/time/provenance, never edits to the original record.

Useful tags:

- `review`;
- `expected`;
- `execution issue`;
- `macro event`;
- `risk lesson`;
- `follow up`.

### Export

V1 export targets:

- selected receipt bundle as versioned JSON;
- current filtered table as CSV;
- human-readable single-trade/incident report as Markdown.

Exports include schema and snapshot references. They never expose credentials.

### Ledger acceptance

- Every routed command traces to exactly one authorized intent.
- Venue order and position IDs remain distinct types and columns.
- Duplicate fills appear once in authoritative state and retain dedup evidence.
- Unknown outcomes cannot be filtered away from the default operating view.
- A historical row renders from immutable receipts after restart.
- Filters over the expected local history respond within 50 ms p95.
- Opening the inspector does not copy full evidence archives.

## Cross-page flows

### Morning/session preparation

```text
Desk session plan
  -> inspect Macro upcoming releases
  -> review Fund risk capacity
  -> verify Systems readiness
  -> remain Simulation/Shadow or review arming eligibility
```

### Decision investigation

```text
Desk decision row
  -> Ledger causal inspector
  -> linked Macro observation/release
  -> linked Fund risk snapshot
  -> linked Systems incident or venue reconciliation
```

### Operational incident

```text
Systems incident
  -> live eligibility removed
  -> Desk guarded state updates
  -> Ledger incident receipt pinned
  -> Fund exposure remains visible
```

## Implementation roadmap

### Phase A — shared page substrate

- Add `Macro` to the navigation contract.
- Extract shared shell, table, inspector, freshness, event, and receipt
  primitives.
- Freeze typography, palette, spacing, and compact behavior.
- Add fixture snapshots before networking.

### Phase B — truth systems first

- Implement macro domain IDs, catalog, observation/vintage store, calendar, and
  replay fixtures from `GLOBAL_MACRO_SYSTEM_PLAN.md`.
- Expand receipt families and causal linkage.
- Publish immutable fixture-only macro and ledger snapshots.

### Phase C — Macro page vertical slice

- Build one complete US fixture slice for US100/US500/US30.
- Render region card, matrix, release table, positioning, and provenance drawer.
- Prove revisions and missing values visually.
- Add Europe, UK, and Japan only after the slice is correct.

### Phase D — Ledger V2

- Build lifecycle indexes and fixed row view models.
- Add tabs, filters, inspector, and causal timeline.
- Link current Desk decisions into the new ledger.
- Add restart/replay and unknown-outcome fixtures.

### Phase E — Systems control room

- Make data/engine/venue/machine health the default.
- Add fixed pipeline view, coverage matrix, and incident timeline.
- Preserve strategy runtime as a secondary tab.

### Phase F — Desk and Fund integration

- Add macro event markers and session plan to Desk.
- Add exposure, deterministic stress, and attribution to Fund.
- Link every detail card into Ledger or Systems rather than duplicating detail.

### Phase G — polish and proof

- Keyboard navigation and command palette.
- 1600 x 900, 1460 compact, 125% DPI, and ultrawide visual checks.
- Accessibility and reduced-motion checks.
- Allocation, table-filter, snapshot-swap, and paint performance gates.
- Replay fixtures for release, revision, fill, incident, and restart paths.

## Scope held outside this roadmap

- Markov/HSMM or learned regime systems;
- technical indicator construction;
- opaque macro or EdgeFinder-style scores;
- paid consensus ingestion;
- PMI workarounds or scraping;
- automatic headline sentiment;
- new instruments beyond the six indices;
- live credentials or real order transmission;
- freeform Bloomberg-style window tiling.
