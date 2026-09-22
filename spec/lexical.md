# Lexical structure

Aerofyl source files use the `.fyl` extension. Keywords are lowercase.

## Identifiers

The exact identifier alphabet is not specified yet. The bootstrap accepts an underscore or Unicode alphabetic character first, followed by those characters or Unicode numeric characters.

## Keywords

The known keywords are `public`, `private`, `use`, `using`, `return`, `if`, `else`, `while`, `for`, `break`, `continue`, `true`, `false`, `print`, `input`, `list`, `optional`, `int`, `byte`, `float`, `bool`, `char`, `string`, `void`, `dynamic`, `struct`, and `enum`.

`using` remains reserved without language behavior. `print` and `input` are
reserved names belonging to the explicitly imported `std.io` API. `use`
introduces a module import.

`byte` is soft in identifier positions so the established
`string.byte(index)` member remains source-compatible; it denotes the byte type
and conversion where a type or primary expression is expected.

## Literals

Known literal forms are decimal integers, decimal floating-point values with digits on both sides of the decimal point, double-quoted strings, single-character literals, `true`, `false`, and bracketed collection literals.

Strings are byte-oriented and accept `\n`, `\r`, `\t`, `\0`, `\\`, and `\"`.
Each escape contributes the corresponding byte to UTF-8 string storage. Other
string escapes are errors. A character literal contains exactly one Unicode
scalar value or one of these escapes: `\n`, `\r`, `\t`, `\0`, `\\`, or `\'`.
Other character escapes are errors.

## Whitespace and comments

Whitespace separates tokens and is otherwise ignored. `//` starts a comment
that ends before the next line terminator. `/*` starts a block comment that
ends at the next `*/`; block comments do not nest. Reaching end of file before
`*/` is a source-located lexical error.

## Punctuation and operators

Known punctuation is `(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:`, and `.`. Known operators are `+`, `-`, `*`, `/`, `=`, `+=`, `-=`, `*=`, `/=`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `!`, `&&`, and `||`.
