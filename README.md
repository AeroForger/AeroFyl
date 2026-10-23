<div align="center">

<img src="assets/AeroFyl5.svg">

# Aerofyl

**A compiled systems language, in pre-alpha.**

[Official VS Code extension](https://github.com/AeroForger/AeroFyl-vscode)

</div>

## Where things stand

Aerofyl is early. The compiler you'll find here is written in Rust, and that's temporary: it's the Stage 0 compiler, built for one job, which is to compile the first Aerofyl compiler written in Aerofyl. Aerofyl is not self-hosting yet.

To get there, the Rust compiler supports relative multi-file imports and an `exit(code)` call, so the compiler can be written in the language it compiles.

## What works today

The bootstrap compiler goes all the way from source to a running program: it loads and lexes your code, parses it by hand, resolves names, checks semantics, lowers to HIR and a backend-independent IR, verifies that IR, and writes a Linux x86-64 ELF executable directly.

The language so far has:

- functions, locals, assignment, integer arithmetic (with defined signed behavior), bytes, booleans, and control flow
- structs, payload-free enums, fixed arrays, lists, and strings (including byte and slice operations)
- explicit integer/character conversions
- command-line arguments
- `std.io` for standard streams and `std.fs` for text and binary files, both explicitly imported

## What doesn't (yet)

Linux x86-64 is the only target. There's no package management, no qualified user-module path and no floating-point execution, and some aggregate layouts are still undecided. The [bootstrap details](bootstrap/rust/README.md) draw the exact line.

## Repository layout

- `bootstrap/rust/`: the Stage 0 Rust compiler
- `compiler/frontend/`: the start of the hand-written Aerofyl compiler
- `examples/`: Aerofyl programs that are checked in CI
- `spec/`: the language reference, plus notes on what's still unspecified
- `mdbook/`: the user guide (Coming soon)
- `tests/`: source fixtures and the integration test runner
- `stress-tests/`: bounded stress fixtures for the compiler
- `cli/`: status of the command-line interface work

## Try it

Build everything:

```bash
cargo build --workspace
```

Run the tests (Rust suite, then the `.fyl` fixtures):

```bash
cargo test --workspace
./tests/run_tests.sh
```

Check a file without producing an executable:

```bash
cargo run -p aerofyl-bootstrap -- examples/functions.fyl
```

Compile and run an example:

```bash
cargo run -p aerofyl-bootstrap -- compile examples/functions.fyl -o /tmp/aerofyl-functions
/tmp/aerofyl-functions
```

## Where to read next

The language reference starts at [spec/README.md](spec/README.md). The Alpha compiler will be written by hand in Aerofyl under `compiler/`; and also I feel obligated to mention this but Bootstrap compiler was made by AI to a certain extent.

## LICENSE

Apache 2.0