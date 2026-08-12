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
input datetime InpTesterFinalizeAt = 0;
input string InpCanonicalInstrument = "";
input string InpDataSourceId = "";
input string InpDataFingerprint = "";
input datetime InpResearchWindowStart = 0;
input datetime InpResearchWindowEnd = 0;
```

## Research controller versus isolated chart indicator

The research controller and chart indicator are intentionally independent:

- `MasterStructureController.mq5` is the research/testing machine. It retains
  receipt, parity-oracle, deterministic cutoff, and research-identity inputs.
- `MasterStructureChart.mq5` is the complete chart-facing structural indicator.
  It imports its private engine from `MasterStructureChart/` and that engine
  imports only `02marketmain.mqh`, `02dayswings.mqh`, and `02wayne.mqh`.
- The private chart engine is intentionally only seven modules:
  `ChartController.mqh`, `ChartInteraction.mqh`, `MasterTypes.mqh`,
  `MasterAdapters.mqh`, `MasterClusterer.mqh`, `MasterNodeTracker.mqh`, and
  `MasterRegionalModel.mqh`. It contains no auction engine, logger, transit,
  receipt, manifest, censoring, or research-finalization implementation.
- The chart indicator does not import the research `MasterStructure/` tree or
  any `01*` producer. The two programs can therefore evolve and run without
  sharing MQL5 source dependencies or chart-object ownership.

The chart indicator owns only `MSC_CHART_<symbol>_<chart-period>_*` chart
objects and removes that complete owner namespace on deinitialization or
timeframe reload. The first live render may wait while MT5 synchronizes the
producer timeframes. Optional indicator-buffer publication is guarded until
MT5 has sized those buffers; rendering itself does not depend on them.

The chart controller retains only compact, transient interaction state for
labels (`IDLE`, `APPROACH`, `CONTACT`, `PENETRATION`, `BROKEN`, `ACCEPTED`, and
`RETEST`). These are visual annotations, not persisted research episodes.
An offline market can leave a valid chart static; live qualification therefore
uses an actively ticking symbol and treats feed availability separately from
controller health.

Install the isolated chart bundle as follows:

```text
MQL5/Indicators/BuiltMasterStructure/MasterStructureChart.mq5
MQL5/Include/MasterStructureChart/*.mqh
MQL5/Include/02marketmain.mqh
MQL5/Include/02dayswings.mqh
MQL5/Include/02wayne.mqh
```

`MasterStructureMap.mq5` is retained only as historical parity-work evidence.
It is not the live chart indicator and must not be used as its wrapper.

Production qualification presets must supply all research identity fields and
an explicit deterministic finalization time. The controller does not emit
pre-window frames and finalizes once at the configured cutoff.

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
`invocation_id` is excluded from semantic capture and prefix hashes but remains
in the physical `frames.tsv` hash.

Compare repeated invocations with:

```powershell
cargo run --release -p northstar-parity-cli -- compare-oracles `
  --left-receipt 'proof/oracle_run_a.json' `
  --right-receipt 'proof/oracle_run_b.json' `
  --receipt 'proof/oracle_determinism.json'
```

The feature schema uses `regional_cog_velocity_atr` for the regional field
velocity, preserving the separate auction-context `cog_velocity_atr` column.
