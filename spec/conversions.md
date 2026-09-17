# Explicit conversions

Conversions use type-call syntax:

```fyl
int scalar = int(character);
char character = char(65);
int discriminant = int(TokenKind.eof);
byte octet = byte(255);
```

The supported conversions are:

| Source | Target | Behavior |
| --- | --- | --- |
| `int` | `int` | identity |
| `byte` | `byte` | identity |
| `byte` | `int` | zero-extension preserving `0...255` |
| `int` | `byte` | checked conversion |
| `char` | `char` | identity |
| `char` | `int` | Unicode scalar value |
| payload-free enum | `int` | zero-based declaration-order discriminant |
| `int` | `char` | the corresponding Unicode scalar value |

`char(int)` terminates with runtime failure status 70 when the value is
negative, greater than `0x10ffff`, or in the surrogate range
`0xd800...0xdfff`. Integer-to-enum conversion is intentionally unavailable;
not every integer denotes a declared variant. Boolean, string, collection,
struct, and cross-enum conversions are rejected.

`byte(int)` terminates with runtime failure status 70 when the integer is less
than zero or greater than 255. It never truncates or wraps.

Conversions are otherwise explicit. The existing contextual conversion of a
character literal to `int` remains the sole implicit exception, for concise
byte-oriented code such as `source.byte(i) == '0'`. `dynamic` conversion rules
remain unspecified.
