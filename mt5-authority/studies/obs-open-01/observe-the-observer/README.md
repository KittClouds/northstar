# OBSERVE-THE-OBSERVER

This lane studies the manufactured V2.10 observer and its capture apparatus.
It is parallel to the paused G0-G8 behavioral-qualification compound and is not
G9.

The first seal answers only static metrology questions:

```text
observer = explicit inputs
         + ambient platform coordinates
         + hard-coded semantic choices
         + state architecture
         + publication surface
```

Build and verify with the dedicated Rust tool:

```powershell
cargo build --release --manifest-path .\Cargo.toml --target-dir D:\northstar-target-oto
D:\northstar-target-oto\release\obs-open-observe-the-observer.exe build <repo> <out>
D:\northstar-target-oto\release\obs-open-observe-the-observer.exe verify <seal>
```

Current static root:

```text
145fe2f99c75e1214f6cf38b75e83538f4cfef46c5f04b1edfa7ba87b0cd3c8d
```

The runtime probe is a descendant metadata instrument. It reads effective
indicator parameters only. Its source compiles with zero errors and warnings,
but runtime vector execution is deliberately still pending. No parameter sweep
is authorized until that gate closes.
