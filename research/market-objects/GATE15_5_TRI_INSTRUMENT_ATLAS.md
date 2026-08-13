# Gate 15.5 — Tri-instrument market-object atlas

Gate 15.5 is a deterministic relational atlas over the 42 sealed RG3 runs. It does not discover, rename, merge, or promote families.

The three instruments remain distinct:

- Master Controller: causal structural point-state (`SPACE`).
- Compression: containment-process coordinates (`CONSTRAINT`).
- Expansion: post-containment path coordinates (`MOTION`).

The lineage is retained as `compression -> expansion -> optional next compression`. It is never flattened into a training table or promoted to a new family.

## Causal bridge

Each object event receives a bridge receipt containing its event timestamp, selected Master timestamp, age, join mode, Master generation, structural snapshot hash, and regional-basis hash. Selection is exact when possible, otherwise the latest snapshot no later than the event. A receipt fails closed after one source bar. Future snapshots are forbidden.

Point-state attachment and auction interval intersection are separate operators. RG3 co-collected the Master structural point-state but not the seven RG2 attempt/episode/transit relations under the same run identities. The interval table therefore says `NOT_EVALUABLE`; it does not counterfeit that absence as `NO_AUCTION_INTERSECTION`.

## Census result

The atlas contains 1,091 objects, 538 lineages, 2,735 bridge receipts, 3,236 typed nodes, and 10,106 typed edges. Of the bridge receipts, 2,690 are exact, five are causal as-of joins, and 40 are explicit structural-context nulls. All 510 completed expansions reach their recorded destination compression; 28 censored expansions retain `CENSORED_DESTINATION`.

Master context does not collapse the compression views into one organization. Compression correspondence remains weak within the supported regional/contact strata. Expansion is different: some structural strata materially sharpen existing view correspondence. The clearest exploratory example is expansion shape versus hybrid inside `REGION_1|OPEN_CORRIDOR`, where directional refinement is 0.530 versus 0.278 globally. The cell contains 48 eligible objects from 16 runs, all six instruments, and five windows. Seven instrument/window blocks meet the eight-object floor; their refinement p10/median/p90 is 0.224/0.332/0.681. This is distributed exploratory support, not confirmation.

Constraint-to-motion correspondence is weak globally. A `REGION_2|NODE_CONTACT` compression-summary to expansion-shape cell reaches 0.212 directional refinement with 39 observations from 10 runs, all six instruments, and five windows. Six supported instrument/window blocks have refinement p10/median/p90 of 0.139/0.168/0.229. It has distributed exploratory support, but it is not a confirmed process law.

The fully qualified lineage coordinate is highly sparse: 470 exact phenotypes appear among 538 lineages, and none meets the eight-object descriptive floor. That is evidence against treating the full tuple as a ready-made categorical grammar.

## Epistemic boundary

- `NULL_FAMILY` is not a family.
- Structural null, censored destination, not-applicable, data-gap, and unavailable auction authority are separate states.
- Fitted Gate 15 systems contain the exact feature order, winsor bounds, means, scales, centroids, labels, and assignment rule needed for frozen-system transport.
- No out-of-sample rejection radius is invented here.
- The twelve confirmation runs remain frozen and unopened.
- No trading or economic interpretation is present.

The canonical artifacts are in `artifacts/rg3-gate15-5-atlas-final-v5`.
