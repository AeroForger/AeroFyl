# Modules and imports

Each `.fyl` file is a module. A file imports another module with a top-level
declaration:

```fyl
use lexer;
```

To import only named declarations from a file, use `using`:

```fyl
using lexer: scan, Token;
```

`using lexer: scan, Token;` loads `lexer.fyl` and makes only `scan` and `Token`
available to the importing file. `use lexer;` loads the whole file. The same
file resolution rule applies to both forms. A `using` declaration lists names
separated by commas and ends with `;`. Functions and named types can be
selected. The language currently has no module-level variable declarations to
select. Importing a private function from another file remains an error.
Selecting a name not declared by that file is an error. Selective imports of
standard-library modules are not specified; use `use std.io;` or `use std.fs;`.

`use lexer;` resolves only to `lexer.fyl` in the importing file's directory.
The command-line source file is the root module. Imports are recursive, and a
module reached through more than one dependency is loaded once.

All loaded declarations participate in one program-wide namespace for collision
checking. A public function or named type can be referenced from an importing
module only if a `use` chain exposes its file or a `using` declaration selects
that name. A `using` declaration does not expose other names from its file.
Private functions may be referenced only from the file that declares them.
Struct and enum declarations have no visibility modifier in the current
language. Imports are transitive along `use` chains; a `using` edge exposes
only its selected names.

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

Qualified user-module names, directory paths, aliases,
re-exports, separate compilation, and package imports are not supported.
TOML is the intended project configuration format,
but the manifest file name and schema and package management remain
unspecified.
