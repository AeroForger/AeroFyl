# Frontend

`parse_main.fyl` is the parser command line entry point. It prints `parse
succeeded` on success, or source paths, byte offsets, and diagnostic messages on
failure. Its statuses are 0 for valid syntax, 1 for lexical/syntax errors, and 2
for incorrect arguments. Filesystem failures retain the standard library's
runtime failure behavior.

The parser supports imports and selective imports, structs, payload enums,
functions, parameters, basic and named types, list/optional/reference types,
fixed arrays, `string[]`, tuple return types, variables, assignments and
compound assignments, returns, conditionals and `else if`, `while`, `for`,
`break`, and `continue`. Expressions retain precedence and left associativity,
with unary operators, conversions, named calls, members, methods, indices,
collections, struct literals, and interpolation parts. Struct literals accept a
trailing comma. Enum variants follow the bootstrap's comma-separated or adjacent
forms. Tuple types are supported; tuple value syntax is not specified.

## API

- `lexSource(Source source)` returns tokens and lexical diagnostics without file I/O.
- `lex(string path)` loads and tokenizes one file.
- `parseTokens(Source source, list Token tokens)` parses a token stream without file I/O.
- `parse(string path)` loads, lexes, and parses one file.

`ParseResult` contains `program` and `diagnostics`. A successful result has no
diagnostics. A lexical failure preserves the scanner's diagnostics; a syntax
failure reports the first error and returns an empty AST with `rootPath` set.
The token API copies the input token list, synthesizes a missing EOF, handles an
empty list, and rejects tokens following EOF. It expects tokens produced by the
lexer, with valid payloads and byte spans.

Recursive type, expression, and control-flow parsing is bounded to 96 active
nesting-helper entries. Long binary and postfix chains are parsed iteratively.
Loop-bearing helpers start with an early-return `while` guard: returning from
its body executes at most once, while keeping the Rust bootstrap's entry block
free of value definitions. This avoids a convergence defect in its loop verifier
without modifying the bootstrap or bypassing verification.

## AST

`ast.fyl` keeps the existing arena representation. Every child and root is an
index into `AstProgram.nodes`. `roots` contains top-level declarations in source
order; `imports` contains import records separately. Spans are half-open byte
ranges in the input file. Parenthesized expressions retain their enclosing span.

| Node kind | `text` / `number` | Children / names |
| --- | --- | --- |
| `integer`, `floatValue`, `stringValue` | Literal text or decoded string | None |
| `character`, `boolean` | Scalar codepoint or 0/1 in `number` | None |
| `name` | Identifier spelling | None |
| `unary`, `binary` | Operator spelling | Operand, or left/right |
| `call` | Empty | Callee, then arguments; method callees are `member` nodes |
| `member`, `index` | Member name, or empty | Base, or base/index |
| `collection`, `interpolated` | Empty | Values, or decoded text and variable-name parts |
| `structValue` | Struct name | Values; corresponding field names in `names` |
| `typeSyntax` | Basic/named type, `list`, `optional`, `ref`, `tuple`, `array`, or `args` | Inner types; array length text in `names[0]` |
| `variable`, `assignment` | Variable name, or assignment operator | Type/initializer, or target/value |
| `expressionStatement`, `returnValue` | Empty | Expression, or optional return value |
| `block` | Empty | Statements in order |
| `ifStatement` | Empty | Condition, then block, optional else block or nested if |
| `whileStatement`, `forStatement` | Empty | Condition/body, or initializer/condition/increment/body |
| `breakStatement`, `continueStatement` | Empty | None |
| `function` | Function name; `number` is 1 for public, 0 for private | Return type, parameters, body |
| `parameter`, `fieldDeclaration` | Declared name | Type |
| `structDeclaration`, `enumDeclaration` | Declared name | Fields or variants |
| `variantDeclaration` | Variant name; declaration-order index in `number` | Optional payload type |

Numeric expression literals retain text for later checking, including signed
integer boundary cases handled through unary minus. Array lengths retain text
without signed conversion but must fit the bootstrap's 64-bit size range.
`args` represents `string[]`; no other type can omit an array length. Compound
assignment operators are retained explicitly for later lowering.

`AstImport.target` starts empty; resolving it belongs to the next phase.
Duplicate imports and duplicate selected names are syntax diagnostics. Name
resolution, duplicate declarations, type checking, loop-context checks, and the
restriction of interpolation to direct standard I/O output arguments belong to later
analysis. The frontend does not load imported source files while parsing input.

`parser_tests.fyl` checks AST structure and diagnostics. The broader runner is
`compiler/tests/parser.py`. Its Rust syntax comparisons deliberately account for
improved declaration lookahead (named arrays, conversion statements, and
parenthesized expression statements), and for rejecting invalid assignment
targets before semantic analysis.
