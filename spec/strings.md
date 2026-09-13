# Strings

String literals use double quotes. Known operations are:

```fyl
text.length
text.byte(index)
text.slice(start, end)
```

The bootstrap also supports content equality, inequality, and concatenation with `+`. It treats strings as immutable UTF-8 byte sequences. `.length` counts bytes, `.byte(index)` returns an unsigned byte value as `int`, and `.slice(start, end)` copies a half-open byte range. Bounds failures exit with status 70.

String indexing is not supported by the bootstrap. String escape behavior, Unicode character operations, grapheme operations, encoding guarantees for byte slices, memory management, and language-level bounds failure behavior are not fully specified yet.
