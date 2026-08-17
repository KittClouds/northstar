# G9 Dynamic Lift of 04A

This package executes the first qualified real-history attack on an actual 04A information-losing map.

## Qualified result

```text
map = CURRENT_SENTINEL_STATE_V1 -> REDUCED_SENTINEL_GEOMETRY_V1
verdict = UNIVERSAL_BEHAVIORAL_PRESERVATION_FALSIFIED
scope = DECLARED_D_A_ONLY
primary science root = 3844cc9dafe1f54aa54fb501381f16faf3b5c67958bdafc2e54d50325b55b5db
complete run root = a0ab7992bb0bce185185d85e9a4b1186c10b4847059eda220995c3cbbee73478
qualification root = 2e65b1032d43e843129fc6a57fd16d572329cc2c6925265e8dd85d3660ee6ee5
```

The authoritative report is [`seal/G9_CAMPAIGN_FINAL_REPORT.md`](seal/G9_CAMPAIGN_FINAL_REPORT.md). The outer qualification receipt and manifest are:

- `seal/G9_QUALIFICATION_ROOT_RECEIPT.json`
- `seal/qualification_manifest.tsv`

## Rebuild and execute

Use an isolated D: target on Windows:

```powershell
$env:CARGO_TARGET_DIR = 'D:\northstar-g9-target-local'
cargo test --release --manifest-path .\Cargo.toml
cargo clippy --release --all-targets --manifest-path .\Cargo.toml -- -D warnings
cargo run --release --manifest-path .\Cargo.toml -- <mt5-authority-root> <fresh-output-directory>
```

The output directory must be absent or empty. The runner binds and consumes a single-use execution capability before the exact D_A open.

## Permanent limits

- Six other 04A relation surfaces remain `NOT_EVALUABLE` in this lineage.
- Unsearched fibers are not preserved fibers.
- G4's `GrammarStateAndEvent` blind spot remains open.
- No parallax, G10 necessity, market, predictive, economic, or trading authority is claimed.
