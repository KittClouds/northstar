# Phase 13 one-shot result

Phase 13 reached the irreversible consumption gate but did not score any
candidate. The canonical evaluator wrote `CONSUMED_BEFORE_SCORING`, then
aborted while parsing the sealed run receipts because those JSON files carry a
UTF-8 BOM. The evaluator's `serde_json::from_slice` path does not accept that
prefix.

The one-shot contract forbids retrying after consumption. Consequently this is
an infrastructure failure, not a weak or failed model result. No AUC,
calibration, degradation, graduation, or candidate outcome exists for this
holdout.

The twelve collected runs remain independently sealed and usable only in a
future, newly preauthorized evaluation generation. Their market outcomes were
not inspected during this run.
