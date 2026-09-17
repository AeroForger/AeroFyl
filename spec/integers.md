# Integers

`int` is a signed 64-bit two's-complement value in the current language.
Decimal literals must fit the range `-9223372036854775808` through
`9223372036854775807`.

Addition, subtraction, multiplication, and unary negation wrap modulo `2^64`.
The wrapped bits are interpreted again as a signed two's-complement value.
Consequently `9223372036854775807 + 1` is `-9223372036854775808`, and negating
`-9223372036854775808` produces the same value.

Division is signed and truncates toward zero. Division by zero and the one
unrepresentable quotient, `-9223372036854775808 / -1`, terminate with runtime
failure status 70. These checks are explicit runtime behavior and do not depend
on an accidental processor exception.
