# FC01-FQB-S0 finite-box parameter source report

## Outcome

S0 classified provenance for all ten finite-box parameter classes without
producing, deriving, inheriting, or selecting any value.

The result is:

```text
COMPLETE_WITH_UNRESOLVED_SOURCE_AUTHORITY
```

Five extent classes still lack a lawful concrete-value source. No parameter
earned `GOVERNANCE_SELECTABLE_EXPERIMENTAL_EXTENT`, so no governance value grant
was issued.

## Per-parameter classification

| Parameter | Source class | Outcome |
|---|---|---|
| `TOKEN_LENGTH_BOUND` | `SEALED_CONTRACT_DOMAIN` | `SOURCE_AUTHORITY_NOT_IDENTIFIED` |
| `MAX_TOKEN_WORD_LENGTH` | `SEALED_CONTRACT_DOMAIN` | `SOURCE_AUTHORITY_NOT_IDENTIFIED` |
| `HORIZON_BOUND` | `SEALED_CONTRACT_DOMAIN` | `SOURCE_AUTHORITY_NOT_IDENTIFIED` |
| `INTEGER_BOUND` | `SEALED_CONTRACT_DOMAIN` | `SOURCE_AUTHORITY_NOT_IDENTIFIED` |
| `ENUM_DOMAINS` | `SEALED_CONTRACT_DOMAIN` | `VALUE_SOURCE_IDENTIFIED` |
| `RANK_SHELL_CONTRACT` | `SEALED_CONTRACT_EXACT_VALUE` | `VALUE_SOURCE_IDENTIFIED` |
| `CANONICAL_TOKEN_ORDER` | `SEALED_CONTRACT_EXACT_VALUE` | `VALUE_SOURCE_IDENTIFIED` |
| `INTEGER_DOMAIN` | `SEALED_CONTRACT_DOMAIN` | `VALUE_SOURCE_IDENTIFIED` |
| `BOX_CARDINALITY` | `SEALED_CONTRACT_DOMAIN` | `SOURCE_AUTHORITY_NOT_IDENTIFIED` |
| `PROOF_BOX_IS_FINITE` | `CONTRACT_AUTHORIZED_MECHANICAL_DERIVATION` | `VALUE_SOURCE_IDENTIFIED` |

`PROOF_BOX_IS_FINITE` has an identified derivation rule, but its predecessors
are not yet bound; the derivation was not executed.

## Preserved distinctions

```text
source identified        != value materialized
domain known              != member selected
derivation authorized     != derivation executed
missing source authority  != governance may select
```

S0 did not assemble a binding vector, issue a governance value-selection grant,
or revisit the failed V1 binding attempt.

## Activity seal

```text
VALUES_SELECTED          = 0
VALUES_DERIVED           = 0
VALUES_INHERITED         = 0
ACTUAL_BOUNDS_BOUND      = 0
FINITE_BOX_INSTANCE      = NONE
TOKENS / PAIRS / FIBERS  = 0
COMPARISONS / CERTS      = 0
G8_INVOCATIONS           = 0
POPULATION_READS         = 0
```

## Next lawful branch

The five `SOURCE_AUTHORITY_NOT_IDENTIFIED` classes require a separate authority
resolution question. No governance-selectable subset has been earned yet. The
campaign remains on hold before any actual number enters the ruler.
