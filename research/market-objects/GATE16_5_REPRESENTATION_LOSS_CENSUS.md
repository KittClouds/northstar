# Gate 16.5: representation-loss census

Gate 16.5 observes which distinctions each frozen Gate 16 comparison contract
cannot express. It does not select, repair, rank, or interpret a representation.

## Authority

The admitted population is exactly the sealed RG3 corpus:

```text
runs                         42
objects                   1,091
compression objects         553
expansion objects           538
same-kind object pairs   297,081
```

Admission fails closed unless both the 42-run and 1,091-object counts match the
Gate 15 seal. Gate 15, Gate 15.5, Gate 16, and the twelve unopened confirmation
windows are read-only ancestry.

For every frozen contract `r`, the observed relation is:

```text
R_r(i, j) := Compare_r(i, j) returns finite exact 0.0
```

`R_r` means only that one watcher cannot distinguish the pair. The code uses
equivalence-class terminology only where a fixed comparison domain passes
exhaustive reflexivity, symmetry, and transitivity checks. Shared-prefix
contracts remain pair-dependent indistinguishability graphs.

## Exhaustive census

All 297,081 same-kind pairs were evaluated under all 20 frozen
representation/distance/mode contracts:

```text
contract-qualified evaluations       5,941,620
exact-zero relations                   498,115
unique object pairs affected           161,957
  compression                           31,811
  expansion                            130,146
epsilon-near relations                       0
typed not-comparable relations         276,174
```

The 498,115 value is not a unique-pair count. One object pair can be
indistinguishable under several contracts.

Summary and trajectory contracts produced no exact or machine-derived
epsilon-near collisions in this corpus. Event and graph contracts did. This is
a corpus observation, not a proof of global injectivity.

## What the frozen watchers erased

Every event/graph exact-zero pair had different raw-history and continuous
trajectory hashes. The collisions are therefore material representation loss,
not duplicate raw objects.

Event sequence:

```text
                                raw       canonical
compression exact zeros        753            1,521
expansion exact zeros         2,918            4,767
```

The raw zero set is contained in the canonical zero set. The additional
canonical zeros coincide with direction/event-orientation differences erased
by canonicalization. Some raw event collisions also differ in total object
duration, showing that the ordered event receipt does not encode every aspect
of the object's observed lifetime.

Graph V1 is much more lossy:

```text
                                       raw       canonical
compression typed-multiset          15,955          31,811
compression WL2                     15,945          31,785
expansion typed-multiset            66,184         130,146
expansion WL2                       66,184         130,146
```

For canonical expansion graphs, both graph distances induce three exhaustive
classes of sizes 510, 27, and 1. Raw expansion graphs induce five classes of
sizes 290, 220, 18, 9, and 1. This records severe collapse in Graph V1. It does
not establish that the graph view contributes useful information; RG3 did not
emit authoritative branch/merge topology, so that question remains
`NOT_EVALUABLE`.

Typed-multiset and WL2 graph zero kernels are identical for expansion and
nearly identical for compression. Event-zero pairs are contained within the
canonical graph-zero sets, but graph-only disagreement is not evidence of
incremental information.

## Canonicalization and local geometry

Canonicalization changes neighborhood membership substantially while retaining
the order of many surviving neighbors:

```text
                  mean top-10 Jaccard   mean shared rank correlation
compression                 0.3814                           0.9800
expansion                   0.4271                           0.8583
```

This separates neighborhood turnover from exact class merging. Neither effect
is labeled useful or harmful.

## Retrospective Gate 15 diagnostic

Nearest-neighbor family agreement remains descriptive only. Depending on the
frozen view and object kind, the non-NULL same-family nearest rate ranges from
0.853 to 0.957 across all six instruments and all 42 runs. `NULL` is excluded
from family rates and is never promoted into a family.

## Point-state SPACE pressure test

The canonical complete-trajectory distance is slightly smaller within the same
available Master structural stratum than across different strata:

```text
same-stratum pooled median       1.272146
different-stratum pooled median  1.298012
pooled difference                0.025866
anchor median delta              0.005597
block bootstrap p10 / p50 / p90  0.002665 / 0.005597 / 0.008806
```

Support is broad but not uniform: 25/42 run blocks, 5/6 instrument blocks, and
6/7 calendar-window blocks have positive anchor deltas. US30 is negative at the
instrument aggregate. The finding is therefore a small, distributed
point-state association—not a universal law, causal statement, or mechanism.

The bridge has 1,049 exact joins, 5 causal as-of joins, and 37 unavailable
contexts. Auction interval overlap remains `NOT_EVALUABLE` because RG2 and RG3
intervals are not co-identified.

## Determinism and storage

One-thread and four-thread release executions generated 16/16 byte-identical
artifacts. The canonical logical census hash is independent of compression:

```text
logical census SHA-256
f88bbdffa5c6579617b1ade8ca6c1361da0aba3cc681a6a04e89c5be0c9c5cda

physical artifact-set SHA-256
8d8ab934fa9cf4a352a0bb51ee6fd8455ae7a55c4a4110408147727295abe731
```

Every contract summary contains a BLAKE3 stream over all evaluations, including
ordinary nonzero distances that are not retained as rows. Exact-zero,
epsilon-near, and not-comparable rows are retained in deterministic Zstandard
streams. The 1,091 raw-authority descriptions are normalized once, and compact
pair receipts carry two-bit axis states.

The rehydration verifier decompresses both ledgers and proves that every pair
resolves to admitted authority objects and every exact-zero pair has exactly
one raw-difference receipt.

## Epistemic statement

Gate 16.5 maps the blindness of each frozen watcher. It establishes no best
representation, no universal geometry, no market taxonomy, no prediction, no
mechanism, and no economic meaning.
