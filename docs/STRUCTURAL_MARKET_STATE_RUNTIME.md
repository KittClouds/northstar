# Northstar structural market state runtime

## Status

This document records the executable Phase II-B warm surface. It implements
the deterministic contracts and price-only map substrate that was proven before
venue ingestion. A HeroFX TradeLocker LIVE desktop account is now verified, but
its REST feed is not yet admitted to this structural runtime; the MT5 bridge
remains deferred.

Implemented now:

- compact structural IDs and explicit reference/venue price spaces;
- point levels and price bands with causal provenance;
- deterministic structural mutations and an atomic single-writer Level Book;
- current and previous reference-session highs/lows;
- five retained session extreme body-to-wick bands;
- versioned opening-range high, low, and midpoint;
- versioned eight-level rolling range projection;
- compatibility-aware structural-node merging;
- stable node identity plus merge, split, preservation, and expiry genealogy;
- price-ordered corridors;
- fast current-price location with ATR-normalized distances;
- immutable chart lines/bands projection with no market mathematics;
- versioned BLAKE3 mutation and terminal-snapshot fingerprints;
- explicit unavailable volume, adaptive-value, and volume-profile state.

Not implemented in this cut:

- trading-day and trading-week levels, because the current canonical calendar
  has session identity but not explicit trading-day/week identity;
- TPO and volume profiles;
- VolKitt C1-C5 and COG;
- VWAP;
- liquidity shelf discovery;
- structural dynamics;
- Phase II-C interaction grammar;
- strategy, model, Nautilus, or execution behavior.

## Ownership

```text
canonical M4 bars + versioned UTC calendar
                  |
                  v
          StructuralEngine
                  |
          producer mutations
                  |
                  v
             LevelBook
                  |
                  v
        NodeBook + corridors
                  |
                  +----> MarketLocation fast path
                  |
                  +----> ChartStructureSnapshot
```

The implementation lives in `src/market_state`. It is deliberately outside
`chart`, `operating_chart`, and GPUI modules. Deleting every renderer cannot
change structural output.

## Price-space invariant

Every structural object declares one of:

```text
ReferenceIndex
VenueExecutable
```

The node builder filters by instrument and price space before grouping. A
reference-index level can never merge with a TradeLocker venue level. Future
basis projection must create a distinct venue-space derivative rather than
rewriting the reference object.

## Identity and replay

`LevelId` packs producer, instrument, and producer-local first-seen sequence.
`StructuralNodeId` likewise scopes its local identity to the instrument. The
same ordered canonical bar stream therefore produces globally distinct,
repeatable IDs without random state, wall-clock input, hash-map iteration
order, or renderer participation.

Ordinary movement emits `Updated` with the same ID. Completion emits `Frozen`.
Replacement emits `Superseded`, and retirement emits `Expired`. The Level Book
never physically removes objects, so historical mutation replay retains their
identity and state transitions.

Each non-empty mutation batch advances exactly one derivation generation. The
book validates a complete generation before changing storage. An independent
book replaying emitted mutations must reconstruct the same terminal active
snapshot.

`fingerprint_mutations` and `fingerprint_snapshot` use versioned, field-ordered
BLAKE3 encodings. They are equality receipts, not serialization formats.

## Implemented producers

### Reference-session structure

The producer consumes the versioned materialized UTC `SessionCalendar`; it has
no host-time-zone or hard-coded New York assumptions.

During a session:

- current high changes only on a new high;
- current low changes only on a new low;
- the upper extreme band is `[max(open, close), high]` for the high bar;
- the lower extreme band is `[low, min(open, close)]` for the low bar.

At the next session, current high/low retire after publishing frozen previous
session high/low objects. Extreme bands freeze and five completed sessions are
retained.

### Opening range

`OpeningRangeSpec` owns the duration. High, low, and midpoint share an exclusive
group, preventing accidental sibling collapse. Geometry freezes at the earlier
of the configured endpoint and reference-session close.

There is no ORB interpretation.

### Rolling range projection

`RangeProjectionSpec` owns lookback and ordered integer parts-per-million
fractions. The default fractions are:

```text
0, 0.213, 0.333, 0.500, 0.666, 0.750, 0.900, 1.000
```

Projection uses fixed-point integer arithmetic. IDs persist while rolling
geometry moves. All siblings share an exclusive group and cannot merge with one
another even when the range compresses.

## Node compatibility and genealogy

The merge threshold is:

```text
max(ATR fraction, noise multiple, tick multiple)
```

All constants live in `NodeMergeSpec`. Candidate traversal and tie-breaking
are price/ID ordered. A candidate must be role-compatible with every existing
member. Lower and upper roles never merge directly, and objects sharing a
non-zero exclusive group never merge.

Node tracking uses contributor overlap, price overlap, center distance, and
stable ID tie-breaking. The published genealogy distinguishes:

```text
Created
Preserved
Merged
Split
Expired
```

No strength score or market interpretation exists in this layer.

## Fast and structural paths

`StructuralEngine::on_bar` is the structural path. It updates ATR, producers,
the Level Book, node graph, corridors, and chart projection only when geometry
changes.

`StructuralEngine::locate_price` is the fast path. It performs a location lookup
against the immutable graph and reuses the existing level and graph `Arc`
allocations. It performs no producer calculation, clustering, profile rebuild,
formatting, or GPUI work.

The rolling projection computes prices directly from its fixed fraction slice;
it does not allocate a temporary price vector per bar. Level Book validation
reuses its hash-map capacity between generations.

## Volume boundary

This cut always publishes:

```text
volume_quality = Unavailable
adaptive_value_available = false
volume_profile_available = false
```

That is finished operational truth, not a placeholder. Future VolKitt, VWAP,
and volume profile producers cannot activate until a canonical source carries
an admitted `ExchangeVolume`, `VenueVolume`, or explicitly labeled
`TickVolumeProxy` quality.

Missing volume is never converted to zero. Reference index observations are
never treated as traded volume.

## Revision boundary

An in-place `CanonicalBar` revision is rejected with
`RevisionRequiresRederivation`. The producer state is not silently rewound or
historical geometry overwritten. The next cut must drive the corrected prefix
through a new explicit structural derivation generation and publish
supersession mutations.

## Verified warm-surface tests

The focused suite proves:

- atomic Level Book generations and frozen-object immutability;
- identical IDs, mutations, graphs, locations, and fingerprints on two replays;
- terminal reconstruction from mutation batches alone;
- opening-range freeze;
- session rollover and previous-session truth;
- exclusive-sibling non-merge;
- compatible contributor merge and stable node identity;
- price-space isolation;
- fast-path immutable allocation reuse;
- fail-closed volume state;
- explicit revision rejection;
- wrong-instrument rejection before mutation.

## Next honest activation steps

1. Add canonical trading-day and trading-week identities to the calendar before
   implementing previous-day/week producers.
2. Add a replay-owned structural derivation journal and corrected-generation
   supersession path.
3. Feed qualified historical OHLC through the exact engine to tune per-index
   ATR/noise/tick merge specifications.
4. Implement TPO from price history.
5. Admit volume-bearing producers only after source quality is known.
6. Publish `ChartStructureSnapshot` through the operating snapshot plane and
   render its already-computed lines/bands in GPUI.

No strategy, Phase II-C grammar, Markov/HMM/HSMM model, Nautilus dependency, or
order path belongs in this runtime.
