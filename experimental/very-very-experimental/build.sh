#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
output=${1:-/tmp/aerofyl-experimental}
cd -- "$repo"
cargo build --release --offline -p aerofyl-bootstrap
timeout 180 "$repo/target/release/aerofyl-bootstrap" compile "$repo/experimental/compiler/main.fyl" -o "$output"
