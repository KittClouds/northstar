# Phase 12 implementation state

## Proven now

### 12A — packed auction research artifact

Contract: `RG2_AUCTION_RESEARCH_PACKED_V1`.

The artifact deliberately contains the seven auction research relations, not
the three multi-gigabyte structural streams. It embeds the sealed RG2 corpus
SHA as provenance and has independent semantic and physical hashes.

Current proof:

- source corpus SHA-256:
  `9cbad8545db23cba7ccea2bd062aeba700597deb4be19cecbb74647f65ef8413`
- packed semantic SHA-256:
  `2bcbfccec8261dcb9894baae7623b3ec3452612af99d28f201c0f2a5d1a78dec`
- artifact SHA-256:
  `3b7ddff33bffa55814e8493d5727a9bbb9ca62bb4cb26e71251812859b5d6b25`
- artifact size: 46,776,680 bytes

The mmap format has an explicit little-endian header, section directory,
64-byte-aligned data, row-offset indexes, section hashes, schema provenance,
and bounds-checked reopening.

### 12E — read-only research adapter

The adapter queries packed bytes directly. It verifies exact ID-set hashes for
eligible, positive-observed, censored, and analyzable observations for all five
Phase 10.5 targets. Counts alone are not accepted.

The corrected rejection universe is:

```text
eligible      5,737
censored        686
analyzable    5,051
positive      3,504
```

### 12F.0/F.1 — model freeze and inference parity

Four Phase 11 candidates are frozen with complete preprocessing, exact f64 bit
patterns, cohort identity, explicit `calibration = NONE`, and development-only
reference vectors. Rust reproduces all scores and probabilities with zero
observed error:

```text
reclaim             ridge           1,547 x 99
initial acceptance  ridge           5,051 x 129
initial rejection   ridge           5,051 x 129
retest hold          boosted stumps  1,251 x 94
```

The future holdout remains untouched.

## Fresh input-tape determinism proven

### 12B.0

The optional MT5 oracle recorder and Rust verifier are implemented, and the
deployed MQL5 controller compiles with 0 errors and 0 warnings. Two independent
headless US30 M5 bounded replays each produced 1,056 frames over the same
research window. Their canonical capture SHA-256 is identical:

`839c07b98b77d4db707493b97ce04ad677878a3f79c54a7834d1de1658bb0ce7`.

All five cumulative semantic frame prefixes match through sequence 1,056.
Their unique invocation IDs and physical `frames.tsv` hashes differ, proving
that invocation provenance remains distinct without contaminating market
semantics. The machine-readable evidence is in
`proof/mt5_fresh_oracle_receipt_run5.json`,
`proof/mt5_fresh_oracle_receipt_run6.json`, and
`proof/mt5_fresh_oracle_determinism.json`.

The canonical capture hash excludes `invocation_id` by contract. Physical
per-file hashes retain it, so repeated executions can have distinct provenance
while identical market semantics remain directly comparable.

### 12B / 12C.2-C.5 / 12D

These reconstruction cuts now pass on the five-session, transit-bearing US30
M5 oracle. Rust independently reproduces normalization, compatibility-gated
DBSCAN, deterministic nodes and provenance, stable node lifecycle, regional
geometry, causal frozen features, and the complete auction ledger.

The certificate covers 5,155 frames, 487,662 normalized levels, 1,554 DBSCAN
rebuilds, 40,459 stable-node rows, 283,419 provenance rows, 643 events, 145
attempts, 52 episodes, and seven transits. Event order, relational rows,
censoring, terminal event sequence, and terminal hash all agree exactly under
their serialization contracts.

### 12C.1 remains open

Full producer parity still requires raw producer input history plus exact
producer warmup and state contracts. The present tape starts at raw producer
outputs, so MT5 remains producer authority. See `PHASE12_BD_CERTIFICATE.md`.

No live socket, TradeLocker, order execution, UI, or portfolio authority has
been added.
