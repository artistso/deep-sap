# DEEP SAP

Repository-facing summary: see [`README.md`](README.md).

DEEP SAP is an unclassified synthetic ocean sensor-fusion simulation. It uses deterministic synthetic targets and may use public environmental observations solely as scenario context. It is not an operational detection, surveillance, navigation, or targeting system.

## Required merge gates

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo check --target wasm32-unknown-unknown --no-default-features
trunk build --release
python tools/math_reference_check.py
```

See `SECURITY.md`, `docs/IMPLEMENTATION_STATUS.md`, and `GAME_SPEC.md` before extending external-data or tracking code.
