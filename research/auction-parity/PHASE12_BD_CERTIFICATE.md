# Phase 12B-D reconstruction certificate

Status: `PASS` for 12B, 12C.2-C.5, and 12D on the bounded US30 M5
development oracle. Phase 12C.1 remains explicitly open.

## Proven boundary

The Rust verifier consumes the MT5 parity tape at the raw structural producer
output boundary. From that boundary it independently reconstructs:

1. deterministic normalization and canonical level ordering;
2. compatibility-gated one-dimensional DBSCAN;
3. weighted node construction, provenance, and stable node lifecycle;
4. corridor geometry and regional median/COG/population-sigma state;
5. causal frozen feature snapshots;
6. auction attempts, episodes, events, censoring, and transits;
7. terminal event-sequence and accumulator hashes.

The verifier does not read MT5 node, feature, or auction outputs as runtime
inputs. Those files are comparison oracles only. C1 producer parity is not
claimed because the tape does not contain the producers' raw historical inputs
and warmup state.

## Transit-bearing certificate

Capture:

```text
canonical instrument   US30
timeframe              M5
window                 2026-04-13 through 2026-04-17
tester model           open prices
frames                 5,155
canonical capture SHA  2396c018e7ed9c37272a9de05ba67d539b2e883d3b7c8471ea1e9544b5c8f3ec
```

Exact independent reconstruction:

```text
normalized levels      487,662
DBSCAN rebuilds          1,554
stable node rows        40,459
provenance rows        283,419
regional snapshots       5,155
causal features           5,155
auction events              643
auction attempts            145
auction episodes             52
auction transits               7
terminal hash       5277863164389634815
```

Event order is compared by canonical event sequence. Relational tables are
compared by their primary keys because append order reflects completion time,
not relational identity. Nullable corridor fields preserve the schema's `\N`
semantics while the in-memory engine retains its negative sentinel.

Machine-readable evidence:

- `proof/mt5_bd_transit_oracle_receipt.json`
- `proof/phase12_bd_transit_parity_receipt.json`

## Remaining gate

Closing 12C.1 requires a separate producer-input tape and independent Rust
implementations of VolKitt, profile, day swings, and Wayne with their exact
warmup and state-transition contracts. Nothing in this certificate weakens
MT5's current producer-authority role.
