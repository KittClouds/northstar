# TradeLocker read-only activation

Northstar's TradeLocker cut is deliberately unable to transmit orders. The
transport exposes only JWT authentication plus GET requests for configuration,
accounts, instruments, contracts, sessions, quotes, state, positions, orders,
order history, and executions.

## Protected credential

Create one **Generic Credential** in Windows Credential Manager:

- Internet or network address: `Northstar/TradeLocker/live`
- User name: the TradeLocker account email
- Password: the TradeLocker account password

Northstar reads this entry with `CredReadW`. The email and password never enter
source configuration, durable receipts, the canonical journal, the Ledger, or
logs. Both UTF-8 and Windows UI-style UTF-16 credential blobs are accepted and
the process-memory strings are zeroized on drop.

Do not put TradeLocker credentials in a `.env` file, command line, catalog, or
fixture.

## Non-secret runtime selectors

Set these in the environment that launches Northstar:

```text
NORTHSTAR_TRADELOCKER_SERVER=HEROFX
NORTHSTAR_TRADELOCKER_CREDENTIAL_TARGET=Northstar/TradeLocker/live
```

The credential target is optional because the value above is the compiled
default. Account selection is also optional when exactly one account is
returned. If several accounts exist, set exactly one of:

```text
NORTHSTAR_TRADELOCKER_ACCOUNT_ID=<TradeLocker accountId>
NORTHSTAR_TRADELOCKER_ACC_NUM=<TradeLocker accNum>
```

Northstar rejects ambiguous or missing account selection. `accountId` and
`accNum` are distinct TradeLocker identities.

## Atomic epoch contract

Every publication must contain one coherent account-scoped capture:

1. `/trade/config` layouts and limits.
2. Account selection and account state.
3. All six admitted instruments and their INFO/TRADE routes.
4. All six instrument detail/contracts and session identities.
5. All six quotes.
6. Positions, non-final orders, final order history, and executions.
7. One receive-time window and one configuration fingerprint.

A missing field, crossed quote, short table row, column-layout drift, missing
route, incomplete instrument set, or transport failure rejects the complete
candidate. The prior epoch remains visible as stale. Fund, Systems, venue
truth, and the auto-generated Ledger view publish together in one office
generation only after durable Ledger appends succeed.

Unknown open orders or positions are preserved as venue truth and fail the
account-truth gate. They are never silently adopted as Northstar-owned trades.

## Intentionally absent

- No order placement, modification, cancellation, or position modification.
- No live arming.
- No Nautilus integration.
- No MT5 bridge.
- No strategy, indicator, or statistical-model authority.

The next activation proof is a sanitized live epoch capture after the Generic
Credential exists. Provider payload retention must be reviewed before any raw
response is made durable or promoted to a golden fixture.

## Live activation proof

Verified against the HERO FX live environment on 2026-08-11:

- JWT authentication accepts the provider's documented `201 Created` response.
- The sole account is selected without persisting or displaying either account identifier.
- All six index bindings resolve to distinct TradeLocker instruments with INFO and TRADE routes.
- Instrument metadata is read on the INFO route used by TradeLocker's official client.
- Contract tick tiers are selected against the same epoch's midpoint quote; a null first
  `leftRangeLimit` is treated as the unbounded first tier.
- `/trade/config` rate limits pace repeated reads, with bounded idempotent-GET retry for
  fixed-window `429` responses and no retrying write surface.
- Two consecutive coherent epochs published 6 contracts, 6 quotes, 0 positions,
  0 open orders, 0 executions, and 0 unknown venue objects.
- No raw provider response was retained. Only canonical venue truth and Ledger receipts
  were committed.
