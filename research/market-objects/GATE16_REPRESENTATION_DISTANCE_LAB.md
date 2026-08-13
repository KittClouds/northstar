# Gate 16 — Representation and Distance Laboratory

Gate 16 is sealed as a derived, intrinsic, partial-geometry laboratory over the unchanged RG3 market-object corpus.

## Authority and boundaries

- Observational authority: 42 RG3 runs, 1,091 objects, canonical corpus `68ed0ad0…d8684b8`.
- Complete-life geometry admits 1,048 completed objects. The 43 censored objects retain `CENSORED_SUFFIX` and are never padded or stretched into terminal objects.
- Master Controller point-state context is used only in retrospective diagnostics. It never enters an intrinsic representation.
- Auction interval overlap remains `NOT_EVALUABLE` because RG2 auction ledgers were not co-collected under RG3 run identities.
- Gate 15 and 15.5 assignments are diagnostic probes only. No representation or distance was tuned against them.
- The twelve confirmation runs remain frozen and unopened.

## Frozen geometries

Gate 16 defines raw and canonical variants for four representation classes:

1. Robust fixed-width summaries.
2. Censor-safe 21-point continuous/state trajectories.
3. Typed, duration-aware event sequences.
4. Typed intrinsic process graphs with two Weisfeiler–Lehman refinement rounds.

The public comparison is partial. It returns either a finite distance plus support receipt, or `NOT_COMPARABLE` with a typed reason. A per-object distance vector is emitted against a deterministic within-kind anchor; each coordinate remains independent and can be unavailable.

Reflection and canonicalization are distinct. Reflection is tested as an involution; conditional canonicalization is tested as idempotent. Shared-prefix trajectory comparison first selects the common causal horizon, then resamples both objects on that same interval.

## Proof

- Rust tests: 19 passed, one explicit 100k mmap lane ignored in unit tests because the dedicated performance executable runs it.
- Clippy: clean with warnings denied.
- One-thread/four-thread rebuild: all 18 artifact files byte-identical.
- Canonical Gate 16 lab hash: `49937da6b2f3abac1f6426a15b40525ec614b74052446437bbb484c93aefe833`.
- All 24 frozen distance probes pass.
- 100,000-object computational fixture: PASS; scientific evidence is explicitly false.

## What fell out

- Raw and canonical trajectory views are related but not interchangeable. Top-10 overlap is 0.381 for compression and 0.427 for expansion; rank correlation among shared neighbors is 0.980 and 0.858 respectively.
- Event and graph neighborhoods differ strongly. Top-10 overlap is 0.185 for compression and 0.064 for expansion. This does not yet prove incremental graph information: graph V1 cannot contain branch/merge topology that RG3 never emitted.
- Summary and trajectory neighbor scans have no exact collisions inside the emitted top-10 audit scope. Event sequence has 2,970 and graph has 9,520 materially distinct raw histories at zero observed distance. These are representation-loss findings, not all-pairs collision counts.
- Gate 15 local-family agreement with matching Gate 16 geometry is 0.904580 across six instruments. This is retrospective compatibility, not confirmation or optimization.
- Canonical trajectory nearest-neighbor median distance is 0.3371 within the same available Master structural stratum versus 0.3722 otherwise. This is a descriptive point-state result; blocked uncertainty and auction-interval conditioning are not claimed.

## Performance fixture

The 100,000-object fixture is replicated computational data only. It measures packed construction, one-query scan, deterministic top-10 selection, mmap traversal, and scalar/SIMD plus thread-count parity. It does not claim a 100k × 100k matrix or add scientific population evidence.

## Epistemic statement

Gate 16 establishes deterministic, censor-aware, direction-explicit partial geometries. It characterizes what each representation preserves, loses, and regards as locally similar. It does not establish a universal geometry, discover a market family, predict an outcome, or assign economic meaning.
