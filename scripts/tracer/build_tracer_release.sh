#!/bin/bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

# Build and package a release binary for aptos-tracer.
# Example:
# scripts/tracer/build_tracer_release.sh Linux aptos-tracer-v0.1.0 false true

set -euo pipefail

NAME="aptos-tracer"
CRATE_NAME="aptos-tracer"
CARGO_PATH="aptos-move/aptos-tracer/Cargo.toml"
PLATFORM_NAME="${1:-}"
RELEASE_TAG="${2:-}"
SKIP_CHECKS="${3:-false}"
COMPATIBILITY_MODE="${4:-false}"
RELEASE_REPO="${5:-${GITHUB_REPOSITORY:-sentioxyz/aptos-core}}"

if [[ -z "$PLATFORM_NAME" || -z "$RELEASE_TAG" ]]; then
  echo "Usage: $0 <platform_name> <release_tag> [skip_checks] [compatibility_mode] [release_repo]"
  exit 1
fi

ARCH="$(uname -m)"
OS="$(uname -s)"
VERSION="$(sed -n '/^\w*version = /p' "$CARGO_PATH" | sed 's/^.*=[ ]*"//g' | sed 's/".*$//g')"

if [[ "$SKIP_CHECKS" != "true" ]]; then
  if ! [[ "$RELEASE_TAG" =~ ^aptos-tracer-v([0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
    echo "$RELEASE_TAG is malformed, must match '^aptos-tracer-v[0-9]+\\.[0-9]+\\.[0-9]+$'"
    exit 2
  fi

  EXPECTED_VERSION="${BASH_REMATCH[1]}"
  if [[ "$EXPECTED_VERSION" != "$VERSION" ]]; then
    echo "Wanted to release for $EXPECTED_VERSION, but Cargo.toml says the version is $VERSION"
    exit 3
  fi

  if curl -s --stderr /dev/null --output /dev/null --head -f "https://github.com/${RELEASE_REPO}/releases/download/${RELEASE_TAG}/${NAME}-${VERSION}-${PLATFORM_NAME}-${ARCH}.zip"; then
    echo "${RELEASE_TAG} already exists with ${NAME}-${VERSION}-${PLATFORM_NAME}-${ARCH}.zip"
    exit 4
  fi
else
  echo "WARNING: Skipping version checks!"
fi

echo "Building $NAME $VERSION for ${OS}-${PLATFORM_NAME} on ${ARCH}"
if [[ "$COMPATIBILITY_MODE" == "true" ]]; then
  RUSTFLAGS="-C target-cpu=generic --cfg tokio_unstable -C target-feature=-sse4.2,-avx" cargo build --locked -p "$CRATE_NAME" --profile cli
else
  cargo build --locked -p "$CRATE_NAME" --profile cli
fi

cd target/cli

ZIP_NAME="${NAME}-${VERSION}-${PLATFORM_NAME}-${ARCH}.zip"
echo "Zipping release: ${ZIP_NAME}"
zip "$ZIP_NAME" "$CRATE_NAME"
mv "$ZIP_NAME" ../..
