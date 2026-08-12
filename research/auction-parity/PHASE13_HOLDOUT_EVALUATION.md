# Phase 13: one-shot RG2 holdout evaluation

Phase 13 is fail-closed. The committed preauthorization package freezes the candidate family, model bytes and semantics, feature schema, development references, reservation allowlist, metrics, uncertainty procedure, multiplicity correction, decision rules, and evaluator-source identity.

The current state is intentionally:

```text
PREAUTHORIZED_NOT_AUTHORIZED
holdout_authorized = false
```

No reserved window is collected or scored by the freeze command.

## State progression

```text
PREAUTHORIZED_NOT_AUTHORIZED
  -> explicit authorize command
AUTHORIZED_UNCONSUMED
  -> validate exact 12-run sealed bundle
  -> atomically create consumption receipt
CONSUMED_BEFORE_SCORING
  -> score once
COMPLETE or ABORT
```

Authorization requires the literal confirmation `AUTHORIZE_ONE_SHOT_RG2_HOLDOUT`. The token is created with create-new semantics. Evaluation refuses a protocol produced by different evaluator source. The consumption directory is created atomically before any model score is calculated, and a second evaluation cannot reuse it.

## Holdout bundle contract

The evaluator accepts a `NORTHSTAR_RG2_HOLDOUT_BUNDLE_V1` JSON manifest containing exactly the twelve preauthorized reservations. Each run must bind:

- deterministic `run_key`;
- canonical instrument and broker symbol;
- data source;
- `VERIFICATION_A` or `VERIFICATION_B`;
- exact reserved timestamps;
- a content-hashed run receipt whose JSON contains the same `run_key`.

Each candidate has one content-hashed TSV of causally frozen, model-ready raw observations. The evaluator—not the bundle producer—applies the frozen model preprocessor and model. Required identity columns are:

```text
target
run_key
episode_id
attempt_id
canonical_instrument
holdout_id
target_censored
target_label
```

The remaining columns are the raw numeric and categorical fields named by the frozen preprocessor. `\\N` is the only missing value. A censored row must have `target_label=\\N`; an uncensored row must have a Boolean label.

## Frozen inference and gates

- Primary metric: paired per-observation log-loss improvement over the frozen development prevalence.
- Censoring: retained in eligible/censoring denominators, excluded from score metrics.
- Uncertainty: deterministic 10,000-repetition run-cluster bootstrap, seed `117011`, 95% interval.
- Family correction: one-sided Holm correction across all four candidates.
- Reliability: fixed-width probability buckets `[0.0, 0.1, ..., 1.0]`.
- Calibration: diagnostic intercept and slope only; no recalibration is permitted.
- Temporal transport: positive primary point delta is required in both reserved calendar blocks.
- Outcomes: candidates graduate independently as `PASS`, `WEAK_RESEARCH_ONLY`, or `FAIL`.
- Sparse support is a candidate `FAIL`, not an infrastructure abort.

The exact candidate-specific thresholds are part of `artifacts/phase13-preauthorization/preauthorization.json` and therefore part of the protocol hash.

## Commands

Freeze is repeatable metadata work. Authorization and evaluation are deliberately separate commands:

```text
northstar-holdout freeze ...
northstar-holdout authorize --protocol ... --output ... --confirmation AUTHORIZE_ONE_SHOT_RG2_HOLDOUT
northstar-holdout evaluate --protocol ... --authorization ... --bundle ... --state ... --report ...
```

Do not issue the authorization command until the one-shot evaluation is intentionally approved. Do not edit a model, feature, threshold, calibration rule, or evaluator after authorization.
