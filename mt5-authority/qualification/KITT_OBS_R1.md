# KITT-OBS-R1 — Receipt Integrity and Lifecycle Qualification

This qualification hardens the standalone `KittDailySwingStateMachine` observer
without changing its daily-swing geometry or chart presentation semantics.

## Frozen authority split

| Surface | Authority |
|---|---|
| `frames.tsv`, `zones.tsv`, `events.tsv` | machine-readable observer output |
| `manifest.tsv`, `receipt.tsv` | run authority and integrity seal |
| chart objects | visualization only |
| indicator buffers / `iCustom` / `CopyBuffer` | `NONE` |

The chart can be useful and still has no machine-readable authority.  A consumer
must verify the receipt and manifest before admitting a run.

## Lifecycle contract

`receipt.tsv` is an append-only lifecycle ledger.  The receipt state is separate
from the run outcome:

```text
OPEN -> FINALIZING -> CLOSED
```

`CLOSED` is earned only after the logger has flushed and closed the three data
files, independently recounted rows, calculated byte counts and FNV-1a-64
digests, written `manifest.tsv`, and verified the manifest.  `COMPLETED` is a
separate outcome and requires the declared completion predicate to be true.

An ordinary deinitialization without the declared tester boundary is recorded as
`STOPPED_EARLY` when the terminal callback is delivered.  If the process exits
before finalization, the receipt remains `OPEN` and is an orphan; no tool repairs
it in place.  Reusing a run key is rejected before any existing file is opened
for writing.

The finalization contract hash binds the inclusive tester boundary and its
fallback behavior.  Experiment identity and physical run identity are separate
fields in every data and authority artifact.

## Verification and replay

The stdlib-only verifier is:

```text
python mt5-authority/tools/verify_kitt_observer_receipt.py self-test
python mt5-authority/tools/verify_kitt_observer_receipt.py verify <run-directory>
python mt5-authority/tools/verify_kitt_observer_receipt.py replay <run-a> <run-b>
```

The self-test covers valid non-empty and valid header-only empty runs, missing
artifacts, data digest changes, schema changes, post-close writes, receipt writes
after `CLOSED`, manifest count/hash tampering, and an orphaned `OPEN` receipt.

## Standard-MT5 evidence

The qualification used the portable **MetaTrader 5** tester only.  The
Trading.com terminal/editor was not opened or used.

The R4 and R5 runs used the same experiment, symbol (`EURUSD`), timeframe
(`M5`), history window, and tester boundary, while carrying distinct physical
run IDs.  Both sealed with `COMPLETED`, `CLOSED`, and
`manifest_verified=TRUE`:

```text
R4: frames=961, zones=155, events=155
R5: frames=961, zones=155, events=155
semantic replay root:
005450c1fd3505a1f7f3a05ea87f09afbd0322664ac853502250e4dd2c2c9f46
```

R11 repeated the sealed path with the latest duplicate-run and write-result
guards present:

```text
R11: frames=961, zones=155, events=155, manifest_verified=TRUE
manifest digest: FNV1A64_13364680161203777738
```

The no-boundary R10 run intentionally remained `OPEN` and was rejected by the
verifier.  A duplicate R4 launch left the sealed receipt byte-for-byte
unchanged.

## Qualification matrix

| Gate | Status | Evidence |
|---|---|---|
| receipt state/outcome split | QUALIFIED | typed lifecycle rows |
| experiment/run identities | QUALIFIED | explicit fields; R4/R5 distinct |
| two-phase finalization | QUALIFIED | manifest and independent reconciliation |
| no writes after close | QUALIFIED | guarded writer; verifier fault case |
| immutable terminal manifest | QUALIFIED | receipt-bound digest |
| valid empty artifacts | QUALIFIED | header-only self-test |
| count reconciliation | QUALIFIED | logger counters vs file recount |
| orphan handling | QUALIFIED | R6 remains OPEN and is rejected |
| duplicate run protection | QUALIFIED | R4 receipt unchanged on repeat |
| deterministic replay | QUALIFIED | R4/R5/R7/R9/R11 semantic root identical |
| buffer contract | QUALIFIED | indicator declares zero buffers; manifest says `NONE` |
| live runtime equivalence | GAP / NOT CLAIMED | portable tester only |

## Narrow seal

`KITT_OBSERVER_RECEIPT_LIFECYCLE_V1 = QUALIFIED` under the declared standard
portable-tester contract.  This qualifies receipt truthfulness and replay
identity; it does not establish scientific validity of the swing observer,
universal MT5 runtime equivalence, or any `CopyBuffer` consumer authority.
