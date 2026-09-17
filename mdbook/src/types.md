# Types

Known basic types are `int`, `byte`, `float`, `bool`, `char`, `string`, `void`, and `dynamic`. Named structs and enums are also types. Fixed arrays include their length, such as `int[4]`, while lists use forms such as `list int`.

The executable bootstrap currently handles a narrower set than the parser. In
particular, floating-point executable lowering and defined `dynamic` behavior
are absent. Most types require exact matches. `int(value)` explicitly converts
characters and enum values to integers, while `char(value)` converts a valid
Unicode scalar integer to `char`. `byte` is an unsigned value from 0 through
255; `byte(int)` checks that range and `int(byte)` preserves the value.

`int` is signed 64-bit. Addition, subtraction, multiplication, and negation
wrap; signed division truncates toward zero and fails with status 70 for zero
or the `INT64_MIN / -1` case.
