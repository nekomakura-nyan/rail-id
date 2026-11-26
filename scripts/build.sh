#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

# The default build has no TPM dependency. `--tpm` enables the standard
# profile; `--no-tpm` is retained as a compatibility no-op.
TPM=0
case "${1:-}" in
  --tpm) TPM=1 ;;
  --no-tpm) TPM=0 ;;
esac

# A plain string is intentional: it is empty or two simple words. Arrays under
# `set -u` are not portable to the bash 3.2 shipped with macOS.
FEATURE_FLAGS=""
if [[ "$TPM" -eq 1 ]]; then
  FEATURE_FLAGS="--features tpm"
fi

echo "== cargo fmt --check =="
cargo fmt --check

echo "== cargo clippy =="
cargo clippy --all-targets $FEATURE_FLAGS -- -D warnings

echo "== cargo test =="
cargo test $FEATURE_FLAGS

echo "== cargo test (railid-tpm) =="
cargo test --manifest-path railid-tpm/Cargo.toml

echo "== cargo build --release =="
cargo build --release $FEATURE_FLAGS

echo "== cargo doc =="
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps $FEATURE_FLAGS

echo "railid checks completed."
