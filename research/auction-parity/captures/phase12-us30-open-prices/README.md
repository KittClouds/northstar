# Phase 12 US30 qualification replay

This preset records one deterministic, bounded MT5 oracle tape:

- canonical instrument: `US30`
- broker symbol: `US30`
- timeframe: M5
- tester model: open prices
- research window: 2026-04-13 through 2026-04-14
- rendering: disabled
- controller logging: disabled
- parity oracle: enabled

The tester shuts down after its explicit finalization cutoff. Output is written
under the MT5 common-files `MasterStructureParity/<run_key>` directory. This is
a capture-contract and determinism qualification, not an every-tick production
corpus and not proof of independent Rust auction/topology reconstruction.
