# Source fixtures

`compile-pass/` contains source files that the bootstrap must accept. `compile-fail/` contains source files that it must reject. `runtime/` and `verifier/` reserve locations for future file-based tests; current runtime and verifier coverage remains in Rust unit tests near the implementation.

Run all current file-based checks from the repository root:

```bash
./tests/run_tests.sh
```
