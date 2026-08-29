#!/usr/bin/env bash
set -euo pipefail

cargo test --locked
cargo build --release --locked --target x86_64-pc-windows-gnu

artifact="target/x86_64-pc-windows-gnu/release/zcode-fresh-reset.exe"
test -s "$artifact"
file "$artifact" | grep -q 'PE32+'
sha256sum "$artifact"
