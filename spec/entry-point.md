# Entry point

The known entry-point form is:

```fyl
public void main()
{
}
```

The other legal signature is:

```fyl
public void main(string[] args)
{
}
```

An executable's root module must have exactly one of these signatures. Missing,
multiple, private, non-`void`, or otherwise parameterized entry points are
compile errors when emitting an executable. Imported modules do not provide the
root entry point.

`string[]` is a read-only command-line argument collection valid only as the
sole parameter of `main`. The native executable path (`argv[0]`) is excluded.
Each subsequent OS argument is copied, in order, into an immutable AeroFyl
byte string. With no user arguments, `.length` is zero. Indexed access uses the
same checked behavior as other argument/list access and fails with status 70
when out of bounds.

Programs terminate normally with status zero when `main` returns. `exit(code)`
terminates the process immediately and does not return. It accepts exactly one
`int`. On the Stage 0 Linux target, the environment observes the low eight bits
of the code, so `exit(300)` produces status 44. A direct `exit` expression is a
terminal path for return analysis.

No other entry-point signature is currently legal. A future general array type
or alternate platform ABI may replace the special `string[]` representation
without changing the source-level signatures above.
