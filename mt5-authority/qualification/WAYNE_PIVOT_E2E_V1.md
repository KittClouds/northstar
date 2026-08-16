# WAYNE_PIVOT_E2E_V1 qualification

Status: `QUALIFIED_WITHIN_DECLARED_TEST_SCOPE`

This is a new standalone Wayne/Wyatt pivot observer. It does not modify the
installed `WaynesPivots.mq5`, `MasterStructureController.mq5`, or any
Trading.com terminal artifact.

## Surface

- Indicator: `mt5-authority/WaynePivotStateMachine.mq5`
- Core and logger: `mt5-authority/WaynePivotStateMachineCore.mqh`
- Formula/grammar fixture: `qualification/WaynePivotStateMachineCoreTests.mq5`
- Buffer probe: `qualification/WaynePivotStateMachineBufferProbe.mq5`
- Configuration: `InpCountPeriods`, `InpTimePeriod`, `InpInstanceID`,
  interaction buffer, acceptance bars, future preview, and an explicit tester
  finalization boundary.
- No Fibot levels or Fibot inputs exist in the new surface.

The standard pivot formulas are preserved:

```text
PP = (high + low + close) / 3
R1 = 2*PP - low             S1 = 2*PP - high
R2 = PP + (high - low)      S2 = PP - (high - low)
R3 = 2*PP + high - 2*low     S3 = 2*PP - 2*high + low
M0 = (S2 + S3) / 2           M1 = (S1 + S2) / 2
M2 = (PP + S1) / 2           M3 = (PP + R1) / 2
M4 = (R1 + R2) / 2           M5 = (R2 + R3) / 2
```

All M levels are unconditional. M2, M3, M4, and M5 cannot be switched off.
PP, all standard pivots, and both zones are also machine-published.

The two zones are explicit structural bands:

```text
M1_S2_ZONE = [S2, M1]
M4_R2_ZONE = [M4, R2]
```

They use the same state grammar as point levels. No buy/sell or execution
meaning is assigned to them.

## State and event grammar

Each point and zone has a typed lifecycle:

```text
FRESH -> TOUCHED -> TESTED -> ACCEPTED
                    \-> REJECTED
                    \-> BROKEN -> RECLAIMED
```

Events are append-only and include CREATE, TOUCH, RETEST, ACCEPTANCE,
REJECTION, BREAK, RECLAIM, ROLLOVER, SNAPSHOT, and FINALIZE. Interaction
decisions use completed chart bars and a declared point-buffer tolerance.

Pivot geometry comes only from a completed source period. The optional current
period preview is marked developing, drawn for inspection, and never enters the
interaction state machine or authoritative frame snapshots.

## End-to-end receipts

The logger writes append-only Common Files TSV artifacts per symbol/timeframe/
instance:

```text
WaynePivotE2E_*_frames.tsv
WaynePivotE2E_*_levels.tsv
WaynePivotE2E_*_zones.tsv
WaynePivotE2E_*_events.tsv
WaynePivotE2E_*_receipt.tsv
```

The receipt is opened as `OPEN` and closes at the declared tester boundary (or
normal deinitialization). The final row binds frame, level, zone, event counts
and a deterministic 64-bit replay root. The root is a deterministic checksum,
not a cryptographic chain-of-custody primitive.

## Compile receipts

Compiler: standard `C:\Program Files\MetaTrader 5\MetaEditor64.exe`, build
6116 family. The Trading.com compiler was not invoked.

| Target | Errors | Warnings | Compile time |
|---|---:|---:|---:|
| `WaynePivotStateMachine.mq5` | 0 | 0 | 1064 ms |
| `WaynePivotStateMachineCoreTests.mq5` | 0 | 0 | 585 ms |
| `WaynePivotStateMachineBufferProbe.mq5` | 0 | 0 | 503 ms |

Final source/binary hashes:

```text
WaynePivotStateMachine.mq5      916996E5C7DCF24F072AB08053EBED663861581A40ECB84600A13899097F2B83
WaynePivotStateMachineCore.mqh  5DDD512F4F0B297DE70966A917FE66553176FADDC3A09797EC91D6C92D876C14
WaynePivotStateMachine.ex5      85BCB9F53A80DFC91BA919B0DB79629A3326375105BB5F81116CB326EBF56C9D
```

## Strategy Tester receipts

Declared environment:

```text
terminal       isolated portable standard MetaTrader 5
build          6116
history        MetaQuotes-Demo cached history
symbol         EURUSD
chart          M5
source period  D1 (MQL5 enum 16408)
model          OHLC bar states
window         2026-04-01 through 2026-04-05
live trading   disabled
DLL imports    disabled
```

### Core fixture

The hand-computable fixture passed all checks:

```text
formula_ok=1
zone_geometry_ok=1
mandatory_ok=1
touch=1 accepted=1 broken=1 reclaimed=1
lower_zone=1/1/1/1
upper_zone=1/1/1/1
outcome=PASS
```

The tester generated 288 bars / 1101 ticks and passed in 0.056 seconds.

### Full indicator replay

Two independent runs both sealed:

```text
frames             864
level snapshots   12960
zone snapshots     1728
events              183
tester time       2.128 s / 2.123 s
receipt state      CLOSED
replay root        10745703185817818515
```

The root and all semantic counts were identical across the two rebuilds.
Observed focused event coverage in the latest run included:

```text
PP          TOUCH 1  ACCEPTANCE 1  BREAK 4  RECLAIM 3
M2          TOUCH 2  ACCEPTANCE 1  BREAK 7  RECLAIM 5
M4          TOUCH 1  BREAK 9         RECLAIM 8
M1/S2 zone  TOUCH 1  ACCEPTANCE 1  BREAK 8  RECLAIM 7
M4/R2 zone  TOUCH 1  BREAK 6         RECLAIM 5
```

### Buffer probe

The probe exercised all 17 buffers through `iCustom`/`CopyBuffer`:

```text
bars                 864
copy_failures          0
invalid_values         0
zero_active             0
min_active / max      17 / 17
mandatory M2/M3/M4/M5/PP  PASS
zone boundaries         PASS
tester outcome          PASS (2.093 s)
```

## Authority checks and gaps

The installed original authorities remain unchanged:

```text
standard WaynesPivots.mq5       2E28B7C32B4193499B8D5FC7D3E1C735FEB952206C2E3C85AA18F281A8FB22B0
Trading.com WaynesPivots.mq5    2E28B7C32B4193499B8D5FC7D3E1C735FEB952206C2E3C85AA18F281A8FB22B0
MasterStructureController.mq5  227768519E0145679AA25A332418FD32B7F05A45BC8C99BCB75283377B33EF5D
```

The new indicator is installed only under the standard terminal's
`Indicators/NorthstarWayneE2E` directory.

Not claimed by this qualification:

```text
live-runtime equivalence
universal MT5 determinism
financial or predictive meaning
execution/trading authority
persistent cross-run state learning
cryptographic tamper resistance
```

The next useful gate, if needed, is a separate receipt-lifecycle qualification
for recovery/orphan handling. The pivot geometry and interaction grammar are
otherwise ready to replace the Wayne contribution at the controller boundary.
