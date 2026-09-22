# Types

Known basic type names are:

- `int`
- `byte`
- `float`
- `bool`
- `char`
- `string`
- `void`
- `dynamic`

Named structs and enums are types. Fixed arrays use `T[N]`; lists use `list T`;
optional values use `optional T`. The bootstrap reserves `string[]` only for
the sole parameter of `public void main`.
Explicit references use `ref T`; see [references.md](references.md).

The bootstrap requires exact type matches except that a character literal may
be used where an `int` is expected; its integer value is its Unicode scalar
value. This makes ASCII byte operations readable, for example
`source.byte(i) == '0'`. The distinct `byte` type is unsigned and restricted to
`0...255`; `char` remains a Unicode scalar type. For non-ASCII characters,
UTF-8 byte values are not the same as the character's scalar value. Other
conversions, coercions, promotion, and the behavior of `dynamic` are not
specified.

`int` is signed 64-bit in the current language, with arithmetic rules defined
in [integers.md](integers.md). `bool` stores `false` or `true`, and `char` is a
Unicode scalar value. Decimal integer tokens in a context requiring `byte` are
byte literals only when their value is at most 255. Explicit integer, byte,
character, and enum conversions are
defined in [conversions.md](conversions.md). Type alignments, a stable ABI, and
floating-point execution remain unspecified; binary64 remains only the
reserved bootstrap representation for `float`.

General generics, raw pointers, and tuple values are not specified yet.
