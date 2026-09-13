# Command-line interface

There is no separate Aerofyl CLI implementation yet. The current commands are provided by the Rust bootstrap binary.

Check one source file:

```bash
cargo run -p aerofyl-bootstrap -- path/to/source.fyl
```

Compile one source file to a Linux x86-64 executable:

```bash
cargo run -p aerofyl-bootstrap -- compile path/to/source.fyl -o path/to/output
```

The final CLI name, command structure, project commands, and configuration are not specified yet.
