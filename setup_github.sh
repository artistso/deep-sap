#!/usr/bin/env bash
set -euo pipefail

GH_USER="${GH_USER:-artistso}"
REPO_NAME="${REPO_NAME:-deep-sap}"

cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo check --target wasm32-unknown-unknown --no-default-features
python tools/math_reference_check.py
trunk build --release

git add .
git commit -m "feat: stabilize synthetic sensor-fusion vertical slice" || true

if command -v gh >/dev/null 2>&1; then
  gh repo view "$GH_USER/$REPO_NAME" >/dev/null 2>&1 || \
    gh repo create "$GH_USER/$REPO_NAME" --public --source=. --remote=origin \
      --description "Unclassified synthetic ocean sensor-fusion simulation"
  git push -u origin main
else
  echo "GitHub CLI not installed; validated local build without creating a remote."
fi
