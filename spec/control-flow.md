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

`if` and `while` conditions have type `bool` in the bootstrap. `break` and `continue` are accepted only inside a loop. `else if`, `for`, `foreach`, `switch`, and unreachable-code rules are not specified yet.
