# G1-R — Executable Authority Reconstruction Qualification

This package qualifies a new executable descendant of the sealed G1 source/semantic authority. It does not recover or re-identify the historical G1 binary.

The descendant depends on the sealed `obs-open-04a-g1` crate and calls only its qualified `init` and `step` functions. New code is limited to input decoding, invocation orchestration, output capture, canonical serialization, and qualification receipts.

The qualification corpus is synthetic and non-population. Historical binaries are corroboration-only and are not used to confer identity.

## Nonclaims

- Historical G1 binary identity is not recovered.
- G1 source semantics are not reimplemented or modified.
- No population, target, outcome, pair, or optical execution authority is earned.
- FC-01A remains a separate downstream qualification.
