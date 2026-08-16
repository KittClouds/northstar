# FC01-DEP — Final Prerequisite Authority DAG Report

## Decision

```text
GRAPH_MATERIALIZATION = PASS_WITH_UNRESOLVED_AUTHORITY_NODES
CALLABLE_SELECTION    = PASS_FOR_DECLARED_ACTIVE_SET
CYCLE_CHECK           = PASS_NO_CYCLE_FOUND_IN_DECLARED_EDGES
FC01-DEP              = NOT_EVALUABLE
ROOT_PRODUCERS        = []
NEXT_GATE             = NONE
```

The assumed linear roadmap was not promoted. The sealed contracts expose a dependency graph with several authority nodes whose instance producers and transport owners are not identified.

## Active G8 callable set

The frozen FC01 contracts select only:

```text
verify_separator
    selected because separator incidence requires EXACT_VERIFIER_ACCEPT

verify_bounded_exhaustion
    selected because the finite question box requires exact finite coverage
```

The following sealed G8 surfaces remain indexed but inactive in the declared FC01 scope:

```text
verify_ancestry
verify_no_witness_box
verify_finite_box_minimum
verify_universal_proof
```

They were not silently added to the active requirement union.

## The authority nodes discovered

Satisfied or sealed roots:

```text
FC01A_DOORWAY
G8_BOUNDARY
G5_GRAMMAR_AUTHORITY
G5_TOKEN_REALIZATION_CONTRACT
G8_AUTHORITY_BINDING
```

Unresolved nodes:

```text
G8_04A_SEMANTIC_ADAPTER
FINITE_QUESTION_BOX_AUTHORITY
G5_TOKEN_REALIZATION_INSTANCE
G6_FIBER_AUTHORITY
G8_FROZEN_COMPARISON
G8_SEPARATOR_CERTIFICATE
G8_COVERAGE_RECEIPT
```

The distinction between the token realization contract and a token realization instance is intentional. The contract is a sealed semantic object; it does not grant authority to produce instances.

## To Sol

The positive archaeology receipt confirms:

- `verify_separator` and `verify_bounded_exhaustion` are the active G8 consumers for the frozen FC01 contract;
- `FC01A` is not identical to `G8_04A_SEMANTIC_ADAPTER`;
- G5 owns token semantics, but no lawful token-instance producer is identified;
- G6 owns comparison/fiber semantics, but no 04A fiber instance is available;
- the `FrozenComparison` owner is not identified as a single sealed authority;
- no producer root is eligible.

No FC01C/D/E implementation or output design was performed.

## To Kammi

The scheduler-laundering attacks all held:

```text
consumer requirement -> producer authority       BLOCKED
constructor exists -> invocation authority       BLOCKED
schema equality -> instance authority             BLOCKED
old roadmap order -> dependency evidence          BLOCKED
similarity -> fiber membership                     BLOCKED
helper function -> claim authority                 BLOCKED
downstream need -> upstream redesign                FORBIDDEN
```

The declared graph has no cycle under the separated contract/instance nodes. This is not a claim that every future expanded graph is acyclic; it is only the result for the sealed edge set.

## To Kage

The graph is mechanically recorded in:

```text
G8_CALLABLE_DEPENDENCY_LEDGER_V1
FC01_AUTHORITY_OBJECT_OWNERSHIP_LEDGER_V1
FC01_PREREQUISITE_AUTHORITY_DAG_V1
FC01_GATE_PRODUCER_CONSUMER_MAP_V1
```

The computed eligible producer set is empty. Therefore no downstream gate is awakened. `FC01-DEP` closes as `NOT_EVALUABLE` because required semantic/instance/transport ownership is not fully earned, not because a verifier rejected anything.

## Adapter naming firewall

```text
FC01A_COMPOSITE_CLOSURE
    !=
G8_04A_SEMANTIC_ADAPTER_CONTRACT_V1
```

The relation is recorded as partial surface overlap, not identity. The G8 adapter contract remains `NOT_QUALIFIED`.

## Access audit

All counters are zero:

```text
REAL_04A_HISTORY_READS = 0
POPULATION_READS       = 0
PAIR_CONSTRUCTION      = 0
PAIR_READS             = 0
TOKEN_REALIZATION      = 0
FIBER_CONSTRUCTION     = 0
CERTIFICATE_CONSTRUCTION = 0
G8_VERIFIER_INVOCATIONS = 0
EXPLORER_INVOCATIONS   = 0
ARM_EXECUTIONS         = 0
TARGET_READS           = 0
OUTCOME_READS          = 0
```

## Board state

```text
FC01A    = CLOSED_WITH_RESTRICTIONS
FC01B-A0 = CLOSED_NOT_EVALUABLE / TOPOLOGY INPUT
FC01-DEP = CLOSED_NOT_EVALUABLE
FC01B-A1 = FROZEN
FC01C+   = FROZEN
P1-P5    = BENCHED
P6       = BLOCKED
POPULATION = UNOPENED
PAIRS      = NONE
TOKENS     = NONE
```

## Governing sentence

```text
The schedule is now an empirical object.
No root producer was earned.
Nothing wakes.
```
