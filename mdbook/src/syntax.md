# Syntax

Aerofyl files use `.fyl`. Keywords are lowercase and statements such as declarations, assignments, calls, `return`, `break`, and `continue` end with semicolons.

```fyl
public void main()
{
    int value = 40 + 2;
}
```

The bootstrap recognizes identifiers, decimal integer and float literals, string literals, character literals, Boolean literals, grouping, calls, collection literals, struct literals, member access, indexing, and the currently documented operators.

Character literals support `\n`, `\r`, `\t`, `\0`, `\\`, and `\'`. String
escapes and comments are not specified. A module imports a sibling file with
`use name;`; `use std.io;` and `use std.fs;` import built-in standard-library
modules. See the `spec/` directory for the complete current rules.
