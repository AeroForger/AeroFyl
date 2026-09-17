# Lexical structure

Aerofyl source files use the `.fyl` extension. Keywords are lowercase.

## Identifiers

The exact identifier alphabet is not specified yet. The bootstrap accepts an underscore or Unicode alphabetic character first, followed by those characters or Unicode numeric characters.

## Keywords

The known keywords are `public`, `private`, `use`, `using`, `return`, `if`, `else`, `while`, `break`, `continue`, `true`, `false`, `print`, `input`, `list`, `int`, `byte`, `float`, `bool`, `char`, `string`, `void`, `dynamic`, `struct`, and `enum`.

`using` remains reserved without language behavior. `print` and `input` are
reserved names belonging to the explicitly imported `std.io` API. `use`
introduces a module import.

`byte` is soft in identifier positions so the established
`string.byte(index)` member remains source-compatible; it denotes the byte type
and conversion where a type or primary expression is expected.

## Literals

Known literal forms are decimal integers, decimal floating-point values with digits on both sides of the decimal point, double-quoted strings, single-character literals, `true`, `false`, and bracketed collection literals.

The bootstrap treats a backslash in a string as ordinary text; string escape
syntax remains unspecified. A character literal contains exactly one Unicode
scalar value or one of these escapes: `\n`, `\r`, `\t`, `\0`, `\\`, or `\'`.
Other character escapes are errors.

## Whitespace and comments

Whitespace separates tokens and is otherwise ignored by the bootstrap. Comments are not specified yet.

## Punctuation and operators

Known punctuation is `(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:`, and `.`. Known operators are `+`, `-`, `*`, `/`, `=`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `!`, `&&`, and `||`.
