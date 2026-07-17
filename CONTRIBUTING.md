# Contributing

DEEP SAP is an unclassified synthetic sensor-fusion simulation.

## Required engineering boundaries

1. Tracking systems may consume observations and prior estimates only.
2. Hidden truth components may be read only by scenario motion, sensor generation, and isolated scoring tests.
3. Public environmental feeds must never be described as direct target detections.
4. Browser code must not accept or store private provider credentials.
5. Every mathematical change needs deterministic tests, including degenerate geometry and units.

## Local checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo check --target wasm32-unknown-unknown --no-default-features
node --check cloudflare_worker_proxy.js
node --check static/service-worker.js
trunk build --release
```

## Adding an environmental provider

Return fields with provider, observation timestamp, fetch timestamp, and quality. Merge only the fields the provider actually supplies. Never use “last sample wins” across unrelated environmental variables.
