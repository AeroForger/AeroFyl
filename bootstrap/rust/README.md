# Aerofyl Rust bootstrap compiler

This crate is a temporary, dependency-free compiler skeleton. It deliberately
implements only the language facts currently recorded in the repository task.

## Implemented pipeline

The driver loads a `.fyl` file and runs source mapping, lexing, handwritten
parsing, name resolution, exact-match semantic checks, HIR construction, and
lowering to a small backend-independent IR. The middle-end verifies that IR
before `CompileOptions::check()` returns or x86-64 lowering begins.

The x86-64 package lowers the milestone integer subset using the System V AMD64
ABI, encodes machine instructions directly, generates Linux startup code, and
wraps it in a complete ELF64 executable. No assembler, linker, libc, or external
compiler is used.

## IR verification

`middle::verify::verify_module` runs immediately after HIR-to-IR lowering. The
normal compilation path is therefore:

```text
source -> AST -> HIR -> IR -> verify IR -> x86-64 -> ELF
```

Verification checks that function, block, value, and local identifiers are
valid; entry blocks and branch targets exist; every block has a terminator; and
all blocks are reachable. Unreachable blocks are hard verification failures in
this milestone. A terminator is stored separately from a block's instruction
list, so the representation cannot express instructions after a terminator.

The verifier also checks that values are defined on every incoming control-flow
path before use, locals exist and have consistent types, operations have the
required operand and result types, branch conditions are Boolean, calls resolve
to matching function signatures, and returns match their function type.
Short-circuit expressions use one logical result defined in path-disjoint
predecessors of the same merge block; other multiple definitions are rejected.

Verification returns all reasonably independent problems it finds as structured
`IrVerificationError` values. Driver diagnostics label these failures as
`internal compiler error: invalid IR`: they indicate a compiler bug after valid
source has passed semantic analysis, and are distinct from source diagnostics
that include source locations. A failed verification stops compilation before
the backend is called.

## Supported source forms

- `public` and `private` function declarations
- the documented basic types
- type/name parameters
- fixed arrays such as `int[4]`
- lists such as `list int`
- tuple return types containing types only
- initialized local declarations
- `return` statements and semicolon-terminated call expressions
- identifier, decimal integer, decimal float, string, character, call, and
  bracketed collection expressions
- signed 64-bit integer literals, unary negation, and precedence-aware `+`, `-`,
  `*`, and `/`
- `true` and `false`, unary `!`, short-circuit `&&` and `||`
- integer `==`, `!=`, `<`, `<=`, `>`, and `>=`; Boolean `==` and `!=`
- `if`/`else`, `while`, `break`, `continue`, and exact-type variable assignment

Executable lowering currently supports `int` parameters, `int` and `void`
returns, integer locals, integer arithmetic, and calls. An executable must have
exactly one `public void main()` with no parameters. Linux startup calls it and
then exits with status zero through the x86-64 `exit` syscall.

For this milestone, `int` is a signed 64-bit value. The reserved primitive
representations are IEEE-754 binary64 for `float` and a `u32` Unicode scalar
value for `char`; executable lowering for those two types is intentionally not
present. Boolean values are canonical `0` (false) or `1` (true) in registers and
occupy one eight-byte bootstrap stack slot, matching the simple IR value layout.

The deliberately simple backend gives every parameter, local, and IR temporary
an eight-byte stack slot. It uses fixed caller-saved registers for operations,
passes the first six integer arguments in the SysV registers, passes remaining
arguments on the stack, and preserves 16-byte call-site stack alignment.

Integer addition, subtraction, multiplication, and negation use ordinary x86-64
two's-complement machine behavior. Signed division uses `cqo`/`idiv`; division by
zero and the machine's signed-division overflow case trap normally. Aerofyl-level
overflow and trap semantics remain unspecified, so this bootstrap does not claim
more than the underlying machine behavior.

Conditions require `bool`; integers are not converted implicitly. Logical `&&`
and `||` lower into CFG branches, so their right operand is evaluated only when
required. The backend-independent IR uses basic blocks with conditional branch,
unconditional jump, and return terminators. The encoder resolves calls and both
forward and backward branches with signed rel32 fixups.

Integer return analysis recognizes direct returns and `if`/`else` where both
branches return. It is deliberately conservative for loops because Aerofyl has
not specified unreachable-code or infinite-loop rules. There is no `else if`
shorthand, `for`, `foreach`, `switch`, or compound assignment in this milestone.

String and character escape processing is intentionally absent. A backslash in
a string is ordinary text; character literals contain one Unicode scalar value.

## Specification still required

The following decisions are intentionally not made by the bootstrap:

- the exact identifier alphabet (the lexer currently uses a minimal Unicode
  alphabetic/underscore convention, marked with a source TODO)
- comment syntax
- alternate integer spellings and Aerofyl-level overflow/trap semantics
- floating-point code generation and runtime representation
- string/character escapes and runtime representation
- `use` and `using` grammar, module lookup, and imported-name behavior
- tuple value syntax
- standalone collection inference and collection representation
- implicit conversions, coercions, promotion, `dynamic` behavior, and other
  type-system relationships
- builtin signatures and behavior for `print` and `input`
- richer control flow such as `for`/`foreach` and conditionals beyond the forms
  documented above
- project manifest file name and TOML key schema

Unsupported constructs retain their frontend/IR interfaces but executable
lowering returns structured diagnostics or backend errors. Assignment is a
statement targeting an existing variable; compound assignments are unsupported.

The bootstrap-only executable command is:

```text
cargo run -p aerofyl-bootstrap -- compile <source.fyl> -o <executable>
```
