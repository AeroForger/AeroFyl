<div align="center">

<img src="assets/AeroFyl3.png">

# Aerofyl

Aerofyl is a compiled systems programming language currently in pre-alpha.

The current compiler is written in Rust and exists to bootstrap the first compiler written in Aerofyl. It is the Stage 0 implementation, not the permanent compiler and not a claim of self-hosting. Its current subset includes relative multi-file imports and intentional `exit(code)` termination for compiler development.

</div>

## Development phases

| Phase | Compiler |
| --- | --- |
| Pre-alpha | Rust bootstrap compiler |
| Alpha 1 | First compiler written in Aerofyl |
| Later Alpha | Aerofyl compiler compiles later versions of itself |

## Current implementation

The bootstrap includes source loading, lexing, a handwritten parser, name resolution, semantic analysis, HIR, backend-independent IR, IR verification, and direct generation of Linux x86-64 ELF executables. Its implemented source subset includes functions, locals, assignment, integer arithmetic, bytes, Boolean logic, control flow, structs, payload-free enums, fixed arrays, lists, strings, string byte and slice operations, filesystem access, and command-line arguments.

The executable backend is limited to Linux x86-64. The current compiler subset
has defined signed-integer behavior, explicit integer/character conversions,
stable command-line arguments, the explicitly imported `std.io` standard-stream
API, and the `std.fs` text/binary filesystem API. Package
management, qualified user-module paths, comments, string escapes,
floating-point execution, and several
aggregate layouts remain unspecified or unimplemented. See [bootstrap
details](bootstrap/rust/README.md) for the precise boundary.

## Repository layout

- `bootstrap/rust/`: Stage 0 Rust compiler
- `compiler/frontend/`: starting point for the manually written Aerofyl compiler
- `examples/`: checked Aerofyl source examples
- `spec/`: language reference and records of unspecified areas
- `mdbook/`: user-facing guide
- `tests/`: source fixtures and integration test runner
- `stress-tests/`: bounded compiler stress fixtures
- `cli/`: status of command-line interface work

## Build and test

Build the workspace:

```bash
cargo build --workspace
```

Run the Rust test suite and the `.fyl` fixture checks:

```bash
cargo test --workspace
./tests/run_tests.sh
```

Check one source file without emitting an executable:

```bash
cargo run -p aerofyl-bootstrap -- examples/functions.fyl
```

Compile an existing example:

```bash
cargo run -p aerofyl-bootstrap -- compile examples/functions.fyl -o /tmp/aerofyl-functions
/tmp/aerofyl-functions
```

The language reference begins at [spec/README.md](spec/README.md). The Alpha compiler will be written manually in Aerofyl under `compiler/`; only its initial frontend data model exists today.
