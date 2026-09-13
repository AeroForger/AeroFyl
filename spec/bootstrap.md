# Rust bootstrap compiler

The Rust implementation is the Stage 0 compiler used during Aerofyl pre-alpha. It exists to compile the manually written Alpha 1 compiler. It is temporary and Aerofyl is not self-hosted yet.

The bootstrap pipeline is source loading, lexing, handwritten parsing, name resolution, semantic analysis, HIR, backend-independent IR, IR verification, Linux x86-64 lowering, direct instruction encoding, and direct ELF generation.

Temporary implementation choices, including stack layout, heap records, process-lifetime allocation, Linux syscalls, System V AMD64 calls, status 70 runtime failures, and current scalar representations, are not permanent language or ABI rules. Precise implementation details and unsupported forms are documented in [the bootstrap README](../bootstrap/rust/README.md).

Development phases:

```text
Pre-alpha
Rust bootstrap compiler

Alpha 1
First compiler written in Aerofyl

Later Alpha
Aerofyl compiler compiles later versions of itself
```
