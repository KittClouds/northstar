# OSV1 Branch Point

Date: 2026-08-14

This note records the point where Northstar's first closed observational system
stops being a linear gate sequence and becomes the immutable parent of multiple
research branches.

## Scientific and release identity

```text
OSV1 scientific root
6e18e10fc0fb3571aa21b4e20105f783dafe2872812d217d672ab5e959bb890d

Northstar release tag
osv1-v1.0.0

Northstar release commit
9ced26cb3a7ae6aeefe6c1a2f63a493e28ceca48

OSV1 release pack SHA-256
502d55a37e0592901906f3aa46f6298fb3f5d0614a4150b9e61fecb8d480d980
```

The scientific root identifies OSV1. The Git tag and commit identify the
repository container that carries the release. Neither identity substitutes for
the other.

## MT5 authority companion

The MT5-side research controls, campaign definitions, schemas, receipts, reports,
and source artifacts are frozen separately:

```text
repository
https://github.com/KittClouds/northstar-mt5-authority

tag
mt5-authority-v1.0.0

commit
390ea55e9834ea739996c3e533f1e0025dcfcea9
```

The bulky MT5 observation payload remains local by explicit policy:

```text
storage policy     D_DRIVE_ONLY_NOT_REMOTE
local root         D:/northstar-mt5-corpus/osv1-mt5-freeze-v1
payload files      590
payload bytes      11986996702
manifest SHA-256   58d13f99985a3256c00a0b2a4ed895cd8dc778db969dae6f20da8e54f18cf768
remote storage     false
```

The MT5 repository publishes only the content-addressed manifest and receipt for
those bytes. Local corpus availability is therefore distinct from scientific
identity and from Git availability.

## Authority boundary

- Gate 16.6 closed OSV1 science.
- Gate 16.7 packaged, cold-rehydrated, verified, and exposed OSV1 without
  regenerating or changing its science.
- `COLD_REHYDRATION` and `COLD_SOURCE_REGENERATION` remain distinct claims.
- Typed absence, censoring, insufficiency, provenance, and `source_kind` survive
  all consumer boundaries without coercion.
- Downstream consumers are read-only with respect to OSV1 and may emit only
  content-addressed sidecars bound to the parent root.
- No observational result is silently promoted into predictive, economic, or
  trading authority.
- The unopened confirmation authority remains unopened unless a future protocol
  explicitly authorizes it.

## Branch topology

```text
OSV1
 ├── OBS-*   observational transport, geometry, loss, symmetry, and new authority
 ├── PHX-*   Phoenix visualization and graph-analytic sidecars
 └── FIN-*   financial research downstream of observational authority
```

There is no Gate 16.8 implied by this note. Future work begins on an explicitly
named branch with its own protocol, authority inputs, receipts, and limits. OSV1
remains the frozen parent rather than a mutable notebook.
