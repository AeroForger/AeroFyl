<div align="center">

<img src="assets/AeroFyl5.svg" alt="Aerofyl logo">

# Aerofyl

**A compiled systems language, in pre-alpha.**

[Official VS Code extension](https://github.com/AeroForger/AeroFyl-vscode)

</div>

## Where things stand

Aerofyl is early. The compiler you'll find here is written in Rust, and that's temporary. It's the Stage 0 compiler, built for one job, which is to compile the first Aerofyl compiler written in Aerofyl. Aerofyl is not self-hosting yet.

To get there, the Rust compiler supports relative multi-file imports and an `exit(code)` call, so the compiler can be written in the language it compiles.

## What works today

The bootstrap compiler goes all the way from source to a running program: it loads and lexes your code, parses it by hand, resolves names, checks semantics, lowers to HIR and a backend-independent IR, verifies that IR, and writes a Linux x86-64 ELF executable directly.

The language so far has:

- functions, locals, assignment, integer arithmetic (with defined signed behavior), bytes, booleans, and control flow
- structs, payload-free enums, fixed arrays, lists, and strings (including byte and slice operations)
- explicit integer/character conversions
- command-line arguments
- `std.os` for command execution, `std.io` for standard streams and `std.fs` for text and binary files, all explicitly imported

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
Try the dev kit for AeroFyl:

```bash
curl -fsSL https://raw.githubusercontent.com/AeroForger/AeroFyl/main/install-dev.sh | bash
```

## Where to read next

The language reference starts at [spec/README.md](spec/README.md). The Alpha compiler will be written by hand in Aerofyl under `compiler/`.

## Creator note

### AI usage

The bootstrap compiler was made with AI assistance to a certain extent. Generative AI was used in these places:

- `bootstrap/rust/middle`, including `hir.rs` and `verify.rs`
- `bootstrap/rust/backend`: originally written by me, but later additions to the compiler led me to use AI to add code to the backend
- `bootstrap/rust/resolution`: same as the backend

AI was only used as a tool To speed up parts of the work. All AI-generated code was tested thoroughly and edited by me to fix its mistakes. The logo, the syntax, and the language itself were fully designed by me. In the future I plan to scrap the bootstrap, and the new self-hosted compiler written in the language itself won't use any kind of AI assistance.

### Why it exists

AeroFyl is my second programming language. I made it for a few reasons:

1. To improve my Rust, compiler, and testing skills
2. Building a language in my free time gave me a project I actually enjoy working on (most of the time)
3. To have bragging rights :/

I don't expect AeroFyl to become anything huge or popular. It's more of a hobby language, so if you're reading this, please don't take the language too seriously.

## LICENSE

Apache 2.0
