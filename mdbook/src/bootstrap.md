# Bootstrap Compiler

The Stage 0 compiler is written in Rust. It validates source through IR, lowers the implemented executable subset to x86-64 instructions, encodes those instructions, and writes a Linux ELF file directly.

The bootstrap uses System V AMD64 calling behavior and Linux syscalls.
Heap-backed allocations live until process exit. Runtime failures use status
70, and programs can terminate intentionally with `exit(code)`. The syscall and
allocation details remain temporary implementation choices.

Sibling modules can be loaded recursively with `use name;`. Public functions
cross file boundaries, private functions remain file-local, and structs and
enums are shared through the loaded dependency graph.
`std.io` and `std.fs` are built-in modules but remain explicitly imported.
Runtime helpers are linked only when reachable from emitted program operations.

The source crate is under `bootstrap/rust/`. The manually written Aerofyl compiler begins under `compiler/`, but it has not been implemented.
