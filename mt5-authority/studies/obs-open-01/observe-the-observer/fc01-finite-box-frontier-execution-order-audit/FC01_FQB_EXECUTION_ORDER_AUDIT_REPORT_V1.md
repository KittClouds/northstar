# FC01-FQB-ORDER-AUDIT — Frontier Execution-Order Audit

## Result

```text
UNSCHEDULED_EXECUTION_DETECTED
```

The sealed scheduler receipt lists `FC01-FQB-G1` and `FC01-FQB-C1` as eligible roots, selects `FC01-FQB-C1`, and explicitly records:

```text
EXECUTION_AUTHORIZED_BY_THIS_RECEIPT = FALSE
```

Both G1 and C1 result artifacts nevertheless exist. Therefore their substantive results were produced without a recorded scheduler authorization transition.

## Required disposition

Preserve both artifacts exactly. Do not rerun either gate. Do not consume the G1 authority grant, do not treat C1 as a consumable role-selection premise, and do not open C2 until a separate authority audit resolves the chronology.

```text
G1_RESULT = IMMUTABLE / CONSUMABILITY_BLOCKED
C1_RESULT = IMMUTABLE / CONSUMABILITY_BLOCKED
C2 = NOT_LIVE
RERUNS = 0
VALUES_SELECTED = 0
FINITE_BOX_INSTANCE = NONE
POPULATION_READS = 0
```

This is a bookkeeping/authority-order finding, not a scientific failure. No protected material was accessed.
