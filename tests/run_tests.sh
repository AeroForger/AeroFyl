#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
compiler=(cargo run --quiet -p aerofyl-bootstrap --)

for source in "$repository_root"/examples/*.fyl \
              "$repository_root"/compiler/frontend/*.fyl \
              "$repository_root"/tests/compile-pass/*.fyl \
              "$repository_root"/stress-tests/*/*.fyl; do
    "${compiler[@]}" "$source" >/dev/null
done

for source in "$repository_root"/tests/compile-fail/*.fyl; do
    if "${compiler[@]}" "$source" >/dev/null 2>&1; then
        printf 'expected compilation failure: %s\n' "$source" >&2
        exit 1
    fi
done

printf 'Aerofyl fixture checks passed.\n'
