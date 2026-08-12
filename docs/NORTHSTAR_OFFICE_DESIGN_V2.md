# Northstar Office design direction V2

## Product thesis

Northstar is a personal indices trading office for one operator, six indices,
one explicit data plane, and one guarded execution path. It is not a generalized
terminal, broker storefront, crypto exchange, social feed, or imitation
Bloomberg workstation.

The product should answer five questions quickly:

1. What matters now?
2. What does Northstar know, and how fresh is it?
3. What is the system doing or refusing to do?
4. What capital is exposed and why?
5. Can every conclusion be traced back to source and venue truth?

The five top-level rooms are:

```text
Desk | Macro | Fund | Systems | Ledger
```

- **Desk** is the live operating surface.
- **Macro** is the global context and release room.
- **Fund** is capital, risk, and portfolio truth.
- **Systems** is infrastructure, data, replay, and execution health.
- **Ledger** is the immutable causal record.

## What to take from the references

The Dribbble trading-dashboard collection and supplied screenshots are useful
for composition, not product requirements.

Keep:

- one dominant work surface instead of a wall of equal cards;
- thin headline metric rails;
- modular secondary panels with consistent headers;
- restrained rounding and borders;
- focused detail rails and drawers;
- compact, high-quality tables;
- charts that have breathing room;
- obvious selection and hover states;
- a clear difference between summary, evidence, and action.

Reject:

- wallet/deposit/buy-sell storefront controls;
- giant balance numbers with no operating context;
- decorative donuts and speedometer gauges;
- neon background glows and glass effects;
- dense red/blue heatmap walls;
- color as the only carrier of direction;
- crowd sentiment as a core input;
- an unexplained all-knowing score;
- twenty-five macro columns visible at once;
- feeds, widgets, and instruments outside the six-index mandate.

## Visual character

Northstar should feel like a calm night office: dark graphite, precise mint
light, warm warning amber, and rare coral intervention. It should look serious
without becoming sterile.

### Core palette

| Token | Role | Direction |
| --- | --- | --- |
| Canvas `#101312` | window background | nearly black green |
| Surface `#171B1A` | primary panels | graphite |
| Raised `#1B201E` | rows, cards, controls | warm charcoal |
| Active `#17332C` | selected context | deep mint |
| Border `#303936` | structural separation | low contrast |
| Text `#E2EBE8` | primary copy | soft white |
| Muted `#7C8A85` | labels and metadata | sage gray |
| Mint `#57DFBB` | healthy, selected, favorable | primary accent |
| Seafoam `#9FE6D4` | charts and secondary positive context | light accent |
| Lavender `#A8B6FF` | comparison and reference series | analytical accent |
| Amber `#F0B862` | upcoming, waiting, guarded | attention |
| Coral `#E47069` | loss, block, breach, stale | exceptional danger |

Coral is not the default color for every negative number. A declining series
can remain neutral text with a down arrow. Coral is reserved for operating
consequence: breached, stale, rejected, unknown, or loss beyond a threshold.

### Direction encoding

Every directional value uses at least two signals:

```text
arrow or sign + label or number + optional restrained color
```

Examples:

- `↑ accelerating` in mint;
- `↓ cooling` in neutral text;
- `! stale 18m` in coral;
- `• unchanged` in muted sage;
- `T-14m` in amber.

This avoids the red-versus-blue grid from the references and remains legible to
color-impaired users.

## Information hierarchy

Each page follows the same four levels:

1. **Operating header** — page purpose, clock/mode, and the most important state.
2. **Situation strip** — four to six small metrics, each with source/freshness.
3. **Primary work surface** — chart, matrix, pipeline, or ledger table.
4. **Inspector** — a contextual rail or drawer containing provenance and detail.

The user should never have to scan twelve equally weighted cards to discover
the page's purpose.

### Shared 1600 x 900 shell

```text
+-----------------------------------------------------------------------+
| NORTHSTAR / INDEX | Desk Macro Fund Systems Ledger | health | mode    |
+-----------------------------------------------------------------------+
| situation strip: next risk / freshness / capital / venue / clock      |
+-------------+-----------------------------------------+---------------+
| context     |                                         | inspector     |
| list or     |          primary work surface           | evidence or   |
| filters     |                                         | guarded action|
|             |                                         |               |
+-------------+-----------------------------------------+---------------+
| optional event timeline, receipt strip, or secondary table            |
+-----------------------------------------------------------------------+
```

The context rail may collapse below 1460 logical pixels. The inspector becomes
a right-side overlay drawer on compact layouts. The primary work surface must
never be squeezed into decorative irrelevance.

## Shared primitives

The detailed pages should be assembled from a small reusable GPUI vocabulary:

- `OfficeShell`
- `PageHeader`
- `SituationStrip`
- `PanelHeader`
- `MetricCell`
- `StatusPill`
- `FreshnessStamp`
- `DenseTable`
- `GroupedTableHeader`
- `MicroTrend`
- `RangeBar`
- `EventCountdown`
- `EvidenceChip`
- `SourceBadge`
- `InspectorDrawer`
- `ReceiptTimeline`
- `EmptyTruthState`
- `GuardedAction`

These primitives should be host-agnostic so the standalone page and a future
Phoenix host share the exact same rendering contract.

## Typography and number treatment

- Human-readable sans serif for headings, labels, and explanations.
- Tabular numerals for all prices, percentages, timestamps, and identifiers.
- Monospace only for receipt IDs, hashes, provider codes, and raw venue IDs.
- Sentence case for page titles; compact uppercase only for section rails.
- No label smaller than is comfortably readable on a 1600 x 900 desktop at
  Windows 125% scaling.
- Numbers align by decimal where comparison matters.
- Units stay visible and are never inferred from color or column position.

## Motion and interaction

Motion communicates change, not decoration.

- Snapshot updates cross-fade or tick once; panels do not pulse continuously.
- A new release briefly highlights the changed cells, then settles.
- Stale state uses a static warning treatment, not flashing red.
- Drawers use short, deterministic transitions.
- Chart cursor and selection feedback stay display-linked.
- Reduced-motion mode disables nonessential transitions.

The office is keyboard-first:

| Key | Action |
| --- | --- |
| `1`..`5` | move between the five rooms |
| `Ctrl+K` | command/search palette |
| `/` | focus current-page filter |
| `J` / `K` | move through table or receipt rows |
| `Enter` | open inspector |
| `Esc` | close inspector or modal |
| `Space` | pin/unpin selected evidence |
| `G` | open live-arming gate review from Desk |

## The personal-office test

Every proposed component must pass these questions:

- Does it help operate the six-index fund today?
- Does it reveal truth, provenance, risk, or causality?
- Can it be understood without a vendor-specific legend?
- Is it still useful with no network connection during replay?
- Does it reduce the need to keep a spreadsheet or broker tab open?

If not, it does not belong in Northstar V2.

## Explicit design exclusions

- No Markov, HSMM, regime-learning, or opaque AI score surfaces.
- No new technical indicator suite.
- No crowd sentiment gauge.
- No generic multi-asset watchlist.
- No embedded news firehose.
- No broker account funding or wallet UX.
- No social, copy-trading, competition, or leaderboard features.
- No full terminal layout with freely tiled arbitrary widgets.
- No red/blue bullish-versus-bearish design language.
- No page that can visually imply live permission when only health gates pass.
