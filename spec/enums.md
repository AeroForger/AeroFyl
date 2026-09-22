# Enums

Enum variants may be payload-free or carry one exact typed payload.

```fyl
enum State
{
    idle,
    running,
    stopped
}

State state = State.running;

enum Result { ok(int), error(string) }
Result result = Result.ok(42);
if (result.is(Result.ok)) {
    int value = result.payload(Result.ok);
}
```

Variants are qualified by their enum name. Variant names must be unique.
Values copy by value and support equality and inequality with the same enum
type. Different enum types do not compare. `int(value)` exposes a stable
zero-based discriminant assigned in declaration order. This source-level
mapping does not make the in-memory representation a stable external ABI. Enum
declarations currently have no visibility modifier and are available through
the loaded import graph.

An enum with any payload variant uses a tagged two-slot allocation: a
declaration-order tag and one payload handle/value slot. `is(Enum.Variant)`
tests the tag. `payload(Enum.Variant)` returns that variant's declared type and
performs a runtime tag check; a mismatched extraction fails with status 70.
Payload enums are suitable for explicit recoverable results such as
`Result.ok(value)` and `Result.error(diagnostic)`. Propagation uses ordinary
conditionals and returns; there is no hidden exception control flow.

Payload-enum equality, explicit discriminants, unqualified variants, and
ordering comparisons are not specified.
