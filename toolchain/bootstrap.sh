#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

command -v rustup >/dev/null || { echo "Install rustup from https://rustup.rs/" >&2; exit 1; }
rustup toolchain install 1.99.1 --component clippy --component rustfmt --target x86_64-unknown-linux-gnu
bash toolchain/bootstrap-protoc.sh

echo "Rust, Cargo, protoc versions:"
rustup run 1.99.1 rustc --version --verbose
rustup run 1.99.1 cargo --version
.toolchain/protoc/bin/protoc --version

if [[ "$(node --version 2>/dev/null || true)" != "v26.13.1" ]]; then
  echo "Install Node.js 26.13.1 from the official distribution before running npm ci; see toolchain/versions.toml." >&2
  exit 1
fi
if [[ "$(npm --version 2>/dev/null || true)" != "12.2.2" ]]; then
  echo "Expected npm 12.2.2 with the pinned Node.js toolchain; found $(npm --version 2>/dev/null || true)." >&2
  exit 1
fi
npm ci --ignore-scripts --no-audit --no-fund
