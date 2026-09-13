# Functions

A function declares visibility, a return type, a name, parameters, and a body.

```fyl
public int add(int x, int y)
{
    return x + y;
}
```

Calls use positional arguments. The bootstrap checks argument counts and exact types. It accepts `public` and `private`; overloading and default arguments are not specified.
