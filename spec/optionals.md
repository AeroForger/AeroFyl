# Optional values

`optional T` explicitly represents either a value of exact type `T` or no
value. It does not make `T` nullable and does not add an implicit conversion.

```fyl
optional int found = some(42);
optional int missing = none();

if (found.hasValue)
{
    int value = found.value;
}
```

`some(expression)` requires exactly one expression of type `T`. `none()` takes
no arguments. Both constructors require an `optional T` context, such as a
declared variable, field, argument, or return type; AeroFyl does not infer a
free-standing optional type.

`.hasValue` returns `bool`. `.value` returns `T` when a value exists. Accessing
`.value` on `none()` terminates the bootstrap executable with runtime failure
status 70. There is no implicit default value, truth conversion, or automatic
unwrapping. The Stage-1 bootstrap stores an optional in a tagged,
process-lifetime allocation; that representation is not a stable ABI.
