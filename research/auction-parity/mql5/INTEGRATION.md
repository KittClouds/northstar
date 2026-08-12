# MT5 parity-oracle integration

`MasterParityOracle.mqh` is a disabled-by-default recorder for fresh parity
qualification runs. It does not retrofit new evidence into sealed RG2.

The deployed controller integration has these explicit seams:

1. Include `MasterParityOracle.mqh` from `MasterController.mqh`.
2. Add `bool enable_parity_oracle` to `MST_ControllerConfig`.
3. Default it to `false`; do not include it in `ConfigText()` because recording
   does not alter market semantics.
4. Own one `CMstParityOracle` beside the auction engine.
5. Initialize it with the deterministic `run_key` and unique `invocation_id`.
6. Call `Write(...)` after causal context is built and before
   `m_auction.Observe(...)`.
7. Flush during deterministic finalization.
8. Close during controller deinitialization.

The indicator exposes:

```mql5
input bool InpEnableParityOracle = false;
```

When enabled, files are written under the terminal common-files directory:

```text
MasterStructureParity/<run_key>/
  frames.tsv
  levels.tsv
  nodes.tsv
  sources.tsv
  features.tsv
```

These five files collectively implement:

```text
MST_AUCTION_REPLAY_INPUT_V1
MST_NORMALIZED_STRUCTURE_INPUT_V1
MST_STRUCTURAL_NODE_INPUT_V1
MST_STRUCTURAL_PROVENANCE_INPUT_V1
MST_CAUSAL_FEATURE_INPUT_V1
```

Verify a completed capture with:

```powershell
cargo run --release -p northstar-parity-cli -- verify-oracle `
  --oracle '<common-files>/MasterStructureParity/<run_key>' `
  --receipt 'proof/oracle_<run_key>.json'
```

The verifier rejects missing frames, changing identities, noncontiguous frame
sequences, orphan child records, and missing per-frame causal feature records.
It emits cumulative prefix hashes every 256 frames for first-divergence search.
