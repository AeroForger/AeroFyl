# Semantics

Names must resolve to declarations in scope. A local variable declaration includes a type, name, initializer, and mandatory semicolon. Assignment targets an existing variable, struct field, array element, or list element supported by the bootstrap.

Conditions for `if` and `while` have type `bool`. `&&` and `||` short-circuit in the bootstrap. Function call argument count and types must match the declaration. A non-`void` function must have a return recognized on every path; current loop return analysis is conservative.

The bootstrap rejects duplicate top-level type names, duplicate functions, duplicate struct fields, duplicate enum variants, unknown names, invalid assignment targets, and exact type mismatches. These checks describe current accepted programs where the language specification is incomplete; they do not establish rules for overloading, shadowing, conversions, or unreachable code.

Integer overflow and division failure semantics are not specified yet. Evaluation order beyond short-circuit Boolean operators is not specified yet.
