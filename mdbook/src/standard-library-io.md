# Standard library I/O

Import the built-in I/O module before using its functions:

```fyl
use std.io;

public void main()
{
    println("Hello, World!");
}
```

`print` and `println` write strings, characters, integers, or Booleans to
standard output. `eprint` and `eprintln` write the same values to standard
error. The `ln` forms append `\n`.

`input()` reads a line as a string. `input(string)`, `input(int)`,
`input(bool)`, and `input(char)` select a result type. LF and CRLF delimiters
are removed. Invalid typed input reports `InputTypeError` on standard error and
terminates with status 70; it is not catchable. EOF with no pending bytes is an
empty string, so it is valid for string input and invalid for the other types.

The bootstrap implementation currently supports this API only in emitted
Linux x86-64 executables. Floating-point I/O is not implemented.
