# Control flow

Known control-flow statements are `if`, `else`, `while`, `for`, `break`, `continue`, and `return`.

```fyl
while (condition)
{
    if (done)
    {
        break;
    }
}
```

`if`, `while`, and `for` conditions have type `bool` in the bootstrap. A `for`
loop has an explicit declaration-or-assignment initializer, condition, and
assignment increment:

```fyl
for (int i = 0; i < count; i += 1)
{
    // body
}
```

The initializer runs once. The condition is checked before every iteration.
The increment runs after a normal body completion and before a `continue`; a
`break` exits without running it. The initializer name is scoped to the loop.

`else if` chains
are parsed as an `if` nested in the preceding `else`, with the same behavior as
an explicitly nested form. `break` and `continue` are accepted only inside a
loop. `foreach`, `switch`, and unreachable-code rules are not specified yet.
