# Stage 3 compiler

Stage 3 is the first executable compiler written in Aerofyl. It is intentionally
small and uses only syntax already accepted and documented by the Rust bootstrap.

The compiler accepts a non-empty `.fyl` source file using the existing language
syntax and emits a Linux x86-64 ELF executable that returns zero:

```fyl
public void main()
{
}
```

It reads a `.fyl` source path and an output path from its two command-line
arguments. Empty source is rejected. The generated program is currently a
status-zero stub; source parsing, semantic analysis, and lowering are later
Stage 3 work.

This is a bootstrap milestone, not a complete replacement for the Rust
compiler. The generated executable does not yet lower AeroFyl expressions,
statements, imports, entry points, or standard-library calls. Those features
remain work for later stages; no new syntax or language rule is introduced
here.

Build and run it from the repository root:

```bash
cargo run -p aerofyl-bootstrap -- compile \
  experimental/stage-3/compiler.fyl /tmp/aerofyl-stage3
touch /tmp/aerofyl-stage3-hello
chmod +x /tmp/aerofyl-stage3-hello
/tmp/aerofyl-stage3 examples/hello.fyl /tmp/aerofyl-stage3-hello
/tmp/aerofyl-stage3-hello
```

The output path must already be executable (as shown above), because the
current `std.fs` write API preserves the mode of an existing file and does not
yet provide a permissions operation.

The compiler itself uses the existing `std.fs` and `std.io` modules for source
loading, diagnostics, and executable output.
