# Standard library I/O

The `std` hierarchy is part of Aerofyl and requires no package declaration or
external dependency. Its modules are not placed in scope automatically. The
first standard-library module is imported explicitly:

```fyl
use std.io;
```

`std.io` makes `print`, `println`, `eprint`, `eprintln`, and `input` available
to the program. Using one of these names without importing `std.io` is a
compile error. Other `std` modules are not yet defined.

## Output

`print(value)` writes to standard output and `eprint(value)` writes to standard
error. `println(value)` and `eprintln(value)` write to the corresponding stream
and then write one line-feed byte (`\n`). The non-`ln` forms add nothing.
Empty strings are valid and sequential calls preserve byte order.

The four functions accept `string`, `char`, `int`, and `bool`. Strings are
written as their stored bytes. A character is written as the UTF-8 encoding of
its Unicode scalar value. Integers use signed base-ten notation with no leading
zeroes except for `0`; the entire signed 64-bit range is supported. Boolean
values are exactly `true` and `false`. Floating-point I/O is unavailable while
floating-point executable lowering is unavailable.

## Input

`input()` is equivalent to `input(string)`. The supported forms are:

```fyl
string text = input(string);
int number = input(int);
bool flag = input(bool);
char character = input(char);
```

Each call reads one line from standard input. A terminating `\n` is removed. If
that byte is immediately preceded by `\r`, the `\r` is also removed, so both LF
and CRLF input have the same returned contents. A bare `\r` is data. At EOF,
bytes already read form the final line; EOF before any bytes returns an empty
string. Consequently, EOF is valid for string input but fails conversion for
the other currently supported input types.

String input returns the line bytes unchanged after delimiter removal. Integer
input accepts one or more ASCII decimal digits with an optional leading `+` or
`-`, no whitespace, and a value in the signed 64-bit range. Boolean input
accepts exactly `true` or `false`. Character input requires exactly one
well-formed UTF-8 encoding of a Unicode scalar value; empty input, multiple
characters, overlong encodings, surrogate values, and out-of-range values are
invalid.

Because Aerofyl does not yet have recoverable language-level errors, failed
typed conversion writes a diagnostic of this form to standard error and exits
with runtime-failure status 70:

```text
InputTypeError: expected int, got "hello"
```

`InputTypeError` is a diagnostic category, not a catchable value. Input syscall
and allocation failures also terminate with status 70, but are runtime failures
rather than typed-conversion diagnostics.

## Current implementation boundary

The current executable implementation targets Linux x86-64, uses direct
syscalls without libc, retries partial writes, grows input storage as needed,
and retains allocated input strings for the process lifetime. These platform,
allocation, and ABI choices are bootstrap limitations rather than portable
standard-library guarantees.
