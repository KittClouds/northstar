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

The source-only DoF topology is sealed separately at:

```text
5dae6f027af87de766742391f4c96665d062fbd5aef3cfecf9f2592b561f4bf1
```

It contains 58 identified joints and 173 structural dependency edges. It earns
no claim that those joints are independent dimensions. Dynamic experiments,
market reads, and outcome reads remain zero.

The Trading.com build-6094 toolchain is quarantined for OTO. The guarded compile
path accepts only the explicitly hash-bound MetaTrader-origin editor. The parent
runtime was build 6106; the same installation origin is now build 6116, so any
runtime execution still requires an exact-runtime recovery or a separately
qualified 6106-to-6116 transport bridge.
