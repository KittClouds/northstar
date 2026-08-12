# Durable Ledger V1

Ledger is Northstar's local institutional memory. It stores machine evidence
and operator-authored context behind one append-only contract, but it does not
copy the market-data journal. Canonical sequence ranges, venue identities, and
later order/fill receipts are links into their owning authorities.

## Implemented boundary

```text
GPUI quick composer / canonical and provider reducers
                    |
          bounded command channel
                    |
        one Ledger writer thread
                    |
       +------------+-------------+
       |                          |
176-byte hot header       checksummed cold body
ledger.hot                ledger.body
       +------------+-------------+
                    |
             mmap read model
                    |
     bounded immutable LedgerSnapshot
                    |
  Trade Book / Timeline / case inspector
```

The production root is `%LOCALAPPDATA%\Northstar\ledger-v1`. Tests and visual
QA may override it with `NORTHSTAR_DATA_DIR`; the normal executable never uses
that override unless the operator explicitly sets it.

## Hot record

`LedgerEventHeader` is a padding-free 176-byte POD record containing:

- stable entry, case, parent, correlation, source-event, and actor identities;
- event, receive, and recorded timestamps;
- optional canonical sequence range;
- instrument and account identities;
- body offset, length, and BLAKE3 hash;
- schema, kind, status, actor kind, and flags.

Provider text, notes, review prose, and attachments never enter the hot record.
The current body pack accepts arbitrary bytes and validates length and BLAKE3
before publication. Attachment content-addressing is a later Ledger slice.

## Commit and recovery

One writer owns both files. An append proceeds in causal order:

1. validate identities, time order, canonical range, actor semantics, and the
   parent/case relationship;
2. append the cold frame, payload, and commit marker, then sync it;
3. append the hot frame and commit marker, then sync it;
4. update in-memory idempotence/case indexes;
5. project and atomically publish the next immutable Ledger snapshot.

The cold-first order prevents a visible header from referring to an
uncommitted body. Restart truncates incomplete tails. A fully committed cold
body without a hot commit is an orphan from an interrupted append and is
truncated to the highest verified hot reference. A hot reference with a
missing or corrupt body fails closed.

Every append requires a 128-bit `SourceEventKey`. Retrying the same automatic
event or operator command returns the existing entry without writing a second
record. Amendments and redactions require an existing parent in the same trade
case and always create a new record.

## Snapshot and redraw contract

The projector scans durable headers once at startup and then consumes only new
commits. It publishes:

- total entries, distinct trade cases, machine/operator counts, and incidents;
- case summaries sorted by most recent activity;
- at most 256 newest timeline rows, newest first;
- short UTF-8 previews only for visible rows.

Cold bodies are not loaded during ordinary scrolling. Ledger publication uses
`DomainMask::LEDGER`, so a note cannot invalidate Desk, Macro, Fund, or Systems.
GPUI clears the composer after queue acceptance, then changes the status to
`Durably committed` only after the writer publishes the committed snapshot.

## Automatic material-event contract

`AutomaticLedgerEventCommand` is the provider-neutral boundary for machine
evidence. A source must provide a stable 128-bit event key, actor authority,
event/status kind, event and receive times, optional canonical range,
instrument/account identity, and a cold evidence body. Provider JSON and venue
objects never cross this boundary.

The reducer enforces:

- operator kinds cannot be forged by automatic sources;
- decisions, orders, fills, positions, and P&L require either an existing trade
  case or a stable correlation identity;
- the first correlated lifecycle event allocates one case and later events reuse
  it, including after restart;
- repeated source keys return the prior entry without writing or publishing;
- one canonical macro batch becomes one operational receipt, never one Ledger
  copy per observation;
- every macro/positioning receipt links the exact first and last canonical
  sequences and preserves correction count in its evidence body.

Startup walks committed canonical macro batches and idempotently reconciles any
missing Ledger receipts before the first snapshot publishes. Live BLS, CFTC TFF,
and Eurostat commits use the same reducer. CFTC receipts have a distinct
`Positioning` event kind; neither positioning nor macro backfill fabricates a
trade case.

## Current limits

The working V1 supports typed Plan, Observation, Intervention, and Review
entries; manual trade-case creation; append-only amendments and redactions;
automatic canonical macro/positioning receipts; and the normalized lifecycle
socket for future venue evidence. Events are not fabricated before their owning
sources are connected.

Plans and observations may open a case. Interventions and reviews require an
existing case so an action or outcome cannot float without context. The GPUI
composer keeps drafts local and publishes only through the bounded command
channel. Timeline selection supplies the explicit parent for an amendment or
redaction. The writer verifies that the parent exists, belongs to the same case,
and was authored by an operator. Provider, venue, and Northstar machine evidence
cannot be altered through the operator surface. A mutation always appends a new
linked receipt and leaves the original body and hot record byte-identical.

Still pending:

- durable draft recovery, content-addressed files, and attachment manifests;
- TradeLocker order/fill/position correlation after sanitized REST fixture
  capture; the LIVE desktop account and venue table schemas are now verified;
- sealed mmap segments plus rebuilt bitmap/text indexes at large scale;
- Review Queue, Calendar, Analytics, export, and verified backup/restore.

The multi-order/partial-fill exit test therefore remains open until real venue
fixtures exist. The storage, restart, idempotence, and manual-note cuts are
implemented and tested now.

## Verified automatic-reducer evidence

An isolated network-disabled migration on 2026-08-10 opened a journal containing
2,273 canonical events and appended exactly three machine receipts:

```text
Eurostat   sequence    1..=72      72 macro observations
CFTC TFF   sequence   73..=2152  2080 positioning observations
BLS        sequence 2153..=2273   121 macro observations
```

Ledger grew from empty 64-byte headers to `ledger.hot=688` and
`ledger.body=580`. A second process reopened the same root and left all four
files byte-identical. The final Ledger hashes were:

```text
ledger.hot   2206FF52317890FD238D62496CF3406D542A1AF5EA5C330172488B5B0674470D
ledger.body  D2C71BA168E35446697741B3C39430162EAB4820C70761F76DECA91DDC783572
```

Automated verification passed 57 core tests, 87 desktop tests, integration
smokes, formatting, and strict all-target/all-feature Clippy.

## Verified typed-operator evidence

The release-mode projector processed 100,000 operator entries in 19 ms while
retaining exactly 256 visible rows. This is a bounded projection gate rather
than a disk-throughput claim; durable appends deliberately include file syncs.

An isolated GPUI run then exercised the visible composer against the same
single-writer path used in production:

```text
ENTRY 000004  Plan       accepted
ENTRY 000005  Amendment  parent=000004 accepted
ENTRY 000006  Redaction  parent=000004 redacted
```

After restart the UI reconstructed 6 entries, 1 case, 3 machine receipts, and 3
operator receipts. The original Plan remained visible beside both linked
mutations. The resulting QA journal was:

```text
ledger.hot   1312 bytes  171ED779959064675BB24840D4718FCB3FF359F497BA3DD632B7F158AD59396F
ledger.body  1007 bytes  C2D7A2DE6116F5E4A251BDF731BE750B76399A5AD04AAFB5A6F6B66F355D621B
```

The promoted release executable SHA-256 is
`8C739C031ADF6A2ACA53C126D688BA6691982C6D95541FEF00CD631E920BD041`.
