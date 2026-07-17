# Implementation Status — Stabilization Vertical Slice

## Completed in this pass

### Security

- Removed public Copernicus token generation and query-string tokens.
- Removed browser credential storage and credential-entry demonstrations.
- Replaced wildcard CORS with an origin allowlist.
- Added route and coordinate validation.
- Stopped returning complete upstream URLs, stack traces, secrets, or personal/home coordinates.
- Removed all third-party keyed-provider routes from the release Worker.
- Added optional deployment rate limiting and `no-store` error responses.

### Simulation boundary

- Added an explicit hidden truth marker.
- Replaced truth-attached contacts with independent noisy observation entities.
- Removed target IDs, truth positions, factions, and classes from observations.
- Removed tracker and UI reads of truth position/faction.
- Standardized navigation bearings to 0° north and 90° east.

### Mathematics

- Corrected knots to kilometres per second: `1.852 / 3600`.
- Added circular bearing distance across 0°/360°.
- Required both bearing rays to face the intersection.
- Rejected insufficient observer baseline.
- Added per-track Gauss-Newton weighted bearing localization.
- Added covariance inversion and 95% chi-square uncertainty ellipses.
- Derived velocity and 30-second prediction from estimated fixes.
- Replaced raw-time f32 polynomial normal equations with normalized f64 QR fitting.

### Data integration

- Added asynchronous task entities that are polled and applied.
- Added field-level provenance and quality.
- Prevented one provider sample from overwriting unrelated fields.
- Kept deterministic offline environmental fallback values.

### Web/PWA

- Linked a relative-scope manifest.
- Added a service worker and runtime cache.
- Added S Pen Pointer Event pressure, tilt, twist, hover, and button bridge.
- Added wake-lock reacquisition after visibility changes.

### CI

- Enforced native and WASM checks.
- Removed ignored WASM Clippy failures.
- Added unit tests, dependency audit, JavaScript parsing, release build, artifact upload, and compressed WASM budget.

## Validation limit in the generated package

The code was statically reconstructed. JavaScript, JSON, HTML, YAML, shell, and independent numerical checks are executed by the validation script. The generation environment did not contain a Rust toolchain and could not reach `rustup.rs`, so Rust compilation, formatting, Clippy, and Rust test execution must be completed by CI or a Rust-enabled workstation.

## Next engineering sequence

1. Resolve any Bevy 0.15 API/compiler findings from CI.
2. Commit the generated `Cargo.lock`.
3. Add deterministic integration tests that run the ECS schedule for a complete mission.
4. Replace gizmo waterfall lines with a texture-backed waterfall.
5. Add explicit player mark/confirm/delete interactions.
6. Add track lifecycle states: tentative, confirmed, coasting, dropped.
7. Add replay and after-action review with truth revealed only after mission completion.
8. Profile WebGL on the target tablet, then evaluate a WebGPU feature branch.
