# Control Flow

The implemented subset contains `if`, `else`, `while`, `break`, `continue`, and `return`. Conditions are Boolean.

```fyl
while (value < 10)
{
    value = value + 1;
    if (value == 5)
    {
        break;
    }
}
```

Logical `&&` and `||` short-circuit in the bootstrap. Other control-flow forms are not specified yet.
