# Source fixtures

`compile-pass/` contains source files that the bootstrap must accept.
`compile-fail/` contains files rejected during checking. `executable-fail/`
contains otherwise parseable programs with invalid or missing entry points.
`modules/` contains a multi-file compiler-shaped fixture. `runtime/` contains
executable fixtures for process status, integer boundaries, arguments, file
output, standard I/O, typed input failures, text/binary filesystem operations,
byte conversions, and compiler-oriented values. Detailed runtime and verifier coverage
also remains in Rust unit tests near the implementation.

Run all current file-based checks from the repository root:

```bash
./tests/run_tests.sh
```
