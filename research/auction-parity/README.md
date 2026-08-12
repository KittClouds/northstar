# Northstar Auction Parity

An isolated, read-only bridge from the sealed MT5 research corpus to Northstar.

This workspace deliberately has no dependency edge into the Northstar
application crate. MT5 remains the behavioral oracle; this workspace verifies
and reconstructs its sealed ledgers before any later Northstar wiring.

## Proven scope

- Stable Rust representations for RG2 / dataset schema 7 semantics.
- Exact decimal and MT5-server-time preservation.
- Memory-mapped, zero-copy TSV traversal.
- SHA-256 verification of every sealed run payload.
- Canonical corpus hash reproduction.
- Relational validation of the seven auction datasets.
- Independent reconstruction of all Phase 10.5 view and target cardinalities.
- Independent auction grammar for all 16 golden scenarios in both directions.
- Exact reproduction of all 32 frozen MQL5 event-ID hashes.
- Exact reproduction of all 32 MQL5 ledger serialization hashes.
- Exact reproduction of all 32 MQL5 terminal accumulator hashes.
- Threshold, idempotence, mirror, and episode-gap boundary proofs.

No UI, TradeLocker, order, macro, ledger, or live-stream integration exists here.

## Build

Create the ignored workspace-local `target` junction once so Cargo output stays
on `D:`:

```powershell
$targetRoot = 'D:\northstar-auction-parity-target'
New-Item -ItemType Directory -Force -Path $targetRoot | Out-Null
New-Item -ItemType Junction -Path 'target' -Target $targetRoot | Out-Null
```

Then point `$researchRoot` at the local MT5 research workspace:

```powershell
$researchRoot = 'X:\path\to\mt5-research'
cargo test --workspace
cargo run -p northstar-parity-cli -- verify `
  --corpus "$researchRoot\furnace\corpus" `
  --seal "$researchRoot\phase10\seal\corpus_seal.json"

cargo run -p northstar-parity-cli -- verify-interface `
  --workspace "$researchRoot"

cargo run -p northstar-parity-cli -- verify-golden
```

## Deliberate stop line

The workspace is not wired into Northstar. The cross-language golden ledger is
now exact; verified corpus rows still need to be packed into a stable mmap
artifact and reopened with row/key/view parity. Only that artifact will become
a candidate dependency for a later Northstar adapter crate.
