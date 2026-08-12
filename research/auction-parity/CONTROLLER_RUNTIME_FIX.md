# Master controller live-runtime fix

This cut changes operational scheduling and chart ownership only. Auction,
topology, producer, and research-dataset semantics are unchanged.

## Corrected behavior

- Live updates have one owner: `OnTimer`. `OnCalculate` remains the Strategy
  Tester driver but no longer repeats all embedded producer work on every live
  tick.
- Live updates enable the existing unchanged-structure cache, so DBSCAN is not
  rebuilt every timer interval when structural inputs are identical.
- The initial live producer/profile/DBSCAN rebuild is deferred until after
  `OnInit`, preventing the indicator from blocking chart creation. Tester
  initialization remains synchronous to preserve replay semantics.
- Buffer publication fails safely until MT5 has allocated the four indicator
  buffers on the first `OnCalculate`. This closes the initialization and
  timeframe-change array-overrun path.
- Removal disables refreshes, kills the timer, deletes owned objects, and
  redraws the chart before dataset finalization. Slow finalization can no longer
  strand master objects on the chart.
- Startup also removes stale objects belonging to the same instance prefix and
  rejects invalid timeframe enum values before producer construction.
- A reentrancy guard prevents overlapping refresh ownership.

The embedded producers remain headless and do not inspect or mutate visual
producer indicators. Coexistence is obtained by eliminating duplicated live
work and keeping all master-rendered objects inside its instance-specific
prefix.

## Validation

The deployed controller compiled with MetaEditor using 0 errors and 0 warnings.
The auction golden and research-identity harnesses also compile with 0 errors
and 0 warnings. Two older aggregate harnesses remain stale against earlier
API/schema names; their pre-existing failures are unrelated to this runtime
cut.

The canonical source is `mql5/MasterStructureController.mq5`. Its SHA-256 is
identical to the source deployed in the active D0 terminal.
