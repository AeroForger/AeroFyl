# Semantics

Names must resolve to declarations in scope. A local variable declaration includes a type, name, initializer, and mandatory semicolon. Assignment targets an existing variable, struct field, array element, or list element supported by the bootstrap.

Conditions for `if` and `while` have type `bool`. `&&` and `||` short-circuit in the bootstrap. Function call argument count and types must match the declaration. A non-`void` function must have a return recognized on every path; current loop return analysis is conservative.

The bootstrap rejects duplicate program-wide top-level type names and function
names, duplicate struct fields, duplicate enum variants, unknown or inaccessible
names, invalid assignment targets, and exact type mismatches. Private functions
are visible only in their declaring file. Character-literal conversion to
`int` and an in-range decimal literal in an expected `byte` context are the
currently defined exceptions to exact matching. These rules do
not establish overloading, general conversions, or unreachable-code behavior.

Integer wrapping and checked signed-division behavior are specified in
[integers.md](integers.md). Evaluation order beyond short-circuit Boolean
operators is not specified yet.
