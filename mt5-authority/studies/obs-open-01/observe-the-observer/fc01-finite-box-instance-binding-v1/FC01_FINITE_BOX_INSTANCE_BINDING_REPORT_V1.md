# Finite-box instance binding report

## Outcome

The exact qualified rule, rule-qualification authority, V2 binding authority,
and frozen authorized input vector were bound. The rule was invoked exactly
once.

The result was deliberately not repaired:

```text
RULE_APPLICATION              = PASS
FINITE_BOX_SCHEMA_VALIDATION   = FAIL
FINITE_BOX_AUTHORITY_BINDING  = NOT_EVALUABLE
OVERALL                       = NOT_EVALUABLE
```

The rule returned a qualified descriptor with all ten registered parameter
classes still unbound. Therefore no finite-box instance was created.

## Unbound classes

```text
TOKEN_LENGTH_BOUND
MAX_TOKEN_WORD_LENGTH
HORIZON_BOUND
INTEGER_BOUND
ENUM_DOMAINS
RANK_SHELL_CONTRACT
CANONICAL_TOKEN_ORDER
INTEGER_DOMAIN
BOX_CARDINALITY
PROOF_BOX_IS_FINITE
```

This is not a partial pass. No `9/10` promotion, default substitution,
clamping, rounding, fallback, retry, or manual override occurred.

## Execution controls

| Control | Result |
|---|---|
| Exact rule root bound | PASS |
| Exact rule-authority root bound | PASS |
| Exact V2 authority root bound | PASS |
| Exact input-vector root bound | PASS |
| Rule application count | 1 |
| Retries | 0 |
| Fallbacks | 0 |
| Manual overrides | 0 |
| Actual bounds | 0 |
| Finite-box instance | NONE |
| Tokens/pairs/fibers/comparisons | 0 |
| Certificates/coverage receipts | 0 |
| G8 invocations | 0 |
| Population/real-history reads | 0 |

## DAG consequence

The authority DAG was recomputed after the failed binding attempt. No new
finite-box instance authority artifact exists, so no downstream producer root
became eligible. The scheduler correctly holds.

## Interpretation

The qualified rule is not being declared defective. It was qualified as a
rule descriptor whose values remain unbound. The binding input vector supplied
no authorized actual bound values, and the sealed finite-box schema correctly
rejected the resulting unbound output.

Any future box requires a new, explicitly authorized input lineage or a new
rule/authority lineage. This receipt is immutable.
