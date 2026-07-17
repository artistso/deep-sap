# DEEP SAP

**An unclassified synthetic ocean sensor-fusion simulation built with Rust, Bevy ECS, and WebAssembly.**

DEEP SAP is a tablet-oriented game and engineering testbed. It generates deterministic synthetic targets, converts them into noisy sensor observations, associates observations into tracks, estimates positions with bearing-only nonlinear least squares, and displays covariance-derived uncertainty.

Public ocean data may update environmental context such as surface temperature, salinity, wave height, and bathymetry. **Public data never creates, identifies, classifies, or locates game targets.**

## Current vertical slice

- Hidden truth layer with deterministic synthetic targets
- Correct knot-to-kilometre motion conversion
- Moving ownship to create a measurable bearing baseline
- Noisy bearing and tonal-frequency observations without target IDs or factions
- Observation-to-track association using angular and spectral gates
- Per-track Gauss-Newton bearing fix
- Per-track covariance and 95% uncertainty ellipse
- Motion prediction derived from estimated fixes, not truth positions
- S Pen Pointer Events bridge for pressure, tilt, hover, and barrel-button state
- Credential-free public environmental dashboard
- Installable PWA shell and runtime cache
- Cloudflare Worker with origin allowlisting, fixed public-provider routes, optional rate limiting, and no credential or token relay
- Native/WASM checks, tests, Clippy, dependency audit, JavaScript parsing, and WASM size enforcement in CI

## Explicit boundaries

This project is:

- A synthetic game/simulation
- Unclassified
- Based on simplified public-domain physical relationships
- Unsuitable for navigation, maritime safety, surveillance, targeting, or operational decision-making

It does not contain real acoustic-signature libraries, patrol data, classified information, or a production ASW capability.

## Architecture

```text
Synthetic truth world
        ↓ sensor model only
Noisy bearing observations
        ↓ gating and association
Independent track hypotheses
        ↓ nonlinear least squares
Estimated position + covariance
        ↓ estimated-state motion
Prediction + player-facing display
```

The tracker and UI do not query hidden target position, faction, or class.

## Run locally

```bash
git clone https://github.com/artistso/deep-sap.git
cd deep-sap
rustup target add wasm32-unknown-unknown
cargo install trunk

cargo test
cargo run
# or
trunk serve --address 0.0.0.0 --port 8080
```

The first dependency resolution should produce `Cargo.lock`; commit that file for reproducible application builds.

## Public data proxy

`cloudflare_worker_proxy.js` exposes only validated fixed-provider routes. Configure:

```text
ALLOWED_ORIGINS=https://deep-sap.soquarky.click,https://artistso.github.io
UPSTREAM_USER_AGENT=deep-sap-public-data-proxy/2.0 contact@example.com
```

This release is credential-free: it exposes only fixed routes to public providers that do not require third-party API keys. Copernicus browser password grants, public token-relay routes, and keyed satellite-provider routes are removed.

## PWA and S Pen

The web shell publishes Pointer Event pressure, tilt, twist, hover, and barrel-button state to the WASM simulation. Browser and device support varies, so unsupported values degrade to zero/default state. Mouse and touch remain supported.

## Data attribution

When GEBCO-derived bathymetry is displayed, preserve the provider attribution and applicable GEBCO terms. GEBCO data must not be used for navigation or safety-at-sea decisions.

## Project status

See [`docs/IMPLEMENTATION_STATUS.md`](docs/IMPLEMENTATION_STATUS.md) for completed work, validation limits, and the next engineering sequence.

## Author

Steven Owens — [soquarky.click](https://soquarky.click) — GitHub: `artistso`

MIT licensed. See `LICENSE` and `SECURITY.md`.
