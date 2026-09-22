#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "$0")/../.." && pwd)"
output="${TMPDIR:-/tmp}/aerofyl-stage3-lexer-test"

cd "$repository_root/experimental/stage-3/frontend"
cargo run -p aerofyl-bootstrap --manifest-path "$repository_root/Cargo.toml" -- \
  compile "$repository_root/experimental/stage-3/frontend/lexer_harness.fyl" -o "$output"
"$output"
