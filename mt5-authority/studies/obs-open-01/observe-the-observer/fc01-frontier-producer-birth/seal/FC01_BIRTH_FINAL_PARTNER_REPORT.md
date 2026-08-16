# FC01-BIRTH — New Frontier Producer Authority Constitution

## Decision

`FC01-BIRTH` opened as a new governance lineage because historical producer authority remains `NOT_IDENTIFIED` for both frontier artifacts. This gate does not perform historical recovery and does not construct scientific instances.

Result:

```text
NEW_PRODUCER_AUTHORITIES_CREATED_WITH_RESTRICTIONS
```

## Historical discontinuity

The previous fossil remains unchanged:

```text
FC01-OWN = FRONTIER_OWNERSHIP_PARTIAL
G8_04A historical producer = NOT_IDENTIFIED
finite-box historical producer = NOT_IDENTIFIED
```

The new grants are explicitly new lineage objects. They are not historical authority and cannot be replayed as if they were.

## Grant A — adapter producer authority

```text
G8_04A_ADAPTER_PRODUCER_AUTHORITY_V1
role       = FC01_ADAPTER_PRODUCER_ROLE_V1
qualifier  = FC01_ADAPTER_INSTANCE_QUALIFICATION_GATE_V1
transport  = FC01_ADAPTER_TRANSPORT_GATE_V1
instance   = NOT_CREATED
qualified  = NOT_QUALIFIED
```

This grant permits a future qualification gate to ask whether a representation-preserving adapter instance can be constructed from the sealed G1–G8 semantic contracts. It does not grant new G1, G5, G6, G7, or G8 semantics.

## Grant B — finite-box binding authority

```text
FINITE_QUESTION_BOX_BINDING_AUTHORITY_V1
role       = FC01_FINITE_BOX_BINDER_ROLE_V1
qualifier  = FC01_FINITE_BOX_INSTANCE_QUALIFICATION_GATE_V1
transport  = FC01_FINITE_BOX_TRANSPORT_GATE_V1
bounds     = NOT_BOUND
instance   = NOT_CREATED
```

The grant permits a future gate to qualify a precommitted, content-independent bound-selection rule. It does not select token length, shell, horizon, integer limits, or cardinality now.

Forbidden inputs remain:

```text
population values
population distributions
explorer outcomes
G8 verifier results
cross-arm results
post-hoc tuning
```

## DAG recomputation

After the grants were created, the sealed DAG was recomputed:

```text
cycle_status = NO_CYCLE_FOUND_IN_DECLARED_EDGES
eligible_root_questions = [
    FC01_ADAPTER_INSTANCE_QUALIFICATION_GATE_V1,
    FC01_FINITE_BOX_BINDING_RULE_QUALIFICATION_GATE_V1
]
instances_created = 0
scheduler = EXPOSE_ELIGIBLE_PRODUCER_QUESTIONS_HOLD_EXECUTION
next_gate_woken = NONE
```

Eligibility exposes producer-specific questions. It does not authorize execution.

## Partner results

Sol created the two restricted grants and bound them to sealed semantic roots. Kammi found no historical relabeling, instance/qualification promotion, downstream semantic sculpting, or population-conditioned binding. Kage verified the new lineage, ceilings, zero construction, and scheduler hold.

## Zero-activity seal

```text
adapter implementation             = 0
question-box bound selection       = 0
token realization                  = 0
pair/fiber/comparison construction = 0
certificate/coverage construction  = 0
G8 verifier invocations            = 0
explorer invocations               = 0
real 04A / population reads        = 0
target / outcome reads             = 0
```

## Board after FC01-BIRTH

```text
HISTORICAL FC01 GRAPH = HOLD / NO HISTORICAL PRODUCER GRANTS IDENTIFIED
FC01-BIRTH            = CLOSED / NEW RESTRICTED GRANTS CREATED
ELIGIBLE ROOTS        = PRODUCER-SPECIFIC QUESTIONS EXPOSED
LIVE                  = NONE
SCHEDULER             = HOLD EXECUTION
ADAPTER INSTANCE      = NOT CREATED
QUESTION BOX BOUNDS   = NOT BOUND
FC01C+                = FROZEN
P1-P5                 = BENCHED
P6                    = BLOCKED
POPULATION            = UNOPENED
```

The next action, if chosen, is a producer-specific qualification constitution. No instance is born merely because its producer role now exists.
