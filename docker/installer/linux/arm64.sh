#!/usr/bin/env sh

set -eu

export CC_aarch64_unknown_linux_musl=aarch64-linux-gnu-gcc
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc
cargo build --release --target=aarch64-unknown-linux-musl

cp target/aarch64-unknown-linux-musl/release/vesta .
