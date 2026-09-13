# Getting Started

Build from the repository root:

```bash
cargo build --workspace
```

Check an example without creating an executable:

```bash
cargo run -p aerofyl-bootstrap -- examples/functions.fyl
```

Compile and run it on Linux x86-64:

```bash
cargo run -p aerofyl-bootstrap -- compile examples/functions.fyl -o /tmp/aerofyl-functions
/tmp/aerofyl-functions
```

The examples do not print output because `print` behavior is not specified or implemented. A zero exit status indicates that this example ran without a bootstrap runtime failure.

Run validation with:

```bash
cargo test --workspace
./tests/run_tests.sh
```
