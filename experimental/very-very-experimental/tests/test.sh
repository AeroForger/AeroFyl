#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
work=$(mktemp -d /tmp/aerofyl-experimental-tests.XXXXXX)
trap 'rm -rf -- "$work"' EXIT
compiler=${AEROFYL_EXPERIMENTAL:-$work/compiler}
oracle="$repo/target/release/aerofyl-bootstrap"
if [[ -z ${AEROFYL_EXPERIMENTAL:-} ]]; then
    "$repo/experimental/build.sh" "$compiler"
fi
passes=0
rejects=0
traps=0
run_binary() {
    local binary=$1 source=$2 prefix=$3
    local -a arguments=()
    if [[ -f ${source%.fyl}.args ]]; then
        mapfile -t arguments < "${source%.fyl}.args"
    fi
    if [[ $source == */pass/filesystem.fyl ]]; then
        arguments=("$work/file.bin" "$work/not-present" "$work")
    elif [[ $source == */trap/filesystem_*.fyl ]]; then
        arguments=("$work/not-present-parent/file")
    fi
    local input=/dev/null
    [[ ! -f ${source%.fyl}.stdin ]] || input=${source%.fyl}.stdin
    set +e
    timeout 10 "$binary" "${arguments[@]}" < "$input" > "$prefix.stdout" 2> "$prefix.stderr"
    local status=$?
    set -e
    echo "$status" > "$prefix.status"
    if [[ $status == 124 || $status -gt 128 ]]; then
        echo "execution failed or timed out: $source (status $status)" >&2
        exit 1
    fi
}
for source in "$repo"/experimental/fixtures/pass/*.fyl "$repo"/experimental/fixtures/pass/modules/*/main.fyl; do
    name=$(basename -- "$source" .fyl)
    timeout 60 "$oracle" compile "$source" -o "$work/reference" > "$work/oracle.log" 2>&1 || {
        cat "$work/oracle.log" >&2; exit 1;
    }
    timeout 60 "$compiler" "$source" "$work/native" > "$work/compiler.log" 2>&1 || {
        cat "$work/compiler.log" >&2; exit 1;
    }
    chmod +x "$work/native"
    run_binary "$work/reference" "$source" "$work/reference"
    run_binary "$work/native" "$source" "$work/native"
    for stream in stdout stderr status; do
        cmp "$work/reference.$stream" "$work/native.$stream" || {
            echo "differential mismatch: $name $stream" >&2
            diff -u "$work/reference.$stream" "$work/native.$stream" >&2 || true
            exit 1
        }
        expected=${source%.fyl}.$stream
        if [[ -f $expected ]]; then
            cmp "$expected" "$work/native.$stream" || {
                echo "expectation mismatch: $name $stream" >&2; exit 1;
            }
        elif [[ $stream == status ]]; then
            [[ $(cat "$work/native.status") == 0 ]] || exit 1
        elif [[ $stream == stderr ]]; then
            [[ ! -s $work/native.stderr ]] || exit 1
        fi
    done
    [[ $(od -An -tx1 -N4 "$work/native" | tr -d ' \n') == 7f454c46 ]] || exit 1
    passes=$((passes + 1))
    echo "PASS native + differential: $name"
done
for source in "$repo"/experimental/fixtures/fail/*.fyl "$repo"/experimental/fixtures/fail/modules/*/main.fyl "$repo"/tests/compile-fail/*.fyl "$repo"/tests/executable-fail/*.fyl; do
    if timeout 20 "$oracle" compile "$source" -o "$work/oracle-rejected" > "$work/oracle-reject.log" 2>&1; then
        echo "negative fixture unexpectedly accepted by reference: $source" >&2
        exit 1
    fi
    set +e
    timeout 20 "$compiler" "$source" "$work/rejected" > "$work/reject.log" 2>&1
    status=$?
    set -e
    if [[ $status == 0 || $status == 124 || $status -gt 128 ]]; then
        echo "invalid source not rejected safely: $source (status $status)" >&2
        cat "$work/reject.log" >&2
        exit 1
    fi
    [[ -s $work/reject.log ]] || { echo "missing diagnostic: $source" >&2; exit 1; }
    rejects=$((rejects + 1))
done
if [[ -d $repo/experimental/fixtures/trap ]]; then
    for source in "$repo"/experimental/fixtures/trap/*.fyl; do
        timeout 60 "$oracle" compile "$source" -o "$work/reference" > "$work/oracle.log" 2>&1
        timeout 60 "$compiler" "$source" "$work/native" > "$work/compiler.log" 2>&1 || {
            cat "$work/compiler.log" >&2; exit 1;
        }
        chmod +x "$work/native"
        run_binary "$work/reference" "$source" "$work/reference"
        run_binary "$work/native" "$source" "$work/native"
        for stream in stdout stderr status; do cmp "$work/reference.$stream" "$work/native.$stream"; done
        [[ $(cat "$work/native.status") == 70 ]] || exit 1
        traps=$((traps + 1))
    done
fi
echo "$passes native differential programs, $rejects safe compile rejections, $traps runtime failure comparisons passed."
