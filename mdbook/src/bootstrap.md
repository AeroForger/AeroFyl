# Bootstrap Compiler

The Stage 0 compiler is written in Rust. It validates source through IR, lowers the implemented executable subset to x86-64 instructions, encodes those instructions, and writes a Linux ELF file directly.

The bootstrap uses System V AMD64 calling behavior and Linux syscalls. Heap-backed allocations live until process exit. Bounds, empty-pop, file, and allocation failures exit with status 70. These choices belong to the temporary implementation and are not permanent Aerofyl rules.

The source crate is under `bootstrap/rust/`. The manually written Aerofyl compiler begins under `compiler/`, but it has not been implemented.
