#!/usr/bin/env bash
set -euo pipefail

readonly VERSION="36.2"
readonly SHA256="121f6c7afe1d4d0e3ea6aab9432038599250134cbf4474cb1167d2c7decd4278"
readonly URL="https://github.com/protocolbuffers/protobuf/releases/download/v${VERSION}/protoc-${VERSION}-linux-x86_64.zip"
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly DEST="${ROOT}/.toolchain/protoc"
readonly WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "Pinned protoc bootstrap supports Linux x86_64 only; use a verified platform-specific release artifact." >&2
  exit 2
fi

mkdir -p "${DEST}"
curl --fail --location --silent --show-error "${URL}" -o "${WORK}/protoc.zip"
echo "${SHA256}  ${WORK}/protoc.zip" | sha256sum --check --status
unzip -q "${WORK}/protoc.zip" -d "${DEST}"
"${DEST}/bin/protoc" --version
