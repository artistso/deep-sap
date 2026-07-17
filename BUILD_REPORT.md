# DEEP SAP v0.2 Stabilization Build Report

Generated: 2026-07-17

## Delivered vertical slice

This package converts the original presentation prototype into an unclassified, deterministic synthetic ocean sensor-fusion simulation. Public environmental providers affect environmental context only; they do not generate, identify, classify, or locate synthetic targets.

### Core reconstruction

- Hidden synthetic truth entities separated from sensor observations.
- Observation records contain no target ID, truth position, faction, or class.
- Moving ownship creates a real spatial baseline for bearing-only localization.
- Correct target and ownship motion conversion: $1\text{ knot}=1.852/3600\text{ km/s}$.
- Circular bearing distance and forward-ray validation.
- Per-track weighted nonlinear least-squares position fixes.
- Covariance-derived 95% uncertainty ellipses.
- Estimated-state velocity and 30-second prediction; no truth-fed prediction.
- Numerically normalized `f64` QR polynomial fitting.
- Sonar display, pen bearing, and camera now share the moving ownship reference frame.

### Platform and interaction

- Bevy 0.15.3 pinned with the correct `webgl2` feature.
- Relative-path Trunk build for root domains or GitHub project pages.
- PWA manifest, service worker, offline shell, and runtime asset cache.
- Pointer Events bridge for pressure, tilt, twist, hover, and barrel-button state.
- Wake-lock reacquisition after returning to the app.

### Security boundary

- Removed credential-bearing and defense-named experimental provider modules from the shipping source tree.
- Removed browser credential storage and public OAuth/token relay routes.
- Cloudflare Worker exposes fixed credential-free provider routes only.
- Exact-origin CORS allowlist, strict route validation, safe response headers, no-store errors, and optional rate limiting.
- Removed personal/home coordinates and secret-bearing upstream URL disclosure.

### Engineering controls

- Native format, Clippy, unit-test, and audit jobs.
- WASM check, WASM Clippy, release build, compressed-size budget, and artifact upload.
- GitHub Pages workflow with repository-relative public path.
- Static release validator for JSON, TOML, YAML, JavaScript, inline HTML scripts, shell, Rust delimiter balance, secret patterns, and release boundaries.
- Independent numerical reference test suite.

## Executed validation

`python3 tools/validate_release.py`:

```text
PASS JSON
PASS TOML
PASS YAML
PASS JavaScript/HTML
PASS shell
PASS Rust delimiter balance
PASS release boundary
PASS independent numerical reference checks
```

Independent numerical outputs:

```text
PASS knot_conversion 0.308666666667 km/60s
PASS circular_bearing 359° vs 1° = 2°
PASS bearing_fix estimate=(7.950842,11.914410) error=0.098702 km
PASS covariance ellipse95=(2.589129,0.091892) km
PASS observability_guard stationary baseline rejected by release threshold
```

Source scale in this package:

- Rust: 2660 lines
- Main web shell/demos/proxy/service worker: 778 lines

## Validation limitation

The execution container did not contain `rustc`, `cargo`, or `rustfmt`, and DNS resolution for Rust/package hosts failed. Therefore this report does **not** claim that Cargo compilation, Rust formatting, Clippy, Rust unit tests, or the Trunk release build ran locally.

The included CI workflow is the authoritative build gate and runs:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo check --target wasm32-unknown-unknown --no-default-features
cargo clippy --target wasm32-unknown-unknown --no-default-features -- -D warnings
cargo audit
trunk build --release --public-url ./
```

The first successful dependency resolution should generate `Cargo.lock`; commit it for reproducible application builds.

## Next implementation tranche

1. Resolve any compiler/API findings reported by the first CI run.
2. Add ECS integration tests covering a complete seeded mission.
3. Replace gizmo waterfall marks with a texture-backed spectrogram history.
4. Add player mark, confirm, merge, split, coast, and drop actions.
5. Add mission scoring and after-action replay, revealing truth only after completion.
6. Profile WebGL on the Galaxy Tab target before introducing a WebGPU branch.
