# OTO runtime parameter identity probe

Status: `SOURCE_IMPLEMENTED_RUNTIME_EXECUTION_BLOCKED_PENDING_LINEAGE`

This descendant probe exists only to recover the effective parameter vectors of
the exact V2.00 and V2.10 handles instantiated with the INST-01 argument lists.
It does not alter either fossil, read indicator buffers, read price arrays, join
market data, or authorize a parameter sweep.

The authoritative runtime receipt must preserve the ordered `MqlParam` index,
type, integer/double/string storage values, handle role, symbol, timeframe,
period seconds, digits, terminal company/name/build, connection/tester mode,
program build, specimen hashes, and parent INST-01 root. These are an
environment bundle, not a claim that every field is semantically relevant.

Execution closes only when two independent runs produce semantically identical
ordered vectors under the same declared runtime. Physical capture timestamps are
metadata and are excluded from deterministic vector identity.

The available Trading.com MetaEditor, terminal, and tester are build 6094. The
parent INST-01 authority records build 6106, and no build-6106 terminal was found
in the bounded Program Files scan. Compilation under 6094 qualifies the probe
source only; it does not establish execution parity with the admitted INST-01
runtime. Execution must wait for either the exact admitted runtime or a separately
authorized runtime-transport qualification.

Until then:

```text
EFFECTIVE_RUNTIME_PARAMETER_BINDING = NOT_EVALUABLE_PENDING_RUNTIME_INTROSPECTION
PARAMETER_SWEEP_AUTHORITY            = NONE
```
