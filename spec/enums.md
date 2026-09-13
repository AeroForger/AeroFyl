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

Variants are qualified by their enum name. The bootstrap supports equality and inequality between values of the same enum. It assigns zero-based discriminants in declaration order, but that mapping is not a stable language ABI.

Payloads, explicit discriminants, unqualified variants, methods, and ordering comparisons are not specified yet.
