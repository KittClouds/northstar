# Gate 16.7 — OSV1 Release, Rehydration, and Consumer Boundary

Gate 16.7 is release engineering. It creates no observation, watcher,
representation, distance, finding, interpretation, market object, parameter,
model, economic statement, or trading authority.

Its governing distinction is:

```text
COLD_REHYDRATION
released authority -> verify and reassemble released identity

COLD_SOURCE_REGENERATION
source authority -> execute derivation -> reproduce identity
```

The unqualified term `cold rebuild` is forbidden. This release requires cold
rehydration. Cold source regeneration remains `NOT_EVALUABLE` because the
external MT5 source capsule is absent. A future source capsule would be
adjunct regeneration authority and could not change the OSV1 scientific root.

## Frozen scientific identity

```text
OSV1 root
6e18e10fc0fb3571aa21b4e20105f783dafe2872812d217d672ab5e959bb890d

Gate 16.6 closure commit
96a4069dd202cb0ddd935bc7c43ca855228b76c5
```

Git identifies the repository container. The OSV1 root identifies the
science. Neither substitutes for the other.

## Release payload

`osv1_release.pack.zst` is a deterministic, content-addressed archive. It uses
a fixed packed table, canonical relative member paths, per-member SHA-256,
per-member Zstandard frames, and no timestamps. The reader memory-maps the
container, validates member bounds and paths, and decompresses only requested
members.

The payload contains the Gate 16.6 closure and the consumer-facing Gate 15
through Gate 16.5 authorities for objects, lineages, representations,
distances, neighborhoods, representation loss, findings, insufficiencies, and
provenance. It also carries the machine artifacts required by the two-branch
ancestry DAG.

Absolute members, parent traversal, duplicate members, missing files, source
hash drift, and undeclared authority are rejected.

## Consumer boundary

Northstar exports immutable authority. Phoenix or any other consumer may:

```text
READ
FILTER
PROJECT
VISUALIZE
DERIVE_SIDECAR
```

It may not mutate authority, rewrite object identity, reclassify findings,
promote insufficiency, alter Theta0, replace provenance, or claim economic or
trading authority.

All downstream products use `PHOENIX_DERIVED` and bind the parent OSV1 root.
Sidecars must be written outside the payload directory.

## Typed epistemic states

The consumer contract transports epistemic states as tagged enum objects. It
does not represent required states as nullable strings or scalar sentinels.

```text
NULL
CENSORED
NOT_APPLICABLE
NOT_EVALUABLE
INSUFFICIENT_SUPPORT
OPEN
FROZEN_UNOPENED
SOURCE_RECOVERED
NOT_RUN_BOUND
```

The Rust consumer rejects `null`, empty string, zero, false, `unknown`, and
unknown enum variants. It verifies all nine states against released machine
artifacts. The source-recovered projection preserves the 84 authoritative
`RECOVERED_FROM_COLLECTION_ENVIRONMENT` Theta0 rows and their exact
`NOT_RUN_BOUND_NO_EX5_OR_EXPANDED_MASTER_CONFIG_IN_RUN_RECEIPT` state.

## Cold verification

The cold verifier must run from a clean tracked checkout after the original
`gate16-6-prep` directory has been made inaccessible. It builds the isolated
consumer into a fresh target directory, reads authority only from the release
pack, rehydrates into an external directory, and writes proof sidecars outside
the payload.

The verifier records declared reads, undeclared reads, absolute-resolution
attempts, root identity, pack identity, payload-tree identity before and after,
typed-state results, and source-mutation count.

The release tag is created only after this proof passes. The annotated tag
records both the scientific root and the repository commit.
