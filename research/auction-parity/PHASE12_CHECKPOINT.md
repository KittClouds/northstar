# Phase 12 checkpoint

Status: `SERVING LANE PASS / FRESH INPUT TAPE DETERMINISM PASS / INDEPENDENT RECONSTRUCTION OPEN`

## Isolation and authority

- Standalone Cargo workspace: `research/auction-parity`.
- Cargo output is linked to `D:\northstar-auction-parity-target`.
- No dependency edge enters the Northstar application crate.
- No UI, TradeLocker, order, portfolio, ledger, macro, or live-stream authority
  changed.
- MT5 remains producer and behavioral authority until a fresh bounded replay is
  independently reconstructed and certified.

## Completed serving lane

### Packed RG2 artifact

- Contract: `RG2_AUCTION_RESEARCH_PACKED_V1`.
- Source corpus: 20 admitted runs, 320 sealed files, 3,915,010,108 bytes.
- Source corpus SHA-256:
  `9cbad8545db23cba7ccea2bd062aeba700597deb4be19cecbb74647f65ef8413`.
- Relations: 38,957 events; 7,764 attempts; 1,462 episodes; 49,281 context
  rows; 7,764 feature rows; 628 transits.
- Packed size: 46,776,680 bytes.
- Packed semantic SHA-256:
  `2bcbfccec8261dcb9894baae7623b3ec3452612af99d28f201c0f2a5d1a78dec`.
- Physical artifact SHA-256:
  `3b7ddff33bffa55814e8493d5727a9bbb9ca62bb4cb26e71251812859b5d6b25`.
- Explicit little-endian header, 64-byte alignment, row-offset indexes,
  bounds checks, section hashes, metadata hash, and canonical semantic hash are
  verified when the artifact opens.

### Deterministic repack proof

A second full pack from the sealed source produced the same byte length and
physical SHA-256. Measured wall time was 5,646.897 ms.

The Python model/target freeze was also repeated. Its receipt remained byte
identical with SHA-256
`72f1cb2753d2302c94b9d77bcabeb27cc309f00520a3a87684a098c3ce05b271`.

### Read-only research adapter

The adapter traverses the mmap directly and reproduces the exact eligible,
observed-positive, censored, and analyzable ID sets for all five Phase 10.5
targets. It does not accept cardinality-only parity.

### Frozen model artifacts

Four Phase 11 candidates carry complete preprocessing state, exact f64 bit
patterns, ordered cohort identity, and explicit `calibration = NONE`. Rust
reproduces every development-cohort score and probability with zero observed
absolute error:

```text
reclaim             ridge           1,547 x 99
initial acceptance  ridge           5,051 x 129
initial rejection   ridge           5,051 x 129
retest hold          boosted stumps  1,251 x 94
```

The reserved holdout was not accessed.

### Warm release measurements

Five process-level runs on this workstation:

```text
operation                         min ms   median ms   max ms
verify packed artifact            96.809    103.420   178.877
verify packed target interface   125.542    127.223   137.503
verify four frozen models         35.121     39.605    43.564
```

These are diagnostic measurements, not a cross-machine performance promise.

## Fresh replay lane

- The optional MT5 parity oracle is deployed disabled by default.
- The current D0 terminal controller compiles with 0 errors and 0 warnings.
- It captures the exact pre-auction frame boundary, normalized levels, nodes,
  source provenance, and regional/frozen features.
- Rust validates the five-file capture, feature/frame bijection, referential
  sequence, hashes, and deterministic prefixes. Divergence receipts identify
  the first differing prefix block without claiming an unmeasured exact row.
- The bounded US30 M5 open-prices qualification window produced 1,056 frames,
  101,070 normalized levels, 9,714 node rows, 59,346 provenance rows, and
  1,056 causal feature snapshots.
- Two fresh headless invocations have distinct invocation IDs and physical
  `frames.tsv` hashes but the same canonical semantic capture SHA-256:
  `839c07b98b77d4db707493b97ce04ad677878a3f79c54a7834d1de1658bb0ce7`.
- All five 256-frame semantic prefixes match through sequence 1,056.

This certifies capture contract integrity and repeat determinism for the bounded
qualification window. It does not certify independent Rust reconstruction of
historical auction state, topology, or features. Those remain the next 12B-D
cuts. Full C1 producer parity additionally requires raw producer inputs and
warmup/state contracts in the capture.

## Verification commands

Run from `research/auction-parity`, not the repository root:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p northstar-parity-cli
```

Machine-readable receipts are in `proof/`.
