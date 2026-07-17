# Engineering Roadmap

## P0 — Compile and reproduce

- Resolve all Bevy 0.15 compiler findings in CI.
- Commit `Cargo.lock` and stop ignoring it.
- Require formatting, native/WASM Clippy, tests, dependency audit, and release build.
- Add a browser smoke test that loads the WASM canvas and records console failures.

## P1 — Complete the game loop

- Replace automatic association-only play with player-created evidence marks.
- Add tentative, confirmed, coasting, and dropped track states.
- Separate localization quality from classification probability.
- Add false alarms, contact splitting/merging, and track ambiguity.
- Add mission objectives and after-action scoring.

## P2 — Rendering and tablet input

- Replace gizmo history with a ring-buffer texture waterfall.
- Batch bearing history and uncertainty geometry.
- Add adaptive quality tiers for 60/90/120 Hz displays.
- Capture coalesced pen samples, pressure, tilt, twist, hover, and barrel state.
- Profile WebGL first; evaluate WebGPU only after a measured baseline.

## P3 — Mathematical validation

- Add property tests for angle wrapping, ray geometry, covariance positivity, and unit conversion.
- Add Monte Carlo localization scenarios across baseline, SNR, and bearing-noise ranges.
- Add normalized innovation/residual gates for association.
- Introduce a constant-velocity EKF or UKF only after the batch estimator is covered by tests.
- Record numerical tolerances and reject unobservable geometries explicitly.

## P4 — Public environmental context

- Add typed provider adapters one at a time.
- Preserve field-level provenance, units, freshness, and quality.
- Cache a licensed offline scenario snapshot.
- Display attribution and data-age indicators.
- Keep external observations structurally unable to instantiate or classify targets.

## Release rule

No feature is described as live, real-time, compliant, or production-ready until automated evidence demonstrates the exact claim.
