# Standard library process execution

The `std.os` module is built into Aerofyl but opt-in:

```fyl
use std.os;
```

This namespace access form gives `os.process.execute(...)`:

```fyl
use std.os;

public void main()
{
    Process result = os.process.execute("/bin/echo captured");
}
```

A selective import gives the member name directly:

```fyl
using std.os: process;

public void main()
{
    Process result = process.execute("/bin/echo captured");
}
```

`std.os` exports only `process`; selecting any other name is a compile error.
Both import forms bind the same operations.

## API

```fyl
os.process.execute(string command) -> Process
```

With the snapshot `Process` type:

```fyl
struct Process
{
    int exitCode;
    string out;
    string err;
}
```

The compiler supplies the `Process` type declaration while `std.os` is
imported, in every module that imports it, exactly with the fields and order
above. A source file that declares its own `Process` struct wins; the injected
declaration is then withheld from that module and the local type is used.

## Execution semantics

**No shell.** [provisional] The command string is parsed into an argument
vector before execution, never passed to a shell: no `PATH` search, no
environment or tilde expansion, no globbing, no command substitution. Tokens
are separated by spaces, tabs, and newlines. Both quote characters (`'` and
`"`) group a run of characters so a single argument may contain spaces and the
executable path itself may be quoted. There is no escape character inside
quotes; an open quote without a closing quote is a runtime failure. The first
token is the executable; it must address the file (absolute or relative path).

**Streams.** [provisional] Standard output and standard error of the child are
captured through separate pipes and returned as the `out` and `err` strings,
byte for byte, with no newline normalization, prefix, or trailing trimming.
Empty output produces the empty string. The parent drains both pipes in an
alternating non-blocking loop before waiting, so a child writing a large or
one-sided stream cannot deadlock in a filled pipe. The child writes with the
environment's default buffering; an executable's own buffer state is its own
behavior.

**Exit code.** [provisional] `exitCode` is the child's true terminating status
as reported by the platform's process wait, not an error marker of the
`execute` call. A nonzero child exit is a normal `Process` result, not a
runtime failure. When the child image cannot be launched (unknown executable,
path to a non-executable, and similar launch failures), the child reports the
conventional status 127 through `exitCode` on the current target.
Statuses above 255 on the current target are outside the observable range,
like `exit` in [runtime.md](runtime.md).

## Failures

[provisional] Runtime failures use the same terminal model as
[standard-library-fs.md](standard-library-fs.md): write one diagnostic line to
standard error, then terminate with runtime-failure status 70. The names are
diagnostic categories, not catchable values.

- `ProcessError`: an empty command string, an unterminated quote, and pipe,
  fork, or wait bookkeeping failures inside `execute`.
- `ProcessSignalError`: the child terminated because of a signal. Such a child
  child never yields an `exitCode`; the program terminates with status 70.

Captured partial output from a failing `execute` call is discarded with the
call; there is no catchable result value.

## Implementation reachability

[provisional] Runtime functions are emitted by symbol reachability, as in
[`std.fs`](standard-library-fs.md). Importing `std.os` alone adds no code; only
executions reachable from actual `execute` calls are linked into the
executable.

## Unspecified

- Other process operations: argument arrays, environment, working directory,
  pipes to child input, background/asynchronous execution, timeouts.
- Console inheritance while a child runs is observable through the child but
  not otherwise specified.
- `PATH` search semantics: intentionally absent, not unspecified; a documented
  argument-array mechanism would be needed to add it.

---
**Status snapshot (2026-10-23)**. See [process.md](process.md).

| Rule | Status |
| --- | --- |
| Import forms and `Process` type injection | experimental |
| No-shell execution and tokenizer | experimental |
| Stream capture and drain | experimental |
| Exit code and launch-failure 127 | provisional |
| `ProcessError` / `ProcessSignalError` failures | provisional |
| Everything in Unspecified | unspecified |

**Changelog**

- 2026-10-23. Initial spec for `std.os`, written to match the newly
  implemented module; stream capture and the no-shell tokenizer entered at
  experimental because the module is new, exit-code behavior at provisional
  because it follows well-tested wait semantics.
