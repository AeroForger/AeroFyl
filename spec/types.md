# Types

Known basic type names are:

- `int`
- `float`
- `bool`
- `char`
- `string`
- `void`
- `dynamic`

Named structs and enums are types. Fixed arrays use `T[N]`; lists use `list T`. The bootstrap reserves `string[]` only for the sole parameter of `public void main`.

The bootstrap requires exact type matches because conversions, coercions, promotion, and the behavior of `dynamic` are not specified yet. This exact-match rule is a temporary compiler restriction unless separately specified.

The language-level sizes, alignments, numeric ranges, overflow behavior, floating-point behavior, and stable ABI are not specified yet. The bootstrap currently executes `int` as signed 64-bit, stores `bool` as zero or one, reserves binary64 for `float`, and represents `char` as a Unicode scalar value. Floating-point executable lowering is not implemented.

General generics, pointers, references, ownership, and tuple values are not specified yet.
