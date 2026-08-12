# Northstar Index Office

A standalone GPUI operating surface for an indices-only, Northstar-governed home
hedge fund. The production composition root owns durable Ledger and official
macro journals, deterministic replay, immutable domain snapshots, native chart
rendering, and fail-closed provider/venue states. The original visual prototype
remains available as a separate fixture-backed executable; neither executable
can transmit orders.

The Desk chart is a native GPUI canvas with its own logical time/price engine,
visible-range paint plan, wheel zoom, drag pan, high-DPI snapping, crosshair, and
risk overlays. See `docs/NATIVE_CHART_ENGINE.md` for the Lumen-informed design.

The Northstar-owned trading control plane now includes compact lineage IDs,
distinct TradeLocker order/position identities, reconciliation epochs,
idempotent fills, explicit Massive/Nautilus/TradeLocker ports, and fail-closed
routing. See `docs/CORE_ARCHITECTURE.md`.

The first Phase II-B structural surface is executable in `src/market_state`:
canonical M4 bars now produce deterministic session structure, opening ranges,
rolling range projections, an atomic Level Book, compatible structural nodes,
corridors, fast market location, and renderer-only chart projections. Volume
algorithms fail closed until qualified volume exists. See
`docs/STRUCTURAL_MARKET_STATE_RUNTIME.md`.

The MT5-to-Northstar parity bridge is isolated in
`research/auction-parity`. It verifies the sealed RG2 corpus, reconstructs the
Phase 10.5 research interface, and checks Rust auction-grammar semantics against
frozen MQL5 golden fixtures. It is deliberately not wired into the live office
runtime yet; see `research/auction-parity/PHASE12_CHECKPOINT.md`.

The indices-only official macro and CFTC positioning plan is frozen in
`docs/MACRO_DATA_SPINE.md`. Its first operating vertical slices are now
live-capable: four BLS series, BLS release provenance, FRED rates, BEA GDP/core
PCE, Census MARTS retail sales, four defensible CFTC TFF index contracts, and
two German Eurostat series flow through exact raw receipts, typed canonical events,
append-only replay, immutable Macro snapshots, and the GPUI Macro office. DE40
and UK100 remain explicitly unsupported by CFTC rather than receiving proxies.
This adds no scoring or models.

The V2 home-office expansion is specified in:

- `docs/NORTHSTAR_OFFICE_DESIGN_V2.md` — product character, visual language,
  shared layout, and explicit anti-terminal constraints;
- `docs/GLOBAL_MACRO_SYSTEM_PLAN.md` — macro identities, ingestion, revisions,
  replay, publication, observability, and quality gates;
- `docs/PAGE_BLUEPRINTS_V2.md` — detailed Desk, Macro, Fund, Systems, and Ledger
  page contracts plus the truth-systems-first implementation sequence.
- `docs/CANONICAL_DATA_PLANE.md` — implemented L0 receipts, L1 journal,
  live/replay publication, calendars, custom bars, licensing, and exit gates.
- `docs/REAL_DATA_FRONTEND_AND_LEDGER_PLAN.md` — production snapshot cutover,
  provider-to-page mappings, fixture eradication, and the automatic/manual trade
  journal that turns Ledger into Northstar's institutional memory.
- `docs/DURABLE_LEDGER.md` - implemented hot/cold append contract, crash
  recovery, idempotence, mmap projection, and GPUI operator-note path.
- `docs/OFFICIAL_MACRO_RUNTIME.md` - implemented U.S. official macro and
  release provenance, CFTC, and first Eurostat regional cut, including time
  semantics, source-atomic publication, replay, and GPUI provenance.
- `docs/MASSIVE_REAL_DATA_RUNTIME.md` - implemented verified-subset catalog,
  credential-safe REST bootstrap, atomic L0/L1 publication, delayed/live truth,
  replay, failure states, and the remaining verified-discovery gate.
- `docs/MASSIVE_ENTITLEMENT_AUDIT_2026-08-10.md` - live credential, plan,
  ticker, licensing, proxy, aggregate, and snapshot entitlement evidence.
- `docs/HERO_FX_VENUE_DATA_PLAN.md` - HeroFX MT5 signal sensor, HeroFX
  TradeLocker execution authority, three-price roles, rights gates, and the
  fixed-frame bridge boundary.
- `docs/STRUCTURAL_MARKET_STATE_RUNTIME.md` - executable Phase II-B contracts,
  price-only producers, Level Book, node graph, replay fingerprints, and honest
  data-dependent gates.

## Prototype surfaces

- **Desk** — index universe, optional reference vs HeroFX signal and TradeLocker
  execution price, native candlestick chart, decision ledger, exposure, and
  intervention controls.
- **Fund** — capital, drawdown, limits, equity curve, and strategy book.
- **Systems** — strategy decision trace and evidence-backed promotion gates.
- **Ledger** — immutable decision receipt list.

Every number in the visual-prototype binary is deterministic fixture data.
Buttons that would mutate live trading state are visual contracts only. The
separate operating binary contains no fixtures and renders missing authority as
an explicit unavailable state.

## Run

Keep build products off `C:`:

```powershell
$env:CARGO_TARGET_DIR = 'D:\phoenix-target-index-fund-prototype'
cargo test --all-targets
cargo run --bin northstar-index-fund
cargo run --no-default-features --features desktop --bin northstar-operating
cargo run --features desktop,fixtures --bin northstar-operating-replay
```

`northstar-operating-replay` is an explicit canonical-data demonstration. It
drives the production snapshot and native-chart boundaries through the replay
publisher, carries a `REPLAY` badge, and cannot be confused with the empty,
fixture-free `northstar-operating` composition root.

The executable and window title are intentionally distinct from Phoenix Native.
Closing Northstar quits only the prototype process.

## Architecture boundary

```text
external sources -> L0 raw receipts -> canonical decoders -> L1 journal
                                                           |
                                       +-------------------+------------------+
                                       |                                      |
                                    live tail                           replay reader
                                       +-------------------+------------------+
                                                           |
                                              deterministic L2 state
                                                           |
                                              immutable dashboard snapshot
                                                           |
                                                  index-fund GPUI page
```

The prototype GPUI layer reads `DashboardSnapshot` and emits typed control intentions. It
does not own broker truth, calculate authoritative risk, or call TradeLocker
directly. `MarketDataPort` and `TradingControlPort` are the current prototype
seams; production adapters can replace `PrototypeRuntime` without rewriting the
page.

`Bar` is a fixed 32-byte POD presentation record. `MappedBarReplay` preserves
the original chart-fixture reader while the canonical data plane uses versioned
128-byte events and checksummed mmap journals. `BarSeries` keeps
contiguous low/high lanes and dispatches price extent calculation through
`pulp` SIMD.

The production `operating` layer now adds honest availability metadata,
segmented immutable office snapshots, lock-free current-snapshot reads,
coalesced bounded notifications, a fixture-free `NorthstarRuntime`, and the
first live/replay-identical market projector. Chunked canonical bar snapshots
avoid full-history copies during active-market updates. `northstar-operating`
reads `OfficeSnapshotPort` directly, wakes only for coalesced domain changes,
and paints only the visible range of those chunked bars on the native GPUI
canvas. Logical timeframe slots preserve absent intervals as visible gaps.
It starts honestly with no provider values; the original visual prototype and
canonical replay lab remain behind the explicit `fixtures` feature.

The operating root opens real local Ledger and macro journals at
`%LOCALAPPDATA%\Northstar\ledger-v1`. Typed operator plans, observations,
interventions, reviews, amendments, and redactions travel through a bounded
single-writer queue into a 176-byte hot journal plus checksummed cold body pack;
the Trade Book and Timeline update only after the durable commit publishes.
Mutations require an explicit same-case operator parent; machine evidence is
immutable from the operator surface.
Canonical macro batches now reconcile into idempotent machine receipts linked
to their exact sequence ranges. The provider-neutral automatic-event port also
correlates order, fill, position, decision, and P&L lifecycle events into durable
trade cases; actual venue entries remain empty until TradeLocker evidence
exists.

The same single state writer owns Macro publication. Bounded supervisors cover
BLS observations and release schedules, FRED, BEA, Census, CFTC, Eurostat, ECB,
ONS, Bank of England, and Bank of Japan observations. Credentialed sources start
only when their environment key exists. Each refresh fails closed as one
all-or-none canonical batch. Dataset provenance is journaled separately from
observation material, and Desk consumes the same replay-safe regional snapshot
as Macro. GPUI performs no network, disk, JSON, or journal work while painting.

The operating root also contains the Massive real-data socket. When a licensed
`massive.catalog.json` and the environment-only `MASSIVE_API_KEY` are present,
the reviewed exact-ticker subset becomes raw receipts, one canonical batch,
and one Desk generation. Missing catalog/key, entitlement denial, delay,
disconnect, and schema drift remain explicit states. Unchanged snapshots retain
raw evidence without triggering a redraw. No production ticker is guessed, and
no S3 credential is accepted by the REST boundary.

## Planned provider cut

1. Obtain Massive non-display, strategy, retention, derived-work, and export
   rights; then capture a dated verified-subset catalog and golden receipts.
2. Capture TradeLocker account-scoped config, instrument, route, quote, history,
   session, and reconciliation fixtures; do not assume an undocumented stream.
3. Activate e-Stat only after an application ID and exact table dimensions are
   frozen. The U.S., CFTC, Eurostat/ECB, ONS/BoE, and initial BOJ spine is
   implemented; credentialed FRED/BEA/Census transport proof awaits environment
   keys.
4. Publish L2 data-plane snapshots through the existing GPUI ports.
5. Integrate Nautilus only after canonical replay and provider coverage gates.
6. Keep `NorthstarApp` reusable so standalone and Phoenix hosts provide distinct
   composition roots.

Nautilus component crates are deliberately not linked into this prototype. The
verified toolchain-compatible target is the exact `0.60.0` component family,
which will live behind a dedicated integration crate after licensing and
packaging review. Indicators, Markov/HSMM models, and TradeLocker Studio are out
of scope for this core slice.

## Native chart direction

Northstar does not embed a browser chart or add a second GPU surface. Lumen
Charts is used as an architectural study only. Its backend-independent scales,
visible-range clipping, invalidation hierarchy, cached bottom scene, and cheap
crosshair layer map naturally to GPUI's canvas and event primitives.

## Safety contract

Live arming fails closed unless market data is fresh, TradeLocker is synced,
NautilusTrader has reconciled, the clock is synchronized, and there are no
unknown orders. Northstar's cached instrument contract must also agree with
TradeLocker on sizing, tick/precision, margin, route, and session truth. Passing
these gates only makes the system eligible to arm; strategy, portfolio/risk,
and execution policy retain their own vetoes. Arming is not persisted across
restart or session boundaries.
