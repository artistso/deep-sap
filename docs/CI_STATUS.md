# CI Status and Diagnostic Policy

The bootstrap pull request uses GitHub Actions as the authoritative Rust and WebAssembly build environment.

## Confirmed during bootstrap

- Rust dependencies resolve with the committed `Cargo.lock`.
- `cargo audit` completes without a reported vulnerable dependency.
- Static JavaScript, HTML, JSON, TOML, YAML, shell, release-boundary, and independent numerical checks pass.
- The WebAssembly dependency graph compiles with explicit `getrandom` browser backends.
- Rustfmt is applied by the canonical stable toolchain.

## Failure diagnostics

Native Clippy and Trunk release-build output are retained as workflow artifacts when those gates fail. Fixes must address the exact compiler output; warnings are not suppressed globally.

The pull request remains unmerged until native and WASM compilation, Clippy, tests, dependency audit, release validation, Trunk output, and the compressed-WASM budget pass on the same head commit.
