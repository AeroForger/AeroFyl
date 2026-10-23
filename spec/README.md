# Aerofyl language reference

This directory records language rules that are explicitly known. It separates
language rules from temporary choices made by the Rust bootstrap compiler.

Every rule in every file carries a status: **stable**, **provisional**, or
**experimental**. Behavior the language has not decided is marked
**unspecified** instead of left to inference. Each file begins with a status
snapshot and a changelog. [process.md](process.md) defines how rules change,
what evidence a promotion needs, and why ideas stay out of this directory, and
that file is part of the specification.

- [Change process](process.md)
- [Lexical structure](lexical.md)
- [Grammar](grammar.md)
- [Types](types.md)
- [Optional values](optionals.md)
- [References](references.md)
- [Explicit conversions](conversions.md)
- [Integers](integers.md)
- [Semantics](semantics.md)
- [Functions](functions.md)
- [Control flow](control-flow.md)
- [Structs](structs.md)
- [Enums](enums.md)
- [Arrays](arrays.md)
- [Lists](lists.md)
- [Strings](strings.md)
- [Modules](modules.md)
- [Standard library I/O](standard-library-io.md)
- [Standard library filesystem](standard-library-fs.md)
- [Standard library process execution](standard-library-os.md)
- [Runtime](runtime.md)
- [Memory and value behavior](memory.md)
- [Diagnostics and source locations](diagnostics.md)
- [Entry point](entry-point.md)
- [Bootstrap boundary](bootstrap.md)

**Coverage rule:** the spec covers the full current language as implemented.
Anything implemented but undocumented is a spec defect (see
[process.md](process.md)). If a rule is absent or marked unspecified, no
behavior should be inferred from the current implementation.
