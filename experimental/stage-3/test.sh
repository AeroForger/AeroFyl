#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
experiment_tmp="$(mktemp -d)"
trap 'rm -rf "$experiment_tmp"' EXIT
bootstrap=(cargo run --quiet -p aerofyl-bootstrap --)

stage3="$experiment_tmp/stage3"
hello="$experiment_tmp/hello"

"${bootstrap[@]}" compile "$repository_root/experimental/stage-3/compiler.fyl" -o "$stage3"
touch "$hello"
chmod +x "$hello"
"$stage3" "$repository_root/examples/hello.fyl" "$hello"
"$hello"

empty_source="$experiment_tmp/empty.fyl"
empty_binary="$experiment_tmp/empty"
: > "$empty_source"
set +e
"$stage3" "$empty_source" "$empty_binary"
status=$?
set -e
if [[ $status -ne 70 ]]; then
    printf 'expected empty source to fail with status 70, got %s\n' "$status" >&2
    exit 1
fi

printf '%s\n' 'Stage 3 compiler passed.'
