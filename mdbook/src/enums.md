# Enums

The bootstrap implements payload-free enums and qualified variants.

```fyl
enum State
{
    idle,
    running
}

State state = State.running;
```

Equality and inequality are available between values of the same enum. Payloads and explicit discriminants are not specified yet.
