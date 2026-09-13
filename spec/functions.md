# Functions

Functions have explicit visibility, return type, name, parameters, and a block body.

```fyl
public int add(int x, int y)
{
    return x + y;
}
```

Known visibility values are `public` and `private`. Parameters have explicit types. Calls use the function name followed by parenthesized arguments. Return statements and call-expression statements end with semicolons.

Overloading, variadic functions, default arguments, nested functions, and stable calling conventions are not specified yet. The bootstrap does not lower every parsed type through its executable function ABI.
