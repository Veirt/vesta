#!/usr/bin/env sh

set -eu

cargo build --release --target=x86_64-unknown-linux-musl

cp target/x86_64-unknown-linux-musl/release/vesta .
