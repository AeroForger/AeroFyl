# Modules and imports

Each `.fyl` file is a module and each module is a namespace. The namespace is
named after the file: declarations at the top level of `file2.fyl` live in the
namespace `file2`. A file imports another module with a top-level declaration:

```fyl
use lexer;
```

`use lexer;` loads `lexer.fyl` and puts the namespace itself into scope. Its
members are then reached with a dotted path, and there is no unqualified access
through `use`:

```fyl
use lexer;

public void main()
{
    lexer.scan();
}
```

Before file-scoped namespaces, an import made the imported names usable without
a prefix:

```fyl
// Old behavior. No longer valid.
use lexer;

public void main()
{
    scan();
}
```

Calling `scan()` after only `use lexer;` is a compile error and the diagnostic
suggests the qualified form, `lexer.scan()`.

To import only named declarations from a file, use `using`:

```fyl
using lexer: scan, Token;
```

`using lexer: scan, Token;` loads `lexer.fyl` and makes only `scan` and `Token`
available without a prefix. `using` lists names separated by commas and ends
with `;`. Functions and named types can be selected. The language currently has
no module-level variable declarations to select. A `using` declaration does not
expose other names from its file: `scan()` works, but any other `lexer` member
still needs the `lexer.` prefix.

`use lexer;` resolves only to `lexer.fyl` in the importing file's directory.
The command-line source file is the root module. Imports are recursive, and a
module reached through more than one dependency is loaded once. Missing files,
circular imports, and two imports of the same kind for one module in one file
are errors. `use lexer;` combined with `using lexer: scan;` is valid because
the two declarations bind different names. Diamond-shaped dependency graphs
are valid and do not count as duplicate imports.

## Visibility

Only `public` members of a namespace may be referenced from another file, and
only when a `use` chain exposes the file or a `using` declaration selects the
name. Referencing a `private` function through a dotted path or through a
`using` selection is a compile error. Private functions may be referenced only
from their own file. Struct and enum declarations have no visibility modifier
in the current language.

## `namespace` declarations

The `namespace` keyword declares a nested namespace inside a file:

```fyl
namespace File2 {

public void function()
{
}
}
```

In `file2.fyl`, this member is reached as `file2.File2.function()`. A namespace
body accepts the same declarations as a file: structs, enums, functions, nested
namespaces, and its own `use` and `using` imports. Imports declared inside a
namespace body are visible inside that body and inside namespaces nested in it.

The namespaces behave like C++ namespaces:

- A namespace can be reopened within the same file. A second
  `namespace File2 { }` block in the same file adds members to the existing
  scope. Both blocks are one namespace.
- Declaring the same name twice in one namespace is an error. The language has
  no overloading, so duplicate declarations are rejected regardless of
  signature.
- Namespaces can nest, and dotted paths resolve one segment at a time. A path
  with a missing middle segment is an error named after that segment, for
  example `` `file2.File2` has no member `function` ``.
- A namespace belongs to the file that declares it. Two files can both declare
  `namespace Counts` and stay separate namespaces; nothing merges across
  files, and one file's namespace never makes the other file's declarations
  or members visible.

## File and namespace naming

The file stem becomes the namespace name, so it must be a valid Aerofyl
identifier: letters, digits, and underscores, and it cannot start with a digit.
`my-utils.fyl` is rejected because `my-utils` is not an identifier; there is no
automatic conversion of hyphens to underscores. Two files cannot share one
namespace name.

Members of a file shadow imported namespace aliases: a declaration named `io` in
a file that also has `use std.io;` conflicts with that `use` and is an error.

## Standard-library access

The `std` hierarchy is built into Aerofyl and never needs a `use` for full-path
access. `std.io.print` works with no `use` or `using` at all:

```fyl
// io not imported in this file

public void main()
{
    std.io.print("Idk");
}
```

Full-path std access works in every file. Unqualified std access, like
`print("Idk")` with no `use` or `using`, stays a compile error.

If a file has `use std.io;`, its module can be called by the last segment of
the import path:

```fyl
use std.io;

public void main()
{
    io.print("Idk");
}
```

If the file does not have `use std.io;`, `io.print` is an error and the
diagnostic suggests `std.io.print`.

Std module members keep working by full path in both cases: `std.io.print`
works with or without the `use`. Whole-module std aliases use the last segment
of the import path, and every std module casts a unique short name
(`io`, `fs`, `os`), so two whole-module std imports never collide. Localvariables, parameters, and declarations named after an imported std alias are
rejected, as described above.

Standard-library module names require no import for full-path use; an unknown
`std` module is a compile error and is not searched for as a relative file.
Qualified user-module names, directory paths, aliases, re-exports, separate
compilation, and package imports are not supported. TOML is the intended
project configuration format, but the manifest file name and schema and package
management remain unspecified.

## Symbols

Compilers emit one mangled symbol per function, built from the full dotted
namespace path with `__` between segments, for example `file2__File2__function`.
Manglings stay unique across namespaces and files, so identical leaf names in
different namespaces never collide in an executable.
