# FC01 adapter instance qualification report

## Outcome

`G8_04A_ADAPTER_INSTANCE_V1` qualified with restrictions under the newly issued
`G8_04A_ADAPTER_PRODUCER_AUTHORITY_V2` grant. This is a new descendant
instance. It is not a recovery or relabeling of the historical G8 adapter.

The qualification was limited to one contract-valid synthetic G1 product. The
adapter invoked the already-qualified A1 projector, selected only named fields,
preserved sequence order, packaged the declared downstream views, and attached
lineage/provenance. The result is transport material, not a G6 fiber, G8
certificate, or G8 verdict.

## Authority chain

```text
NS-GOV-ROOT_V1
  -> FC01-BIRTH-V2
  -> G8_04A_ADAPTER_PRODUCER_AUTHORITY_V2
  -> FC01_ADAPTER_INSTANCE_QUALIFICATION_GATE_V1
  -> G8_04A_ADAPTER_INSTANCE_V1
```

The frozen BIRTH V1 grants remain non-consumable. `FC01-BIRTH-C0` remains a
historical `GRANT_CREATION_AUTHORITY_NOT_ESTABLISHED` fossil. No historical
producer authority was recovered or inferred.

## Evidence

| Check | Result |
|---|---|
| V2 producer grant bound | PASS |
| A1 projector reused without mutation | PASS |
| Synthetic nonpopulation fixture | PASS |
| Direct allowlisted field mapping | PASS |
| Representation-preserving packaging | PASS |
| Sequence order preserved | PASS |
| Return variant preserved | PASS |
| G6 output marked input-only | PASS |
| G8 invocation | 0 |
| New semantic rules | 0 |
| Downstream construction | 0 |
| Protected/population reads | 0 |

The qualification runner replayed the same fixture and returned `PASS` for all
checks. The exact A1 projector source root and the V2 grant root are recorded
in the qualification receipt and instance artifact.

## Authority ceiling

The earned scope is:

```text
DECLARED_SYNTHETIC_AND_FORMALLY_UNDERSTOOD_SYSTEMS_ONLY
```

The instance does not establish real-04A applicability, historical adapter
identity, G6 fiber membership, frozen-comparison authority, certificate
validity, G8 verifier authority, or population authority.

## Scheduler consequence

The post-adapter DAG was recomputed. The adapter gate is complete. The remaining
finite-box binding gate is the sole eligible unscheduled root, and the frozen
lexicographic scheduler selected it without scientific meaning. Selection is
not execution: finite-box bounds remain unbound, tokens remain unrealized, and
G8 remains uninvoked.

## Partner closure

- **Sol:** positive construction/lineage receipt; no semantic invention.
- **Kammi:** no illicit authority promotion or downstream leakage witness.
- **Kage:** conjunction closed within synthetic scope; scheduler recomputation
  sealed; next root selected but not executed.

## Final state

```text
FC01A                       = CLOSED_WITH_RESTRICTIONS
FC01B-A0                    = CLOSED_NOT_EVALUABLE
FC01-DEP                    = CLOSED_NOT_EVALUABLE
FC01-BIRTH-C0               = CLOSED / CREATION AUTHORITY NOT ESTABLISHED
NS-GOV-ROOT_V1              = SEALED
FC01-BIRTH-V2               = SEALED / V2 GRANTS CONSUMABLE
ADAPTER_INSTANCE            = QUALIFIED_WITH_RESTRICTIONS
FINITE_BOX                  = ELIGIBLE / SELECTED / NOT_EXECUTED
TOKENS                      = NONE
PAIRS                       = NONE
G8_VERIFIER_INVOCATIONS     = 0
POPULATION                  = UNOPENED
```
