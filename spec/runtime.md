# Runtime behavior

`exit(code)` intentionally terminates the process. It takes one `int`, never
returns, and uses the low eight bits as the observable process status on the
current Linux target. Returning from `main` produces status zero.

Runtime failures use status 70 in the current language subset. They include
invalid array, list, string-byte, and string-slice bounds; popping an empty
list; file failures; allocation failures; and integer division failure. This
provides a defined failure result but not yet a catchable panic value or stack
trace.

The Rust bootstrap emits Linux x86-64 ELF executables directly and uses Linux
syscalls without libc. Heap-backed strings, lists, and struct records are
allocated for the lifetime of the process and are not reclaimed. The syscall,
layout, and allocation strategy remain Stage 0 implementation choices rather
than a stable ABI.

Filesystem operations are specified by [`std.fs`](standard-library-fs.md), and
standard stream behavior is specified by [`std.io`](standard-library-io.md).
File objects, append mode, deletion, and environment access are not specified.
