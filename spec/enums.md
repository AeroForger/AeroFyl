# Enums

The known enum form contains payload-free variants.

```fyl
enum State
{
    idle,
    running,
    stopped
}

State state = State.running;
```

Variants are qualified by their enum name. Variant names must be unique.
Values copy by value and support equality and inequality with the same enum
type. Different enum types do not compare. `int(value)` exposes a stable
zero-based discriminant assigned in declaration order. This source-level
mapping does not make the in-memory representation a stable external ABI. Enum
declarations currently have no visibility modifier and are available through
the loaded import graph.

Payloads, explicit discriminants, unqualified variants, methods, and ordering comparisons are not specified yet.
