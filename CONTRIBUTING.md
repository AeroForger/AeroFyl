# Contributing to AeroFyl

AeroFyl is a compiled systems language in pre-alpha. The Rust compiler in
`bootstrap/rust/` is Stage 0 scaffolding; the real target is the self-hosted
compiler growing in `compiler/frontend/`. Contributions should move that
goal forward without breaking what already executes.

## How to contribute

1. Pick work that serves the pillars below. If a change fights a pillar,
   say so in the PR and explain why it is still worth it.
2. Keep `experimental/` alone. It is exploration space, not project code:
   do not refactor it, gate CI on it, or copy patterns out of it without
   discussion.
3. Match the language subset that exists. New syntax needs spec text in
   `spec/`, parser support in both frontends where applicable, and tests.
4. Document every change: update the relevant `README.md` or `spec/` page
   in the same PR. Undocumented behavior is unfinished behavior.
5. Verify before opening the PR (see below) and paste the results in the
   PR description using the pull request template.

## Required verification

Run what your change touches, at minimum:

```sh
cargo test -p aerofyl-bootstrap
python3 compiler/tests/parser.py
```

The parser suite builds native executables, checks syntax parity with the
Rust frontend, and enforces termination (nesting limits, long chains,
malformed-input fuzz). If you touch the bootstrap backend or runtime,
also run the affected fixtures under `tests/` and `examples/` to confirm
they still execute with the expected exit statuses.

## The pillars, and how to follow them

Ranked. Higher pillars win conflicts.

### Main: Executable First

The language exists to produce programs that run. A contribution is done
only when something executes:

- Every feature or fix must end in runnable proof: a passing test that
  compiles *and runs* a binary, or a checked example under `examples/`.
- Do not merge syntax the backend cannot execute without marking it
  clearly in `spec/` as unimplemented. Parsing ahead of lowering is
  allowed only when documented (as with current interpolation
  differences) and tracked.
- Prefer deleting dead or unrunnable code over carrying it.

### Semi-main: Speed

- No pathological algorithms. The frontend parses 2000-long postfix and
  binary chains iteratively and caps nesting at 96 — keep it that way.
- Watch compile times: the test runner budgets ~30s per bootstrap build
  and ~5s per parse. If your change makes builds flaky or slow, it is a
  regression even if tests pass.
- Measure before claiming a speedup; paste numbers in the PR.

### Semi-main: Small binaries

- The backend emits minimal Linux x86-64 ELF via direct syscalls, no
  libc. Do not add runtime dependencies, vendored blobs, or stdlib
  surface without justification.
- New `std` modules must earn their place: propose the smallest API that
  covers the use case, in `spec/` first.

### Other: Transparency

- Every failure must explain itself: diagnostics carry source paths and
  byte spans (`path:start-end: error: message`); warnings (`: warning: `)
  never fail a build silently or loudly — they inform.
- No silent behavior changes. If output, exit codes, or accepted syntax
  change, the PR description says so up front and docs are updated.
- Record known divergences (e.g. `DIVERGED_FIXTURES` in
  `compiler/tests/parser.py`) instead of hiding them.

### Other: Safety

- Defined behavior over clever behavior: signed 64-bit integers, checked
  bounds and conversions, status 70 runtime failures instead of UB.
- New unsafe-adjacent work (syscalls, memory layout, codegen) needs a
  test that fails predictably, not just a test that passes.
- Report safety problems privately per `SECURITY.md`, never as public
  issues or demo exploits.
