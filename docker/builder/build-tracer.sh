#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=cli

echo "Building tracer docker images"
echo "PROFILE: $PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Build all the rust binaries
cargo build --locked --profile=$PROFILE \
    -p aptos-tracer \
    "$@"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos-tracer
)

mkdir dist

for BIN in "${BINS[@]}"; do
    cp $CARGO_TARGET_DIR/$PROFILE/$BIN dist/$BIN
done

