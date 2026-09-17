# Standard library filesystem

Import the built-in filesystem module explicitly:

```fyl
use std.fs;
```

It provides textual `readFile`/`writeFile`, binary
`readBytes`/`writeBytes`, and `exists`. Binary data uses `list byte`; each
element is an unsigned value from 0 through 255 and occupies one byte in list
storage. `byte(int)` is checked, while `int(byte)` preserves the value.

Text operations preserve string bytes without newline or encoding conversion.
Binary operations preserve every byte and add no metadata. `exists` is true for
any existing filesystem object and false for a normally missing path.

Filesystem failures report `FileReadError`, `FileWriteError`, or
`FileSystemError` on standard error and terminate with status 70. The current
implementation is Linux x86-64-only and uses direct syscalls.

## Examples

The repository includes small complete programs for common filesystem tasks:

- [`file_read.fyl`](../../examples/file_read.fyl) reads the path supplied as its
  first argument.
- [`file_text_roundtrip.fyl`](../../examples/file_text_roundtrip.fyl) writes a
  text value and verifies the exact value after reading it back.
- [`file_bytes.fyl`](../../examples/file_bytes.fyl) writes and checks raw bytes,
  including ELF magic, zero, and `255`.
- [`file_exists.fyl`](../../examples/file_exists.fyl) exits successfully only
  when its argument names an existing filesystem object.
