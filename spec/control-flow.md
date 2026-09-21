# Control flow

Known control-flow statements are `if`, `else`, `while`, `break`, `continue`, and `return`.

```fyl
while (condition)
{
    if (done)
    {
        break;
    }
}
```

`if` and `while` conditions have type `bool` in the bootstrap. `else if` chains
are parsed as an `if` nested in the preceding `else`, with the same behavior as
an explicitly nested form. `break` and `continue` are accepted only inside a
loop. `for`, `foreach`, `switch`, and unreachable-code rules are not specified
yet.
