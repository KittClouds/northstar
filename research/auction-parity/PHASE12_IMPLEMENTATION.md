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

## Capture-ready, awaiting fresh replay evidence

### 12B.0

The optional MT5 oracle recorder and Rust verifier are implemented and the MQL5
controller compiles with 0 errors and 0 warnings. A capture has not been
manufactured from sealed RG2 because RG2 never recorded this input tape.

Fresh bounded replays must now produce the five oracle files before full
historical auction parity can be certified.

### 12B / 12C / 12D

These remain evidence-gated:

- 12B requires captured frame-by-frame auction inputs.
- 12C.2–12C.5 use captured normalized levels, nodes, provenance, and regional
  geometry.
- 12C.1 full producer parity additionally requires raw producer input history
  and producer warmup/state contracts; MT5 remains producer authority until
  those inputs are captured.
- 12D causal reconstruction compares Rust state at each captured frame against
  the captured feature snapshot. Merely reopening frozen feature rows is
  already proven by 12A, but is not mislabeled as independent reconstruction.

No live socket, TradeLocker, order execution, UI, or portfolio authority has
been added.
