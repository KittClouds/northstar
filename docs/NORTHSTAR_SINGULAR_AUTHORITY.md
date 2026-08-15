# Northstar Singular Authority

`KittClouds/northstar` on `codex/northstar-singular-authority-v1` is the
repository authority for the complete Northstar engineering surface.

The migration begins at the latest `codex/phase12-auction-parity` Northstar
commit and preserves the sealed OBS-OPEN-04A study in a dedicated first
descendant commit. That fossil is tagged independently so later integration
cannot blur its scientific identity.

## Authority topology

```text
MT5 source and research authority  ----\
TradeLocker observation contracts -----+--> Northstar canonical contracts
RunRaw / FeatureTape / EpisodeTape -----/              |
                                                       +--> operating runtime
                                                       +--> deterministic RL world
                                                       +--> campaign control plane
                                                       +--> Phoenix publication adapter
                                                       +--> typed receipts and seals
```

Northstar owns operating mode, lineage, safety state, command authorization,
execution lifecycle, reconciliation, and audit receipts. MT5 remains a source
and experimental authority. TradeLocker remains a broker observation and future
execution venue authority. Phoenix remains a consumer/publication host. The
learning runtime is a consumer of typed Northstar worlds and may not redefine
financial or broker semantics.

## Repository surfaces

| Path | Role |
| --- | --- |
| `src/`, `tests/`, `docs/` | Northstar operating runtime and contracts |
| `research/auction-parity/` | Canonical Rust/MT5 auction parity surface |
| `research/obs-open-01/sentinel-memory-metrology/` | Sealed OBS-OPEN-04A fossil |
| `mt5-authority/studies/obs-open-01/sentinel-behavioral-qualification-compound/` | Sealed G0-G8 descendant authority chain |
| `mt5-authority/` | Imported MT5, MQL5, Python, study, seal, and receipt surface |
| `northstar-rl/` | Rust world, PyO3 bridge, Python Gymnasium shell, SB3 compatibility, and campaign control plane |
| `crates/gpui-animated-gradient-text/` | Vendored UI dependency formerly resolved through a Phoenix-relative path |
| `docs/agents/` | Standalone Northstar agent constitutions |
| `authority/migration/` | Source bindings and explicit migration exclusions |

The older `northstar-auction-parity` copy visible from the Phoenix backup is not
installed as a second authority. The selected Northstar branch already contains
the developed 146-file authority surface; the Phoenix copy is a 28-file older
fragment with 11 differing files. Its source commit is preserved in the
migration receipt.

## Learning firewall

The imported learning surface remains inert:

- FORGE-RL-00 is a deterministic environment surface;
- FORGE-RL-01 is a read-oriented joint wind tunnel;
- FORGE-RL-02 is a sealed pre-execution campaign control plane;
- Stable-Baselines3 is a compatibility dependency and future executor only;
- no PPO, SAC, `model.learn()`, learner step, policy weight, or promotion claim
  is introduced by this migration.

Large datasets remain external. Their relative paths, byte counts, and SHA-256
identities are recorded in
`authority/migration/EXCLUDED_LARGE_DATASETS.tsv`. All other tracked and
explicitly untracked, non-ignored source paths were materialized; deeply nested
Windows paths were verified through the long-path API.

## Fossil boundary

- scientific root: `48102bd529abf01591842f85676b86eb66b0600dc8d702bd0dbb3d6b729b4c41`
- Git commit: `aa12e457667a496188852ae7b7ed8d37990b15b7`
- annotated tag: `northstar-obs-open-04a-48102bd529ab`

The tag points to the commit containing only the already-sealed study. All
authority consolidation is descendant work.

## G0-G8 descendant freeze

The outcome-blind behavioral-qualification compound is frozen through G8 and
is at a hard pause. Its sealed roots are:

- G0: `e317cabef33114edf47c50f4628a210ae8f1bd0b59feb436dc37e7df9cfe38bf`
- G1: `65cbe177fd5dc510ab9dc19e203a912a107ab8ac740b55a64926a1164edceafd`
- G2: `bdb26248c750099e2a2aa64f09e75fc66a2c98893892e90e12c47ae9908c0fe6`
- G3: `c936eac1c37cbc70aa9d42bffdea096cdc1afb9269ad73c26f2ddf535eb54bef`
- G4: `37ed98b4ebe447ef3c2152e550c99652d0157aea2c77b8a886379d1ba9e08e15`
- G5: `808f4089ada22e7736460a90efd71b1dec0726dece460ca4dc1df8cbccac554c`
- G6: `cf282d195f90e1ee970b7c5d57329c901848d7894f4ed8c5945755d3935633dd`
- G7: `069a5294c37a6fbede436af2f57700964ce1e752643aeaa26b06de6581183bd5`
- G8: `556cba86bf75a76d67c411db5a62229cb35ec84c92bd0bfbb6fd086971496ece`

Cargo build trees and empirical datasets remain local. No G9 execution or
population opening is authorized by this repository checkpoint.
