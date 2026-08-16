# VOLPRO_KITT_ADAPTIVE_V1 qualification

Status: `QUALIFIED_WITHIN_DECLARED_TEST_SCOPE`

This receipt qualifies a new, standalone adaptive descendant of
`VolProKitt2.mq5`. It does not modify, replace, or promote either installed
copy of the original indicator.

## Authority boundary

- New indicator: `mt5-authority/VolProKittAdaptive.mq5`
- New deterministic core: `mt5-authority/VolProKittAdaptiveCore.mqh`
- Buffer probe: `mt5-authority/qualification/VolProKittAdaptiveBufferProbe.mq5`
- Core battery: `mt5-authority/qualification/VolProKittAdaptiveCoreTests.mq5`
- Only exposed indicator input: `InpCalcTF`
- Machine-facing surface: ten hidden `INDICATOR_DATA` POC buffers, keyed by
  stable adaptive object ID
- Diagnostic surface: `KVA_COMMIT` and periodic `KVA_FRAME` journal records
- Presentation surface: chart objects

The original installed authorities were byte-checked after construction:

```text
VolProKitt2.mq5 bytes  = 26116
VolProKitt2.mq5 sha256 = 7C011368F3D0CD7DD7EE0A2706F37B4065976B591672816773C1BBCF399104AF
standard MT5 copy      = unchanged
Trading.com copy       = unchanged and not used for compile or test
```

## Adaptive contract

The V1 core derives the following from closed market bars:

- structural lookback from bounded distribution similarity candidates;
- cluster count from deterministic candidate scoring over K=2..8;
- structural velocity horizon from short/long range geometry;
- volume-profile resolution independently for each cluster;
- persistent object identity within a run from price distance, range overlap,
  and mass similarity;
- profile changes through confidence gating and three-observation hysteresis.

Candidate lookbacks are 64, 96, 128, 192, 256, 384, and 512 bars. K-means
has a deterministic initialization, convergence exit, and a 100-iteration
safety ceiling. Volume-profile rows are bounded to 8..80 per object. Market
volume uses real volume when present and tick volume otherwise.

The adaptation commits only on a newly closed calculation-timeframe bar. It
does not mutate the active profile on every tick.

## Compile qualification

Compiler: standard `C:\Program Files\MetaTrader 5\MetaEditor64.exe`, build
6116 family. The Trading.com compiler was not invoked.

| Target | Errors | Warnings | Compile time |
|---|---:|---:|---:|
| `VolProKittAdaptive.mq5` | 0 | 0 | 1474 ms |
| `VolProKittAdaptiveCoreTests.mq5` | 0 | 0 | 643 ms |
| `VolProKittAdaptiveBufferProbe.mq5` | 0 | 0 | 432 ms |

Authoritative source and binary digests:

| Artifact | SHA-256 |
|---|---|
| `VolProKittAdaptive.mq5` | `13A406F0D9AA39E338B34B5EE003526C167DCC6833109575FC563D2A39BA63C2` |
| `VolProKittAdaptiveCore.mqh` | `8318E1C777B9E3B2E5B4993E04BCA7CD0D396BC4857CA0643854085B5520AAF0` |
| compiled indicator EX5 | `4D348FF3244685BAAA983C11EFCCFA0B5AAAD4C2C100B30C490CE180238DA5C1` |

## Strategy Tester qualification

Declared environment:

```text
terminal                  standard MetaTrader 5 portable test copy
terminal build            6116
broker/history authority  MetaQuotes-Demo cached history
symbol                    EURUSD
timeframe                 M5
model                     OHLC bar states
interval                  2026-04-01 through 2026-04-05
live trading              disabled
DLL imports               disabled
visual mode               disabled
```

### Direct indicator replay

Two isolated executions of the definitive binary each produced:

```text
ticks                     3311
bars                      864
test outcome              passed
second execution time     0.369 s
semantic diagnostic rows 72
profile commits           45
periodic frame records    27
final profile             L64 / K2 / V12 / generation 45
semantic trace SHA-256    3C9B01978816C44958CD37E0DC4F167EF7A96E25839AD3BEC2AFB0EB5CB5DFC5
replay comparison         byte-identical after log-prefix normalization
```

The observed EURUSD fixture admitted lookbacks 64, 96, and 128 and velocity
horizons from 4 through 18. It selected K=2 throughout this fixture. That is an
observation about this frozen input, not a hard-coded cluster count.

### Deterministic core battery

The synthetic tri-modal fixture selected K=3 with score `0.891149`. The stable
identity fixture preserved object IDs 4 and 5 and assigned new ID 0 to a new
object. The profile hysteresis fixture required three matching observations and
advanced the generation to 2. The EA completed 288 bars / 1101 ticks and the
tester passed in 0.052 s.

### Buffer probe

The `iCustom` / `CopyBuffer` probe exercised every buffer at every new bar and
sealed at the declared boundary:

```text
bars                       864
copy failures              0
invalid numeric values     0
zero-active after warmup   0
minimum active buffers     2
maximum active buffers     2
test outcome               passed
test time                  0.378 s
```

## Qualification matrix

```text
STANDALONE_DESCENDANT         QUALIFIED
ORIGINAL_SOURCE_IMMUTABILITY QUALIFIED
STANDARD_MT5_COMPILE         QUALIFIED
DETERMINISTIC_CORE           QUALIFIED
DYNAMIC_CLUSTER_COUNT        QUALIFIED_BY_SYNTHETIC_FIXTURE
IN_RUN_PROFILE_HYSTERESIS    QUALIFIED
STABLE_IN_RUN_OBJECT_IDS     QUALIFIED
DIRECT_TESTER_EXECUTION      QUALIFIED
BUFFER_COPY_CONTRACT         QUALIFIED
DETERMINISTIC_REPLAY         QUALIFIED_WITHIN_DECLARED_SCOPE
JOURNAL_DIAGNOSTICS          QUALIFIED
PERSISTENT_SYMBOL_MEMORY     GAP
CROSS_RUN_LEARNING           GAP
SEALED_TSV_RECEIPT_WRITER    GAP
MULTI_INSTRUMENT_VALIDATION  GAP
LIVE_RUNTIME_EQUIVALENCE     NOT_CLAIMED
PREDICTIVE_OR_FINANCIAL_EDGE NOT_CLAIMED
```

## Narrow conclusion

`VolProKittAdaptive` is qualified as a deterministic, self-calibrating market
geometry instrument in the declared standard-MT5 portable tester. Its adaptive
profile, stable in-run object identities, diagnostics, and ten POC buffers are
replayable under that contract.

This qualification does not establish profitable prediction, scientific
validity across instruments, live-runtime equivalence, or durable learning
between separate executions. Persistent symbol/timeframe memory should be a
separate descendant gate with its own versioned state, lineage, and replay
contract.
