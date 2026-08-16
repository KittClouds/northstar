# FC01-FQB-G0 — Primitive-Extent Selection Jurisdiction

## Outcome

```text
GATE = FC01-FQB-G0
RESULT = COMPLETE_GOVERNANCE_EXTENT_SELECTION_COMPATIBILITY
MODE = READ_ONLY_COMPATIBILITY_QUALIFICATION
```

The four S1-classified primitive extents were tested for semantic noninterference. A future restricted, pre-observation selection may change bounded range extent only; it may not redefine token, grammar, G6, G8, or proof semantics.

| Parameter | Compatibility result | Selection grant | Value selected |
|---|---|---|---:|
| `TOKEN_LENGTH_BOUND` | `GOVERNANCE_EXTENT_SELECTION_COMPATIBLE` | not issued | 0 |
| `MAX_TOKEN_WORD_LENGTH` | `GOVERNANCE_EXTENT_SELECTION_COMPATIBLE` | not issued | 0 |
| `HORIZON_BOUND` | `GOVERNANCE_EXTENT_SELECTION_COMPATIBLE` | not issued | 0 |
| `INTEGER_BOUND` | `GOVERNANCE_EXTENT_SELECTION_COMPATIBLE` | not issued | 0 |

## What G0 earned

G0 earned only a compatibility finding: bounded pre-observation extent selection is semantically separable from the inherited meanings for these four classes. Any future selection authority must be separately issued and restricted to the exact compatible subset.

```text
SELECTION_AUTHORITY_GRANT = NOT_ISSUED
VALUES_SELECTED = 0
ACTUAL_BOUNDS_BOUND = 0
FINITE_BOX_INSTANCE = NONE
```

Primitive extent is not governance selectability. G0 does not choose values, create a design vector, assemble an authorized input vector, or bind a finite box.

## Zero-activity seal

```text
TOKENS = 0
PAIRS = 0
FIBERS = 0
COMPARISONS = 0
CERTIFICATES = 0
G8_VERIFIER_INVOCATIONS = 0
EXPLORER_INVOCATIONS = 0
POPULATION_READS = 0
REAL_04A_HISTORY_READS = 0
TARGET_READS = 0
OUTCOME_READS = 0
```

## Scheduling consequence

`FC01-FQB-C0` remains an independent frontier question. No ordering between G0 and C0 was inferred, and no automatic value-selection grant follows from this result.

## Nonclaims

No concrete values, governance grant, scientific sufficiency, global exhaustiveness, finite-box instance, token authority, G8 authority, or population authority was earned.
