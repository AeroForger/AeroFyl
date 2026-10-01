#!/usr/bin/env bash
# Orchestration only: all parsing, checking, code generation, and ELF writing
# occur inside the AeroFyl executable. chmod supplies a filesystem operation
# absent from current AeroFyl's std.fs API.
set -euo pipefail
if [[ $# -ne 3 ]]; then
    echo 'usage: experimental/compile.sh <compiler> <source.fyl> <output>' >&2
    exit 2
fi
timeout 60 "$1" "$2" "$3"
chmod +x -- "$3"
