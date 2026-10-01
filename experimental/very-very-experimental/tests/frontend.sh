#!/usr/bin/env bash
set -euo pipefail
task_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$task_root"
task_tmp=$(mktemp -d /tmp/aerofyl-frontend-tests.XXXXXX)
trap 'rm -rf -- "$task_tmp"' EXIT
bootstrap=${AEROFYL_BOOTSTRAP:-"$task_root/target/release/aerofyl-bootstrap"}
timeout 120 "$bootstrap" compile experimental/compiler/parser_harness.fyl -o "$task_tmp/parser"
timeout 10 "$task_tmp/parser" >"$task_tmp/positive.out"
test "$(cat "$task_tmp/positive.out")" = 'frontend tests passed'
count=0
for fixture in experimental/tests/frontend-invalid/*.fyl; do
    set +e
    timeout 5 "$task_tmp/parser" "$fixture" >"$task_tmp/stdout" 2>"$task_tmp/stderr"
    status=$?
    set -e
    test "$status" = 1
    test ! -s "$task_tmp/stdout"
    rg -q ':[0-9]+:[0-9]+: error:' "$task_tmp/stderr"
    count=$((count + 1))
done
{
    printf 'public void main() { int value = '
    for ((index=0; index<110; index++)); do printf '('; done
    printf '1'
    for ((index=0; index<110; index++)); do printf ')'; done
    printf '; }\n'
} >"$task_tmp/deep.fyl"
set +e
timeout 5 "$task_tmp/parser" "$task_tmp/deep.fyl" >"$task_tmp/stdout" 2>"$task_tmp/stderr"
status=$?
set -e
test "$status" = 1
rg -q 'nesting exceeds' "$task_tmp/stderr"
{
    printf 'public void main() { int'
    for ((index=0; index<110; index++)); do printf '[1]'; done
    printf ' value = []; }\n'
} >"$task_tmp/deep-type.fyl"
set +e
timeout 5 "$task_tmp/parser" "$task_tmp/deep-type.fyl" >"$task_tmp/stdout" 2>"$task_tmp/stderr"
status=$?
set -e
test "$status" = 1
rg -q 'array type nesting exceeds' "$task_tmp/stderr"
printf 'frontend: structural assertions and %s invalid fixtures plus expression/type nesting guards passed\n' "$count"
