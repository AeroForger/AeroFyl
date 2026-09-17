#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
experiment_tmp="$(mktemp -d)"
trap 'rm -rf "$experiment_tmp"' EXIT
compiler="$experiment_tmp/aerofyl-stage1-experiment"
bootstrap=(cargo run --quiet -p aerofyl-bootstrap --)

"${bootstrap[@]}" compile "$repository_root/experimental/compiler.fyl" -o "$compiler"

"$compiler" "$repository_root/experimental/fixtures/precedence.afx"
"$compiler" "$repository_root/experimental/fixtures/parentheses.afx"

for rejected in invalid.afx wrong-result.afx; do
    set +e
    "$compiler" "$repository_root/experimental/fixtures/$rejected"
    status=$?
    set -e
    if [[ $status -ne 70 ]]; then
        printf 'expected %s to fail with status 70, got %s\n' "$rejected" "$status" >&2
        exit 1
    fi
done

printf 'Stage 1 experiment passed: Aerofyl compiled, parsed, emitted, and ran bytecode.\n'
