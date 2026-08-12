# Northstar canonical data plane

## Status

The Phase II-A foundation is implemented under `src/data_plane`. The original
visual prototype remains fixture-backed, while the separate fixture-free
`northstar-operating` composition root consumes immutable production snapshots
and renders missing authorities without substitution.

This phase contains no Nautilus dependencies, strategies, indicators,
Markov/HSMM models, credential storage, networking, live arming, or order
transmission.

## Authority invariant

For each `(domain, entity, effective interval)` there is exactly one active
authority. `CanonicalCatalog` rejects overlapping authority leases.

- Massive is reference-index-value evidence, not venue bid/ask authority.
- TradeLocker is venue quote, contract, account, order, fill, and position
  evidence.
- One official adapter is active for each macro `SeriesId`; mirrors are standby
  sources until an explicit authority change.
- Northstar is the sole authority for canonical storage, journal order, custom
  bars, replay, derived snapshots, and export manifests.
- Nautilus will eventually consume Northstar data; it will not become source or
  replay authority.

## Layers

```text
provider bytes
    -> L0 receipt frame
    -> versioned pure decoder
    -> L1 canonical batch
    -> durable commit
    -> one live/replay batch interface
    -> L2 deterministic processors
    -> immutable GPUI snapshots
```

### L0 raw receipts

`ReceiptStoreWriter` persists exact provider payloads plus sanitized endpoint
metadata. Frames carry source, stream, receipt and time identities, lengths,
CRC32 checksums, BLAKE3 hashes, and an explicit commit footer.

Open recovers only an incomplete tail. A committed checksum mismatch is
corruption and fails closed. `PersonalDisplayOnly` data is rejected before raw
retention; the caller must supply a source contract that permits storage.

### L1 canonical journal

Canonical records are fixed 128-byte POD values with no strings, pointers,
reference counting, or heap ownership. Each 80-byte event header preserves:

```text
journal sequence
receipt identity
source event identity
event time
received time
effective time
provider sequence, when available
entity/source/stream/schema/kind/flags
```

The journal is a single-writer, framed append stream. A batch becomes visible
only after its payload and commit footer have been flushed and synchronized.
Duplicate `(source, stream, source_event_id)` values fail before write.

Sealed journals are read through `memmap2`. Batch payloads are validated once
and exposed as zero-copy `&[CanonicalEvent]` slices.

### L2 derived truth

L2 is rebuildable. The first processor is the custom bar engine:

- input: committed `IndexValue` events only;
- anchor: materialized main reference-session open;
- windows: half-open `[open, close)`;
- outputs: M4, M20, H2, and H4;
- missing windows: absent, never forward-filled;
- session tail: explicitly marked partial;
- volume: not invented for index-value feeds;
- late input: a new bar revision, never an invisible overwrite;
- OHLC order: event time, then journal sequence as deterministic tie-breaker.

The current chart `Bar` and `BarSeries` remain compact `f32` presentation lanes.
They are produced from L2 snapshots and are not persisted as canonical truth.

### Operating snapshot bridge

The first production-facing L2 bridge is now implemented under `src/operating`.
It consumes the same committed `EventBatchRef` in live and replay, projects
reference values, venue availability, and canonical bars, then publishes
segmented immutable Desk snapshots through an atomic snapshot store.

The store uses lock-free reads, a unique single-writer publisher, bounded
coalesced wakeups, stable domain generations, and explicit unavailable states.
Intrabar value updates reuse unchanged series arcs. Bar history is sealed in
256-record immutable chunks, so the hot append path copies at most the active
tail rather than full history. Fixtures are now an explicit Cargo feature and
the desktop library compiles without them. The production GPUI subscriber has
no idle timer: a cancellation-aware blocking receiver coalesces domain notices
and schedules one UI invalidation for the latest immutable snapshot.

The native operating canvas consumes those chunks in place. It binary-searches
the visible time interval, traverses only intersecting bars, and derives logical
coordinates from canonical timeframe slots. Missing windows are therefore
painted as gaps instead of being compressed away. The replay lab uses the same
publisher, projector, snapshot, and canvas path as live data; only its explicit
composition root and `REPLAY` badge differ.

## Time contract

```text
ts_event      when the provider says the observation happened
ts_received   when Northstar completely received the bytes
ts_effective  earliest causal eligibility in this dataset
```

`ObservedLive` events cannot have `ts_effective < ts_received`. Historical
imports may use verified publication time only when the dataset manifest says
`VerifiedPublication`; estimated imports remain explicitly marked.

Exact replay follows journal order. It never sorts late events by event time.
As-of replay filters by effective time while preserving the surviving journal
order, preventing later macro vintages from leaking into earlier state.

## Identity and catalog

Provider text is cold metadata. Stable dense IDs own hot records:

```text
SourceId / StreamId
InstrumentId / SeriesId
ReceiptId / BatchId / JournalSequence
CatalogVersion / CalendarId / DerivationVersion
```

`ProviderSymbolBinding` records price scale, effective interval, source,
instrument, symbol, and catalog version. Runtime fixture labels such as
`I:DAX` are not promoted to production mappings without a dated discovery
receipt.

`VenueInstrumentContract` distinguishes environment, account, tradable
instrument, INFO route, TRADE route, session, quantity terms, price tick, and
precision. Overlapping account/instrument contract versions fail closed.

## Provider boundary

`SourceAdapter` performs external I/O and emits `RawReceipt` values.
`CanonicalDecoder` is pure: it transforms a receipt view into a canonical batch
and is testable entirely from golden fixtures.

The implemented Massive value decoder accepts the documented `V` object/array
shape only. It validates the event kind, converts Unix milliseconds with
checked arithmetic, resolves catalog identity and authority, converts prices to
fixed point, derives receipt-independent event identity, and emits `IndexValue`
rather than `VenueQuote`.

`MassiveDeskPipeline` now commits the exact L0 receipt, decodes it, commits the
L1 batch, projects Desk state, and publishes through `OfficeSnapshotPort` in one
supervised composition seam. Decode failure preserves the receipt but cannot
mutate canonical or visible state.

There is deliberately no network client or production ticker map yet. Massive
non-display, durable-retention, derived-work, and internal-export rights must be
confirmed before those paths can be enabled.

TradeLocker decoding remains fixture-gated because responses depend on
account-scoped `/trade/config`, instrument discovery, routes, and dynamic
columns. The HeroFX LIVE desktop account and visible venue schemas were audited
on 2026-08-11, but sanitized REST fixtures have not yet been captured. The
public REST contract does not establish the prototype's former `SyncEnd`
assumption; production readiness is a Northstar-owned coherent epoch commit.

## Publication and replay

`LivePublisher` performs:

```text
append batch
    -> flush and sync
    -> receive CommitReceipt
    -> publish EventBatchRef
```

`ReplayEngine` maps the same journal and publishes the same `EventBatchRef`.
Consumers have no live/replay branch. Replay reports the consumed sequence
range and BLAKE3 input hash.

## Export

`DatasetManifest` publishes atomically and records schema, catalog, calendar,
and derivation versions; replay quality; effective-time bounds; source contract
versions; and segment paths, lengths, layers, and hashes.

Every source is checked against an explicit `ExportScope`. Display-only data
cannot enter an internal research export. Licensed non-display and
internal-restricted data cannot be marked for redistribution.

## Verification

The automated suite covers stable POD sizes, time-travel rejection, macro
vintages, duplicate authority/event rejection, venue contract agreement, exact
L0 mmap round trips, incomplete-tail recovery, checksums, sequence validation,
session/calendar behavior, custom bars, late revisions, as-of replay,
license-aware export, Massive value decoding, end-to-end live/replay bar
identity, and a durable documented Massive `V` receipt reaching the GPUI
snapshot port with its journal provenance intact.

The manual release performance gate appends and replays 100,000 canonical
events from an isolated target directory. The `desktop` feature remains enabled
by default for the app, while `--no-default-features` compiles the canonical
core without linking GPUI so storage/replay measurements are not UI link times.

## Next authorized vertical slices

1. Capture a licensed Massive catalog and six-index golden value receipts.
2. Capture HeroFX LIVE TradeLocker `/trade/config`, instrument, quote, history,
   session, account, order, fill, and position fixtures without storing secrets
   or enabling order transmission.
3. Add typed TradeLocker decoders and a coherent reconciliation snapshot.
4. Add BLS/BEA/Census/FRED US macro receipts, including an original/revised
   publication pair.
5. Connect the licensed Massive transport to `MassiveDeskPipeline`; canonical
   snapshot painting is implemented and awaits licensed live receipts.
6. Add regional adapters one at a time with original, revision, malformed,
   duplicate, late, and missing fixtures.

Nautilus remains deferred until these data-plane gates are complete.
