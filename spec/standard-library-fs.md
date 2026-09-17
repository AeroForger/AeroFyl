# Standard library filesystem

The filesystem module is built into Aerofyl but opt-in:

```fyl
use std.fs;
```

Without this import, its function names are not in scope. No package or
dependency declaration is required.

## API

```fyl
string readFile(string path);
void writeFile(string path, string data);
list byte readBytes(string path);
void writeBytes(string path, list byte data);
bool exists(string path);
```

`readFile` returns every file byte in the existing length-prefixed string
representation. It does not normalize newlines, trim data, expose a trailing
NUL, or validate UTF-8. String literals are UTF-8, but a string returned from
`readFile` may contain arbitrary bytes. Code handling intentionally binary data
should use `readBytes` so its type and representation remain explicit.

`writeFile` creates a missing file or truncates an existing file, then writes
the string bytes exactly. Creation requests mode `0666`, filtered by the
process umask.

`readBytes` returns one `byte` element per file byte without text decoding.
Empty files produce an empty list. `writeBytes` writes only the list's element
bytes: it adds no header, terminator, encoding, or newline. Both functions
preserve `0x00`, `0xff`, and every other byte value exactly.

`byte` is an unsigned scalar from 0 through 255. In-range decimal literals are
accepted when a byte is expected. Bytes support equality and ordering, explicit
`int(byte)` and checked `byte(int)` conversion, and `list byte` indexing,
`.length`, `.push`, and `.pop`. Byte arithmetic is not defined. Each byte-list
element occupies exactly one byte; the list header is not part of file output.

`exists` follows the path as Linux `newfstatat` normally does. It returns
`true` for any existing filesystem object, including directories and special
files, and `false` only for the normal `ENOENT` not-found result. Other errors
are runtime failures.

## Failures and allocation

Read failures write `FileReadError` to standard error; write failures write
`FileWriteError`; other metadata failures write `FileSystemError`. They then
terminate with runtime-failure status 70. These names are diagnostic categories,
not catchable values.

The Linux x86-64 bootstrap reads complete seekable files, retries interrupted
data reads/writes and partial writes, and retains allocated buffers until
process exit. `readBytes` and `writeBytes` currently copy between the file
buffer layout and the one-byte list layout. Files must fit the process address
space and current signed length representation. Embedded NUL bytes in paths are
unsupported.

Runtime functions are emitted by symbol reachability. Importing `std.fs` alone
adds no code; only helpers reachable from actually used operations and their
runtime dependencies are linked into the executable. This is a current
bootstrap guarantee.
