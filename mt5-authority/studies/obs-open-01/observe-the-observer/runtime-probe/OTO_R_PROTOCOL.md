# OTO-R — Runtime Lineage Resolution

`OTO-R` is a pre-dynamic gate. It resolves the runtime identity required before
effective parameter binding or any observer DoF work can be evaluated.

The probe emits one `RUNTIME_INSTANTIATION_BUNDLE_V1` containing:

```text
terminal: company, name, build, connection/tester mode
chart: symbol, timeframe, period_seconds, digits
V200: specimen hashes, effective ordered MqlParam vector
V210: specimen hashes, effective ordered MqlParam vector
probe: parent INST-01 root and program/compile identity
```

Only two exits are lawful:

```text
PATH_A_EXACT_RUNTIME_RECOVERED
    runtime build and required executable identities match INST-01;
    execute metadata probe; then qualify effective vectors.

PATH_B_TRANSPORT_QUALIFICATION_REQUIRED
    exact runtime is unavailable; freeze a narrow semantic transport contract
    before probe execution, or halt dynamic observer metrology.
```

Source compilation on another build is not runtime equivalence. No parameter
sweep, state experiment, or outcome access is authorized at either exit until
the effective V200/V210 vectors are sealed.

The Trading.com build-6094 editor is quarantined for this lane. OTO compilation
must pass the exact path/version/hash guard in `compile_oto_probe.ps1`. This is a
logical authority quarantine: it does not delete, rename, stop, or alter any
installed terminal.
