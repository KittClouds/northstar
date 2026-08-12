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
- Versioned 46.8 MB mmap artifact for the seven auction research relations.
- Exact packed Phase 10.5 target ID-set parity, not count-only parity.
- Complete Phase 11 candidate artifacts and exact Rust inference parity.
- Disabled-by-default MT5 parity-oracle capture and fail-closed Rust verifier.
- Fresh bounded replay input-tape determinism through 1,056 frames.

No UI, TradeLocker, order, macro, execution, or live-stream integration exists here.

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

cargo run --release -p northstar-parity-cli -- pack `
  --corpus "$researchRoot\furnace\corpus" `
  --seal "$researchRoot\phase10\seal\corpus_seal.json" `
  --output "artifacts\rg2-auction-research-v1.mmap" `
  --receipt "proof\rg2_packed_corpus.json"

cargo run --release -p northstar-parity-cli -- verify-packed `
  --artifact "artifacts\rg2-auction-research-v1.mmap"

cargo run --release -p northstar-parity-cli -- verify-packed-interface `
  --artifact "artifacts\rg2-auction-research-v1.mmap" `
  --registry "artifacts\phase12-freeze\target_id_registry.json" `
  --freeze-receipt "artifacts\phase12-freeze\freeze_receipt.json" `
  --receipt "proof\packed_interface_parity.json"

cargo run --release -p northstar-parity-cli -- verify-models `
  --registry "artifacts\phase12-freeze\frozen_model_registry.json" `
  --freeze-receipt "artifacts\phase12-freeze\freeze_receipt.json" `
  --receipt "proof\model_inference_parity.json"
```

## Deliberate authority line

The isolated read-only artifact, adapter, and inference crates are ready to be
wired later. Northstar application authority is unchanged. Full independent
historical reconstruction now proceeds from the certified fresh oracle tape; see
`PHASE12_IMPLEMENTATION.md`.
