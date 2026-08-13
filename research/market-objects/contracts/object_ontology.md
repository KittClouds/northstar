# RG3 object ontology

## Compression

Lifecycle states retain the RCM V2 numeric contract. A compression terminates
as `CLOSED_UP`, `CLOSED_DOWN`, `EXPIRED`, or an explicit censor. A reclaimed
frontier crossing remains within the same compression lineage and never opens
an expansion.

## Expansion

An expansion opens only on `RANGE_CLOSED_UP` or `RANGE_CLOSED_DOWN`. It freezes
the origin compression ID, terminal close, seed frontier, containment frontier,
midpoint, and origin ATR. It terminates by causal handoff to a newly confirmed
compression or by explicit right censoring.

Stable expansion states:

```text
IDLE=0 ACTIVE=1 HANDOFF=2 CENSORED=3
```

Stable expansion events:

```text
NONE=0 CLOSED_UP=1 CLOSED_DOWN=2 COMPRESSION_EXPIRED=3
EXPANSION_HANDOFF=4 HORIZON_CENSORED=5 TEST_END_CENSORED=6
SHUTDOWN_CENSORED=7 DATA_GAP_CENSORED=8
```

## Availability planes

- `ORIGIN`: known when an object begins.
- `PREFIX`: known through the current closed bar.
- `TERMINAL`: known only when the object terminates.
- `POST_TERMINAL`: known only afterward.

Prefix views must mechanically exclude terminal and post-terminal columns.

## Neutrality

The ontology contains no entry, exit, target, stop, profit, loss, win, signal,
or position state. A forecast is a separately receipted observation and cannot
alter object identity or lifecycle.
