# Lexical structure

Aerofyl source files use the `.fyl` extension. Keywords are lowercase.

## Identifiers

The exact identifier alphabet is not specified yet. The bootstrap accepts an underscore or Unicode alphabetic character first, followed by those characters or Unicode numeric characters.

## Keywords

The known keywords are `public`, `private`, `use`, `using`, `return`, `if`, `else`, `while`, `break`, `continue`, `true`, `false`, `print`, `input`, `list`, `int`, `float`, `bool`, `char`, `string`, `void`, `dynamic`, `struct`, and `enum`.

`use`, `using`, `print`, and `input` are reserved by the bootstrap, but their language grammar or behavior is not specified yet.

## Literals

Known literal forms are decimal integers, decimal floating-point values with digits on both sides of the decimal point, double-quoted strings, single-character literals, `true`, `false`, and bracketed collection literals.

String escape behavior is not fully specified yet. Character escape behavior is not specified yet. The bootstrap treats a backslash in a string as ordinary text and requires exactly one Unicode scalar value in a character literal.

## Whitespace and comments

Whitespace separates tokens and is otherwise ignored by the bootstrap. Comments are not specified yet.

## Punctuation and operators

Known punctuation is `(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:`, and `.`. Known operators are `+`, `-`, `*`, `/`, `=`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `!`, `&&`, and `||`.
