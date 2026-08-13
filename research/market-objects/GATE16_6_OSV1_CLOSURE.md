# Gate 16.6 — Observational System V1 closure

Status: `PROTOCOL CORRECTED / SOURCE CLOSURE PASS / NO NEW OBSERVATIONS`

Gate 16.6 seals the continent already mapped. It does not extend the map.

The governing law is:

> Record exactly what the existing observational system saw, how it saw it,
> where it is blind, and where it has not been evaluated.

The machine-readable authority is
`contracts/gate16_6_osv1_closure.json`.

## Identity

OSV1 is:

```text
OSV1 = (W_stage_matrix, Theta0, D0, A0)
```

- `W_stage_matrix` is a typed component bundle with authority and parity
  declared per stage.
- `Theta0` is the admitted composite multitimeframe configuration.
- `D0` contains the separately sealed RG2 and RG3 empirical branches.
- `A0` is their immutable artifact and receipt DAG through Gate 16.5.

OSV1 is not accurately described as one fully parity-verified implementation.
The certified boundary is narrower and stronger:

```text
C1 raw structural producers
  MT5 authority                         SEALED
  independent Rust producer parity     OPEN

C2-D downstream reconstruction
  Rust reconstruction from raw
  producer-output boundary              PASS

RG3 market-object science
  MT5 raw observations                  AUTHORITY
  Rust derivation and sealing           DETERMINISTIC
```

The open C1 boundary is constitutional. Phase 12B-D did not receive producer
input history or exact producer warmup state and therefore did not claim
independent VolKitt, profile, day-swings, or Wayne reconstruction.

## Stagewise parity matrix

The executable matrix prevents adjacent evidence from being blended into one
oversized parity claim. Every row binds the stage boundary, MT5 and Rust roles,
claim class, measured population, certified scope, explicit exclusions, and
the SHA-256 identities of its evidence.

The implementation-parity lane is:

```text
C1 raw structural producers                    OPEN
C2 normalization and canonical ordering        PASS_EXACT
C3 compatibility and DBSCAN                    PASS_EXACT
C4 node provenance and lifecycle               PASS_EXACT
C5 corridor and regional state                 PASS_EXACT
D1 causal frozen features                      PASS_EXACT_CAUSAL_FRAME
D2 auction state and relational ledger         PASS_EXACT
D3 transit receipts                            PASS_EXACT
```

The PASS rows are bounded to the five-session US30 M5 oracle containing 5,155
frames. They do not repair or imply C1 parity, alternative observer-instance
coverage, or cross-scale transport.

The same matrix separately records evidence that must not be called this
implementation parity:

```text
MT5 capture replay                determinism, not parity
Phase 11 model inference          implementation parity, not market confirmation
Gate 13R behavioral holdout       confirmation, not implementation parity
RG3 corpus                        observational authority and sealing
Gates 15 through 16.5             derived rebuild determinism, not MT5 parity
```

Canonical artifacts:

- `artifacts/gate16-6-prep/osv1_stagewise_parity_matrix.json`
- `artifacts/gate16-6-prep/osv1_stagewise_parity_matrix.tsv`
- `artifacts/gate16-6-prep/osv1_stagewise_parity_matrix_receipt.json`

The deterministic generator is `tools/New-OsV1ParityMatrix.ps1`; the
fail-closed byte-rebuild and authority test is
`tools/Test-OsV1ParityMatrix.ps1`.

## Observer-instance scope

Theta0 is one composite clock vector:

```text
Master       M5
VolKitt      H1
Profile      H1
Day Swings   M5
Wayne        D1
```

H1 and D1 are therefore not generically "unobserved timeframes." They already
participate in Theta0. What remains unobserved is the family of alternative
clock vectors: another Master clock, global clock dilation, one-producer clock
substitution, or changed producer-to-Master relationships.

Timeframe ratios may be stored as descriptive configuration codes only. Market
sessions and nonuniform calendar time prevent interpreting `D1/M5` as a clean
physical temporal ratio.

## Theta0 evidence

Every field in the future Theta0 census requires one of these evidence classes:

```text
RUN_RECORDED
CAMPAIGN_CONFIGURED
COMPILED_DEFAULT
RECOVERED_FROM_COLLECTION_ENVIRONMENT
DECLARED_HASHED_NOT_OPERATIONAL
UNAVAILABLE
```

An exposed or hashed setting is not automatically operational. A recovered
source or executable hash is not run-bound unless an admitted run receipt
contains that hash.

Current preparatory evidence establishes that all 42 admitted RG3 runs share:

```text
recorded configuration hash  11558041990222457072
tester timeframe             M5
data source                  BROKER_MT5
```

Their 42 tester configurations also contain one identical non-identity input
preset. This demonstrates a coherent admitted observer point. It does not
retroactively add an expanded Master configuration or executable hash to the
run receipts.

`InpRequireFreshLineageWindow` is declared and included in the collector
configuration text but is not consumed by the collector implementation. Its
Theta0 status is `DECLARED_HASHED_NOT_OPERATIONAL`.

### Field-level reconstruction

The completed Θ₀ census resolves 134 fields over all 42 admitted RG3 runs.
Each field records:

```text
typed value summary
evidence class
run-binding status
consumption status
configuration-hash membership
exact source/config/run-ledger provenance
```

The census deliberately keeps three evidence strengths separate:

```text
measurement_runs.tsv values          RUN_RECORDED
42 preserved tester INI values       CAMPAIGN_CONFIGURED
expanded Master defaults             RECOVERED_FROM_COLLECTION_ENVIRONMENT
```

The last class is not promoted to run-bound fact. The admitted run receipts do
not contain the exact EX5 hash or decoded Master `ConfigText`. The source
closure proves what is presently preserved in the collection environment; it
does not prove which exact binary each historical invocation loaded.

All 42 run ledgers record one RG3 configuration hash:

```text
11558041990222457072
```

That hash covers RG3 `ObjectConfigText`, including compression/expansion
settings and Master semantic-version identifiers. It does not decode or bind
every expanded Master default.

Canonical artifacts:

- `artifacts/gate16-6-prep/osv1_theta0_fields.json`
- `artifacts/gate16-6-prep/osv1_theta0_fields.tsv`
- `artifacts/gate16-6-prep/osv1_theta0_runs.tsv`
- `artifacts/gate16-6-prep/osv1_theta0_receipt.json`

The deterministic generator is `tools/New-OsV1Theta0.ps1`.

## Machine-only findings

The OSV1 findings ledger admits only sealed JSON authority. Every source
artifact is either a machine root certificate/manifest or a child whose exact
SHA-256 is named by a machine seal receipt. Markdown, prose reports, human
summaries, unsealed JSON, and reconstructed interpretations are rejected.

The generated ledger contains:

```text
sealed machine evidence nodes     8
findings                          24
explicit insufficiencies          18
human-summary sources              0
Markdown sources                   0
unsealed machine sources           0
```

Every finding and insufficiency carries one explicit `source_kind`:

- `MACHINE_DERIVED` for a quantity computed by the sealed machine artifact;
- `ARTIFACT_DECLARED` for a status or limitation declared by that artifact;
- `HUMAN_SUMMARY`, which is part of the type system but is inadmissible as
  OSV1 authority and therefore has a canonical count of zero.

Its evidence roots span Gate 13R, Gate 15, Gate 15.5, Gate 16, and Gate 16.5.
The findings retain their source artifact's epistemic status, population,
measurements, JSON pointers, and claim limits. The insufficiency ledger keeps
unavailable auction intervals, forbidden conclusions, and other explicit
limits visible instead of silently dropping them.

Canonical artifacts:

- `artifacts/gate16-6-prep/osv1_machine_findings.json`
- `artifacts/gate16-6-prep/osv1_machine_findings.tsv`
- `artifacts/gate16-6-prep/osv1_machine_insufficiencies.tsv`
- `artifacts/gate16-6-prep/osv1_machine_findings_receipt.json`

The deterministic generator is `tools/New-OsV1MachineFindings.ps1`; the joint
byte-rebuild and authority test is `tools/Test-OsV1AuthorityArtifacts.ps1`.

## Ancestry shape

The empirical ancestry is a DAG, not a single line:

```text
                         shared research ancestry
                         /                      \
RG2 -> 10 -> 10.5 -> 11 -> Gate 13R            RG3 -> 14 -> 15 -> 15.5 -> 16 -> 16.5
                         \                      /
                              OSV1 root seal
```

Leaves and registries are hashed first. The OSV1 manifest is built after its
children and the root receipt is written last. Circular self-hashing is
forbidden.

Canonical ancestry and root artifacts:

- `artifacts/gate16-6-prep/osv1_ancestry_dag.json`
- `artifacts/gate16-6-prep/osv1_ancestry_nodes.tsv`
- `artifacts/gate16-6-prep/osv1_ancestry_edges.tsv`
- `artifacts/gate16-6-prep/osv1_root_manifest.json`
- `artifacts/gate16-6-prep/osv1_root_receipt.json`

`tools/New-OsV1Ancestry.ps1` validates every predecessor contract and status,
binds each evidence file by exact SHA-256, proves acyclicity, and emits the
root receipt only after all child artifacts and the root manifest exist.

`tools/Test-OsV1Closure.ps1` performs two independent clean rebuilds of all 17
closure artifacts, requires byte identity between the rebuilds and the
canonical closure, revalidates the DAG/root receipts, and seals
`osv1_double_rebuild_proof.json`.

## Source dependency closure

The source census has two different exactness contracts:

- MT5 uses the recursive preprocessor include graph selected by the RG3
  collector in the collection terminal. Absolute includes remain absolute;
  current mirrors cannot silently replace them.
- Rust seals repository-relative workspace manifests, lockfiles, and local
  source for the auction-parity and market-object workspaces. Cargo registry
  identities remain represented by their locked package records.

This distinction matters because the collection terminal's
`MasterParityOracle.mqh` wrapper selects an absolute source in the older
Northstar checkout. The census preserves that actual dependency and may report
that a current mirror differs. It does not rewrite history to make the tree
look cleaner.

The completed preparatory census records:

```text
MT5 recursive source files          26
MT5 include edges                   36
unresolved MT5 includes              0

auction-parity local Rust files     53
auction-parity local crates         10
auction-parity locked packages      52

market-objects local Rust files     40
market-objects local crates          1
market-objects locked packages      65

logical closure SHA-256
ef21a72a0109c179469e8b4fd7820a8db218d687622a21c5bb932792e07d42fa
```

The exact artifact is
`artifacts/gate16-6-prep/osv1_source_dependency_closure.json`. Its deterministic
rebuild test is `tools/Test-OsV1SourceClosure.ps1`.

## Preserved limits

Gate 16.6 does not authorize:

- C1 producer parity;
- cross-scale transport;
- auction interval conditioning;
- a preferred representation;
- mechanism or causation;
- economic usefulness;
- policy, deployment, or trading;
- use of the twelve unopened Gate 15 confirmation windows.

Its constrained claim is:

> Gate 16.6 closes one admitted composite observer instance with MT5 producer
> authority and independently parity-verified downstream Rust reconstruction
> from the raw producer-output boundary. It seals the RG2 and RG3 empirical
> ancestry through Gate 16.5 while leaving independent C1 parity, alternative
> observer configurations, cross-scale transport, auction-interval
> conditioning, mechanism, economics, and trading authority explicitly
> unresolved.
