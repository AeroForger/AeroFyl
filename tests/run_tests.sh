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

"${compiler[@]}" "$repository_root/tests/modules/compiler/main.fyl" >/dev/null

runtime_directory="$(mktemp -d)"
trap 'rm -rf "$runtime_directory"' EXIT

for source in "$repository_root"/tests/compile-fail/*.fyl; do
    if "${compiler[@]}" "$source" >/dev/null 2>&1; then
        printf 'expected compilation failure: %s\n' "$source" >&2
        exit 1
    fi
done

for source in "$repository_root"/tests/executable-fail/*.fyl; do
    if "${compiler[@]}" compile "$source" -o "$runtime_directory/invalid-entry" >/dev/null 2>&1; then
        printf 'expected executable compilation failure: %s\n' "$source" >&2
        exit 1
    fi
done

"${compiler[@]}" compile "$repository_root/tests/runtime/compiler_primitives.fyl" -o "$runtime_directory/compiler-primitives" >/dev/null
"$runtime_directory/compiler-primitives"

"${compiler[@]}" compile "$repository_root/tests/runtime/stage1_features.fyl" -o "$runtime_directory/stage1-features" >/dev/null
"$runtime_directory/stage1-features"

"${compiler[@]}" compile "$repository_root/tests/runtime/optional_missing_value.fyl" -o "$runtime_directory/optional-missing-value" >/dev/null
set +e
"$runtime_directory/optional-missing-value"
status=$?
set -e
if [[ $status -ne 70 ]]; then
    printf 'expected missing optional value access to fail with status 70, got %s\n' "$status" >&2
    exit 1
fi

"${compiler[@]}" compile "$repository_root/tests/runtime/integer_boundaries.fyl" -o "$runtime_directory/integer-boundaries" >/dev/null
"$runtime_directory/integer-boundaries"

for failure in divide_by_zero divide_overflow invalid_character_conversion; do
    "${compiler[@]}" compile "$repository_root/tests/runtime/$failure.fyl" -o "$runtime_directory/$failure" >/dev/null
    set +e
    "$runtime_directory/$failure"
    status=$?
    set -e
    if [[ $status -ne 70 ]]; then
        printf 'expected %s to fail with status 70, got %s\n' "$failure" "$status" >&2
        exit 1
    fi
done

"${compiler[@]}" compile "$repository_root/tests/runtime/write_file.fyl" -o "$runtime_directory/write-file" >/dev/null
"$runtime_directory/write-file" "$runtime_directory/generated-output"
if [[ $(<"$runtime_directory/generated-output") != "generated output" ]]; then
    printf 'write_file.fyl did not write the expected bytes\n' >&2
    exit 1
fi

"${compiler[@]}" compile "$repository_root/tests/runtime/arguments.fyl" -o "$runtime_directory/arguments" >/dev/null
"$runtime_directory/arguments" source.fyl output.bin

"${compiler[@]}" compile "$repository_root/tests/runtime/io_output.fyl" -o "$runtime_directory/io-output" >/dev/null
"$runtime_directory/io-output" >"$runtime_directory/io-output.stdout" 2>"$runtime_directory/io-output.stderr"
printf 'Hello, World!\n123\n-456\n0\n-9223372036854775808\ntrue\nfalse\nAé\n' >"$runtime_directory/io-output.expected-stdout"
printf 'error: something happened\n' >"$runtime_directory/io-output.expected-stderr"
cmp "$runtime_directory/io-output.expected-stdout" "$runtime_directory/io-output.stdout"
cmp "$runtime_directory/io-output.expected-stderr" "$runtime_directory/io-output.stderr"

"${compiler[@]}" compile "$repository_root/tests/runtime/io_input.fyl" -o "$runtime_directory/io-input" >/dev/null
printf 'alpha\nbeta\n42\n-9223372036854775808\ntrue\nfalse\nλ\n' \
    | "$runtime_directory/io-input" >"$runtime_directory/io-input.stdout" 2>"$runtime_directory/io-input.stderr"
printf 'alpha\nbeta\n42\n-9223372036854775808\ntrue\nfalse\nλ\n' >"$runtime_directory/io-input.expected"
cmp "$runtime_directory/io-input.expected" "$runtime_directory/io-input.stdout"
test ! -s "$runtime_directory/io-input.stderr"

"${compiler[@]}" compile "$repository_root/tests/runtime/io_crlf.fyl" -o "$runtime_directory/io-crlf" >/dev/null
printf 'windows\r\n' | "$runtime_directory/io-crlf" >"$runtime_directory/io-crlf.stdout"
printf 'windows\n' >"$runtime_directory/io-crlf.expected"
cmp "$runtime_directory/io-crlf.expected" "$runtime_directory/io-crlf.stdout"

"${compiler[@]}" compile "$repository_root/tests/runtime/io_eof.fyl" -o "$runtime_directory/io-eof" >/dev/null
"$runtime_directory/io-eof" </dev/null >"$runtime_directory/io-eof.stdout"
printf '0\n' >"$runtime_directory/io-eof.expected"
cmp "$runtime_directory/io-eof.expected" "$runtime_directory/io-eof.stdout"

for type in int bool char; do
    "${compiler[@]}" compile "$repository_root/tests/runtime/io_invalid_$type.fyl" -o "$runtime_directory/io-invalid-$type" >/dev/null
    case "$type" in
        int) invalid_input=hello ;;
        bool) invalid_input=TRUE ;;
        char) invalid_input=ab ;;
    esac
    set +e
    printf '%s\n' "$invalid_input" | "$runtime_directory/io-invalid-$type" >"$runtime_directory/io-invalid-$type.stdout" 2>"$runtime_directory/io-invalid-$type.stderr"
    status=$?
    set -e
    if [[ $status -ne 70 ]]; then
        printf 'expected invalid %s input to fail with status 70, got %s\n' "$type" "$status" >&2
        exit 1
    fi
    test ! -s "$runtime_directory/io-invalid-$type.stdout"
    grep -Fx "InputTypeError: expected $type, got \"$invalid_input\"" "$runtime_directory/io-invalid-$type.stderr" >/dev/null
done

"${compiler[@]}" compile "$repository_root/tests/runtime/fs_text.fyl" -o "$runtime_directory/fs-text" >/dev/null
"$runtime_directory/fs-text" "$runtime_directory/text.txt" "$runtime_directory/missing.txt" "$runtime_directory"
test -f "$runtime_directory/text.txt"
test ! -s "$runtime_directory/text.txt"

"${compiler[@]}" compile "$repository_root/tests/runtime/fs_bytes.fyl" -o "$runtime_directory/fs-bytes" >/dev/null
"$runtime_directory/fs-bytes" "$runtime_directory/all-bytes.bin" "$runtime_directory/elf.bin" "$runtime_directory/empty.bin"
all_bytes=$(od -An -v -t u1 "$runtime_directory/all-bytes.bin" | tr -s '[:space:]' ' ')
expected_all_bytes=" $(seq -s ' ' 0 255) "
if [[ "$all_bytes" != "$expected_all_bytes" ]]; then
    printf 'fs_bytes.fyl did not preserve all byte values\n' >&2
    exit 1
fi
elf_bytes=$(od -An -v -t u1 "$runtime_directory/elf.bin" | tr -s '[:space:]' ' ')
if [[ "$elf_bytes" != " 127 69 76 70 0 255 " ]]; then
    printf 'fs_bytes.fyl did not write the expected ELF-magic bytes\n' >&2
    exit 1
fi
test ! -s "$runtime_directory/empty.bin"

"${compiler[@]}" compile "$repository_root/tests/runtime/fs_missing_read.fyl" -o "$runtime_directory/fs-missing-read" >/dev/null
set +e
"$runtime_directory/fs-missing-read" "$runtime_directory/absent.bin" >"$runtime_directory/fs-missing.stdout" 2>"$runtime_directory/fs-missing.stderr"
status=$?
set -e
if [[ $status -ne 70 ]] || ! grep -Fx 'FileReadError' "$runtime_directory/fs-missing.stderr" >/dev/null; then
    printf 'missing filesystem input did not report FileReadError with status 70\n' >&2
    exit 1
fi

"${compiler[@]}" compile "$repository_root/tests/runtime/fs_invalid_write.fyl" -o "$runtime_directory/fs-invalid-write" >/dev/null
set +e
"$runtime_directory/fs-invalid-write" "$runtime_directory" >"$runtime_directory/fs-write.stdout" 2>"$runtime_directory/fs-write.stderr"
status=$?
set -e
if [[ $status -ne 70 ]] || ! grep -Fx 'FileWriteError' "$runtime_directory/fs-write.stderr" >/dev/null; then
    printf 'invalid filesystem output did not report FileWriteError with status 70\n' >&2
    exit 1
fi

for failure in invalid_byte_conversion invalid_negative_byte_conversion; do
    "${compiler[@]}" compile "$repository_root/tests/runtime/$failure.fyl" -o "$runtime_directory/$failure" >/dev/null
    set +e
    "$runtime_directory/$failure"
    status=$?
    set -e
    if [[ $status -ne 70 ]]; then
        printf 'expected %s to fail with status 70, got %s\n' "$failure" "$status" >&2
        exit 1
    fi
done

"${compiler[@]}" compile "$repository_root/tests/runtime/exit_23.fyl" -o "$runtime_directory/exit-23" >/dev/null
set +e
"$runtime_directory/exit-23"
status=$?
set -e
if [[ $status -ne 23 ]]; then
    printf 'expected exit_23.fyl to exit with status 23, got %s\n' "$status" >&2
    exit 1
fi

printf 'Aerofyl fixture checks passed.\n'
