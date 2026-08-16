# Finite-box instance binding protocol

This gate binds one instance, once. It freezes the qualified rule root, its
qualification authority, the V2 binding authority, and one authorized input
vector. The rule is invoked exactly once; there are no retries, alternate
configurations, fallbacks, clamps, rounds, defaults, or manual overrides.

The execution records three independent verdicts:

```text
RULE_APPLICATION
FINITE_BOX_SCHEMA_VALIDATION
FINITE_BOX_AUTHORITY_BINDING
```

The qualified rule executed successfully, but its output retained all ten
parameter classes as unbound. Schema validation therefore failed and authority
binding is `NOT_EVALUABLE`. No instance is created. This result is preserved;
no repair is attempted.
