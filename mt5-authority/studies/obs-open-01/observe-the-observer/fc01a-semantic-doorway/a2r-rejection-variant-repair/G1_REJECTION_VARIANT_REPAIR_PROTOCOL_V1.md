# FC01A-A2R — Rejection-Variant ABI Repair

This is a new repair lineage. It does not mutate A1 Attempt 1 or the failed A2 result.

The repair introduces an explicit return variant:

```text
APPLIED  -> frozen G1_FC01A_ABI_V1 -> unchanged A1 projector
REJECTED -> explicit rejection view -> no state/emission/time fabrication
```

Rejected G1 calls are not coerced into applied semantic outputs. Their G1 rejection status is passed through with authorized context, input provenance, and lineage only.

The repair remains synthetic-only and does not close FC-01A by itself.
