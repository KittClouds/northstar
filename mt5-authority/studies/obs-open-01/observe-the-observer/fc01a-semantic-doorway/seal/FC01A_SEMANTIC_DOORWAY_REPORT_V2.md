# FC-01A Semantic Doorway — A0 Sovereignty Seal

## Result

```text
EXECUTION_STATE = SEALED
QUESTION_STATUS  = CLOSED
RESULT           = FC01A_NOT_EVALUABLE
DISPOSITION      = HOLD
```

FC-01A is an eligibility and authority-qualification layer only. It does not alter FC-00, the five scientific identities, or any earlier fossil. No population was opened.

## A0 — G1 semantic sovereignty

The executable architecture is fixed as:

```text
G0_CANONICAL_HISTORY
        |
        v
G1_EXACT_SEMANTIC_KERNEL
        |
        v
FC01A_TYPED_DOWNSTREAM_PROJECTION
        |
        +--> G5_PRESENTABILITY_VIEW
        +--> G6_CORRESPONDENCE_PRIMITIVE_VIEW
        +--> G7_CERTIFICATE_LINEAGE_VIEW
        +--> G8_VERIFIER_INPUT_VIEW
```

G0 is upstream input authority. G1 is the sole semantic constructor. FC-01A consumes `G1_SEMANTIC_OUTPUT_V1`; it does not construct semantic meaning from G0 history.

The source firewall is explicit:

```text
G1_SEMANTIC_CONSTRUCTION_SOURCE = SEALED_G1_IMPLEMENTATION_ONLY

FC01A_CUSTOM_SEMANTIC_TRANSITIONS = 0
FC01A_CUSTOM_LIFECYCLE_RULES     = 0
FC01A_CUSTOM_GENEALOGY_RULES     = 0
FC01A_CUSTOM_REJECTION_RULES     = 0
FC01A_CUSTOM_TEMPORAL_INTERPRETATION = 0
```

FC-01A may orchestrate a sealed G1 call, translate G1 output into named consumer views, canonicalize a declared projection, and emit view roots. It may not reinterpret G1 state, reproduce lifecycle or genealogy logic, reconstruct rejection or time semantics, or add semantic fields not derived and authorized by G1.

If the sealed G1 machinery cannot expose a required surface, FC-01A must return `NOT_EVALUABLE`. Reimplementing a small piece in Python is forbidden.

## Minimal doorway and consumer projections

The canonical doorway object is intentionally minimal and contains only G1-derived fields:

```text
g1_authority_root
context_identity
input_identity
transition_status
state_projection
ordered_emissions
knowledge_time_projection
rejection_status
semantic_lineage_root
```

The doorway then emits four separately named, separately rooted projections. Each has an explicit allowlist; no consumer may implicitly inherit another consumer's fields. The common doorway is therefore one semantic throat without becoming one giant union surface.

## Provenance firewall

Qualification fixtures are typed by provenance, not by filename:

```text
SYNTHETIC_NONPOPULATION
CONTRACT_ONLY
POPULATION_DERIVED_CONTENT
POPULATION_DERIVED_METADATA
```

Only the first two are allowed here. `SEALED` is not treated as synonymous with `NONPOPULATION`; a sealed artifact may still descend from protected population content. Hashes may establish ancestry without opening content.

## Proof scope

The contract records proof methods separately from fixture evidence. Determinism, field noninterference, byte replay, and consumer-view disjointness remain unproven at this layer. Passing synthetic fixtures would qualify behavior on those fixtures; it would not establish an unrestricted theorem. Universal claims are scoped to the declared G1 function domain only, and all-canonical-input byte identity is explicitly not claimed.

## Access audit

```text
population admission                  = 0
scientific content reads               = 0
population-derived content reads      = 0
population-derived metadata reads     = 0
pair cohort instantiated               = FALSE
question tokens realized               = 0
G8 real-history certificates           = 0
arm executions                         = 0
D_B / D_C / D_D reads                  = 0
```

No pair was chosen, no finite question box was instantiated, no token was realized, and no real-history certificate was produced.

## Why the result is not evaluable

The schema and lineage are now frozen, but the doorway implementation has not been qualified. In particular, the A0 source firewall has been declared and bound but not implementation-verified; determinism, field noninterference, byte replay, and consumer-view roots have not been earned. Therefore the result is:

```text
FC01A_NOT_EVALUABLE
DISPOSITION = HOLD
```

The next lawful work is to qualify a doorway that invokes the sealed G1 implementation and performs projection only. If G1 cannot expose the required fields, the doorway remains `NOT_EVALUABLE`; no parallel semantic implementation is authorized.

## Maximum authority

```text
FC01A_SCHEMA_AND_LINEAGE_CONTRACT_ONLY
```

This seal earns no G8 certificate authority, no G6 comparison authority, no pair-cohort authority, no FC-02 authority, and no empirical claim about 04A.
