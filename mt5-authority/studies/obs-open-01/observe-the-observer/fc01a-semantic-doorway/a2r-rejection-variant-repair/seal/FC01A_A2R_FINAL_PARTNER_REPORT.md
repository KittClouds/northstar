# FC01A-A2R Final Repair Report

## Shared result

The A2 failure was repaired in a new lineage. A1 and the failed A2 result remain immutable.

```text
PARENT A2 = NOT_EVALUABLE
REPAIR = REPAIR_QUALIFIED_WITH_RESTRICTIONS
FC01A CLOSURE = NOT_ISSUED
FC01B+ = FROZEN
POPULATION = UNOPENED
```

## The repair

The frozen ABI is now treated as a discriminated return-variant contract:

```text
APPLIED
    -> frozen G1_FC01A_ABI_V1
    -> unchanged A1 projector

REJECTED
    -> explicit rejection view
    -> no state, emission, or time fabrication
```

The rejected variant contains only G1 rejection status, authorized context, exact failed-input provenance, return-variant status, and lineage. It cannot be mistaken for an applied state.

## To Sol

The selected G1-R executable instance and parent roots remain unchanged. Five applied `StepResult` records continue through the original A1 projector. Two rejected G1 returns now have a lawful mechanical representation without inventing missing primitives.

The repair adds no G1 semantics, lifecycle rules, genealogy, timing, rejection reinterpretation, reachability inference, fiber inference, or token realization.

## To Kammi

The repaired path blocks:

- rejected-to-applied coercion;
- synthetic empty-state substitution;
- synthetic empty-emission substitution;
- inferred rejection timestamps;
- rejection-code rewriting;
- cross-variant field leakage;
- A1 projector mutation;
- population-scope expansion.

Replay A and Replay B are byte-identical.

## To Kage

The repair is qualified only as a new restricted doorway component. It does not silently promote the prior A2 result and does not issue full FC-01A closure.

The historical chain remains:

```text
G1-M historical executable identity failed
    -> G1-R descendant authority earned
    -> A2 bound one executable
    -> A2 found rejected-return ABI insufficiency
    -> A2R introduced an explicit rejection variant
```

## Qualification counts

```text
APPLIED records       = 5
REJECTED records      = 2
APPLIED route         = PASS / unchanged A1 projector
REJECTED route        = PASS / explicit rejection view
REPLAY                = PASS
SCOPE EXPANSION       = NONE
```

## Access audit

```text
04A history reads     = 0
population reads      = 0
target reads          = 0
outcome reads         = 0
pair reads            = 0
tokens realized      = 0
arm executions        = 0
D_B / D_C / D_D       = 0
```

## Next state

The system is tightened, but FC-01A still needs a fresh composite closure receipt that explicitly includes this variant repair. No FC-01B work begins until that closure is separately issued.
