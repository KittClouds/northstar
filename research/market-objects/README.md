# Gate 14: deterministic market-object synthesis

Gate 14 adds compression and expansion trajectories without changing the
sealed RG2 auction corpus or the Gate 13 behavioral models. The combined
research generation is RG3; its structural source remains RG2.

The authoritative corpus contains only observations emitted from closed market
bars and frozen sensor geometry. It never contains normalization, mirroring,
resampling, embeddings, clusters, names for discovered families, or trading
labels. Those are separately versioned, reproducible derived artifacts.

The object chain is:

```text
compression C[i] -> expansion E[i] -> compression C[i+1]
                         |
                         +-> timestamped structural-node relations
```

Gate 13 probabilities may later be attached as causal forecast receipts. They
cannot create, terminate, merge, or classify a market object.

## Contracts

- `contracts/rg3_identity.json`: frozen semantic versions.
- `contracts/raw_authority.json`: raw versus derived boundary.
- `contracts/schema_dictionary.json`: normalized TSV datasets.
- `contracts/object_ontology.md`: lifecycle and availability-time semantics.

## Exit evidence

Gate 14 exits only after MQL5 golden tests, incremental/rebuild replay parity,
Rust relational validation, raw corpus sealing, and independent derived-view
regeneration all pass.

## Hash identities

- `byte_sha256` seals the exact nine input files, including the unique
  `invocation_id`.
- `canonical_sha256` excludes only `invocation_id`; it is the replay-stable
  semantic identity of one run.
- `packed_sha256` seals the lossless packed artifact and therefore changes with
  a distinct invocation.
- `canonical_corpus_sha256` binds sorted run keys, canonical run hashes, and
  the derived recipe hash. It does not depend on invocation bytes.

## Qualification evidence

The bounded six-market qualification is sealed under
`artifacts/rg3-qualification`:

```text
markets                 6 / 6 PASS
compression objects     75
compression samples     2,169
expansion objects       74
expansion samples       4,354
object relations        431
structural samples      6,567
```

`replay-certificate-US30.json` proves a separate US30 invocation produced the
same canonical semantic hash and byte-identical non-invocation datasets. The
exact byte and packed hashes differ, proving invocation provenance was retained
rather than erased.

Build and verification evidence:

```text
MQL5 collector          0 errors / 0 warnings
Rust unit tests         4 passed / 1 explicit performance lane
Rust clippy             -D warnings PASS
100k mmap validation    PASS (84,053 us observed on this workstation)
```

## Gate 15: representation-specific trajectory families

Gate 15 extends the sealed RG3 corpus to 42 runs and 1,091 compression and
expansion objects across DE40, FRA40, JPN225, US30, US500, and USTEC. Raw
authority is unchanged. Discovery operates only on replaceable views derived
from the sealed close path, origin ATR, raw displacement and velocity, and
market timestamps.

Three deliberately different family systems are retained per object kind:

```text
summary geometry
resampled trajectory shape
mirrored hybrid geometry
```

Each system has its own deterministic SHA-256 identity. Candidate IDs such as
`C1` and `C2` are local to one representation. `NULL` is an explicit unmatched
assignment and is never a family. Reports expose noise rate, seed stability,
instrument/window concentration, and pairwise family correspondence both with
and without noise. Cross-view disagreement is preserved as information; Gate
15 does not claim a consensus taxonomy.

The prospective V1 discovery report remains sealed as `COLLECT_MORE`. V2 was
created after inspecting that failure, so its supported families are
exploratory rather than independently confirmed. Six family-system identities
and twelve exact future confirmation runs are frozen in
`campaigns/rg3-gate15-family-confirmation.json`; those windows remain unopened
and unauthorized in Gate 15.

Final evidence lives under `artifacts/rg3-gate15-seal-final-v4`:

```text
admitted runs                 42
market objects             1,091
minimum objects/instrument   160
compression censoring       2.7%
expansion censoring         5.2%
largest family market share 24.0%
largest family window share 23.8%
derived byte replay          PASS
aggregate corpus seal        PASS
```
