# Stage 1 compiler experiment

This directory answers one narrow question: can the Rust Stage 0 compiler build
a useful compiler that is itself written in Aerofyl?

`compiler.fyl` implements a tiny compiler for this source language:

```text
program    = "return" expression ";" "expect" integer ";" ;
expression = integer, `+`, `-`, `*`, `/`, and parentheses ;
```

It reads a file, lexes it, parses expressions with operator precedence, emits
bytecode for a stack machine, executes that bytecode, and checks the result
against the `expect` clause. A valid matching program exits successfully.
Malformed input, division by zero, stack errors, and a mismatched expectation
exit with the bootstrap runtime failure status (70).

Run the experiment from the repository root:

```bash
./experimental/stage-1/test.sh
```

This is deliberately isolated from `compiler/`: it is evidence that Stage 1 can
start, not the permanent compiler architecture. This prototype compiles to an
in-memory bytecode list and reports success or failure through its exit status.
