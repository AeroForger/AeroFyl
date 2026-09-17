# Modules and imports

Each `.fyl` file is a module. A file imports another module with a top-level
declaration:

```fyl
use lexer;
```

`use lexer;` resolves only to `lexer.fyl` in the importing file's directory.
The command-line source file is the root module. Imports are recursive, and a
module reached through more than one dependency is loaded once.

All imported declarations participate in one program-wide namespace. Public
functions may be referenced from any module in the loaded dependency graph.
Private functions may be referenced only from the file that declares them.
Struct and enum declarations have no visibility modifier in the current
language and are exported to the dependency graph. Imports are transitive: a
public declaration loaded through an imported module is available throughout
the program.

Importing the same name twice in one file is an error. Missing files, circular
imports, and colliding top-level function or type names are errors. The
program-wide namespace means colliding private function names in different
files are also rejected. Diamond-shaped dependency graphs are valid and do not
count as duplicate imports.

The `std` hierarchy is built into Aerofyl and is resolved separately from
relative files. It never requires a package or dependency declaration.
Standard-library modules still require an explicit import; for example,
`use std.io;` brings standard I/O functions into scope, and `use std.fs;`
brings the minimal filesystem API into scope. An unknown `std`
module is a compile error and is not searched for as a relative file.

Qualified user-module names, directory paths, aliases, selective imports,
re-exports, separate compilation, and package imports are not supported.
`using` remains reserved. TOML is the intended project configuration format,
but the manifest file name and schema and package management remain
unspecified.
