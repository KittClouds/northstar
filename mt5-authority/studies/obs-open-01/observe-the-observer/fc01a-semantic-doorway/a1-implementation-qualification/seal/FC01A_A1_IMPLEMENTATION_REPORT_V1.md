# FC-01A A1 — Sealed G1 Invocation + Projection Qualification

## Result

```text
EXECUTION_STATE = SEALED
QUESTION_STATUS  = CLOSED
RESULT           = NOT_EVALUABLE
DISPOSITION      = HOLD
```

The single A1 question was:

> Does the executable invoke the exact qualified G1 path and then perform only the A0-authorized projections, without reconstructing semantics, leaking authority, or introducing state?

The projection-only path qualified on the synthetic nonpopulation corpus. The exact sealed G1 callable boundary is not materialized in this checkout, so the end-to-end path remains `NOT_EVALUABLE`. This is deliberately not upgraded by projection-only success.

## Architecture and evidence paths

```text
END_TO_END_G1_PATH:
    synthetic canonical G0 input
        -> exact sealed G1 invocation
        -> G1_SEMANTIC_OUTPUT_V1
        -> FC01A projection

PROJECTION_ONLY_PATH:
    contract-valid synthetic G1 semantic object
        -> FC01A projection
```

G1 causal execution was never reordered. Only already-valid semantic products were projected in different invocation orders for the pointwise independence test.

The implementation contains no G1 lifecycle, genealogy, rejection, or temporal reconstruction. It has no mutable process state and reads only the declared G1 ABI fields plus the frozen view registry.

## A1 artifacts

```text
G1_FC01A_INVOCATION_CONTRACT_V1
G1_FC01A_ABI_V1
FC01A_PROJECTION_REGISTRY_V1
FC01A_CANONICAL_SERIALIZATION_V1
FC01A_REJECTION_CONTRACT_V1
FC01A_PROOF_OBLIGATION_LEDGER_V1
```

The projection graph is strictly:

```text
G1 semantic output -> G5 view
G1 semantic output -> G6 view
G1 semantic output -> G7 view
G1 semantic output -> G8 view
```

Projection-to-projection reads are zero. Each view has its own allowlist and root. Ordered emissions preserve source order; unordered object keys use the frozen canonical JSON encoding.

## Rejection namespaces

`G1_REJECTION_STATUS` is pass-through only: no renaming, enrichment, reinterpretation, or dynamic explanation. `FC01A_DOORWAY_DISPOSITION` is a separate finite mechanical namespace. Doorway failures contain only a registered code and no raw values, threshold distances, or content-derived diagnostics.

## Projection-only qualification results

```text
authorized field closure             = PASS
irrelevant-field noninterference    = PASS
pointwise projection order freedom  = PASS
G1 rejection pass-through           = PASS
missing primitive fails closed       = PASS
canonical serialization/replay      = PASS
population blindness                 = PASS
```

The fixtures include valid authorized-field mutations, valid irrelevant-field mutations, G1 rejection pass-through, missing-primitive fail-closed behavior, and repeated projector orders `ABC`, `CAB`, and `BCA`. Invalid ABI objects are not used as noninterference evidence.

## Exact G1 invocation boundary

No qualified G1 callable was available to this A1 execution. The end-to-end invocation returns `G1_REQUIRED_PRIMITIVE_UNAVAILABLE` without fabricating a substitute. Therefore these claims remain unearned:

```text
G1 callable binding
G1 initialization mode
G1 context binding
G1 input ordering at invocation
G1 session-boundary handling
G1 output capture identity
```

Projection-only success is not sealed-G1 invocation success.

## Access and nonclaims

```text
population reads                       = 0
population-derived content reads      = 0
population-derived metadata reads     = 0
pair cohorts                           = 0
question tokens                        = 0
G8 real-history certificates           = 0
downstream sufficiency tests           = 0
```

No target, D_B, D_C, D_D, 04A population, pair, or outcome was read. No FC-01B work is authorized.

## Maximum authority

```text
FC01A_PROJECTION_LAYER_QUALIFIED_WITHOUT_G1_INVOCATION_AUTHORITY
```

The overall A1 result remains `NOT_EVALUABLE` until the exact sealed G1 implementation is bound and the end-to-end path is independently replayed.
