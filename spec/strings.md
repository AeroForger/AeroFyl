# Strings

String literals use double quotes. Known operations are:

```fyl
text.length
text.byte(index)
text.slice(start, end)
```

Strings are immutable UTF-8 byte sequences. Equality and inequality compare
byte contents, not storage identity. `.length` is the nonnegative byte length.
`.byte(index)` returns the byte at `index` as an `int` in `0...255` and requires
`0 <= index < length`. `.slice(start, end)` copies the half-open byte range and
requires `0 <= start <= end <= length`; empty and full slices are valid. A
slice is permitted to contain bytes that are not independently valid UTF-8.
Invalid indexes or ranges terminate with runtime failure status 70.

Concatenation with `+` returns a newly allocated string containing the left
bytes followed by the right bytes. Empty-string concatenation is valid and
does not mutate either operand. Literal storage is read-only; concatenation and
slicing allocate independent process-lifetime storage in Stage 0. Copying a
string value may share immutable storage.

String indexing is not supported; use `.byte`. String escape syntax, Unicode
character iteration, grapheme operations, reclamation, and a stable string ABI
are not specified.
