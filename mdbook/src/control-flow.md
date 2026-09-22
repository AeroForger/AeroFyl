# Control Flow

The implemented subset contains `if`, `else`, `while`, `for`, `break`, `continue`, and `return`. Conditions are Boolean.

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

`for` uses an explicit initializer, Boolean condition, and assignment increment:

```fyl
for (int i = 0; i < count; i += 1)
{
    // body
}
```

The initializer runs once; the condition is checked before every iteration.
The increment runs after the body and before `continue`; `break` exits directly.
