# Runtime behavior

The permanent Aerofyl runtime model is not specified yet.

The Rust bootstrap emits Linux x86-64 ELF executables directly and uses Linux syscalls without libc. Heap-backed values are allocated for the lifetime of the process. Bounds failures, empty list pops, file failures, and allocation failures terminate with status 70. These are Stage 0 implementation choices, not permanent language guarantees.

The known file builtin is `readFile(path)`. The bootstrap accepts one `string`, reads the file as bytes, and returns a `string`. File objects, writes, directories, standard input, and environment access are not specified yet.
