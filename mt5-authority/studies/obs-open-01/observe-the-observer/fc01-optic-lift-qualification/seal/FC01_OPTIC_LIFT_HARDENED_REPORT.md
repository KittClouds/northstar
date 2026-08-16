# FC-01 Sol Optic Lift — Hardened Eligibility Report

## Result

    EXECUTION_STATE = SEALED
    QUESTION_STATUS = CLOSED
    RESULT         = FC01_SOL_NOT_EVALUABLE
    DISPOSITION    = HOLD

FC-00 remains sealed. This layer is eligibility and authority qualification only; it does not alter the five FC-00 scientific identities or their implementations.

## Hardenings applied

1. The lift mode vocabulary now has three values: EXECUTABLE_HISTORY_TRANSFORM, OBSERVABLE_PROJECTION, and DERIVED_OPTICAL_SIDECAR.
2. P1 and P5 are derived sidecars. P2 and P3 are observable projections. P4 is conservatively a derived sidecar until an executable-history proof is earned.
3. G6 equivalence is never inherited automatically. Each optic has an arm-local comparison relation and an unqualified verifier bridge.
4. One common G8_04A_SEMANTIC_ADAPTER_V1 is required before any optic can use real-history certificates.
5. Pairing is frozen as a precomparability specification. Pair count and cohort instantiation remain unbound; comparability failure cannot trigger backfill.
6. The finite question box is a canonical question language. Token identity is independent of certificate serialization, and one token must map to one deterministic realization.
7. Separator incidence and certificate evidence receive separate roots. Total-domain and common-admissible-domain relations are distinct, and EMPTY_BOTH requires a nonempty exhaustively evaluated common domain.

## Arm statuses

    SOL-P1 = ARM_NOT_EVALUABLE
    SOL-P2 = ARM_NOT_EVALUABLE
    SOL-P3 = ARM_NOT_EVALUABLE
    SOL-P4 = ARM_NOT_EVALUABLE
    SOL-P5 = ARM_NOT_EVALUABLE

The common real-history adapter, G5 token realization, G6 arm-local relations, and G8 verifier bridges are not qualified. This is a fail-closed result, not a request to inherit authority by analogy.

## Population firewall

    population reads                  = 0
    pair cohort instantiated           = FALSE
    question tokens realized          = 0
    G8 real-history certificates      = 0
    arm executions                    = 0
    D_B / D_C / D_D / P6 reads        = 0
    cross-arm result reads            = 0

## Next lawful boundary

Qualify the common 04A semantic adapter and its verifier path first. Then bind pair count, structural strata, finite question-box bounds, deterministic token realization, arm-local comparison relations, and optic verifier bridges. Only after those pass may FC-01 consider admission eligibility.

    FC-02 = NOT_AUTHORIZED
    MAXIMUM_AUTHORITY = FC01_OPTIC_LIFT_SCHEMA_CONTRACT_ONLY

