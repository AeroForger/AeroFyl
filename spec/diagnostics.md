# Diagnostics and source locations

The compiler frontend tracks every token and syntax node with a source-file ID
and a half-open byte span `[start, end)`. Source maps retain file paths, source
text, and line-start offsets. Diagnostic rendering resolves the start offset to
one-based line and Unicode-scalar column numbers and marks the relevant span.

Self-hosted compiler code can represent locations and diagnostics with ordinary
supported structs. A practical flat representation is:

```fyl
struct SourceSpan
{
    string file;
    int startOffset;
    int endOffset;
    int startLine;
    int startColumn;
}

struct Diagnostic
{
    string message;
    string file;
    int startOffset;
    int endOffset;
    int line;
    int column;
}
```

Offsets count UTF-8 bytes, lines and columns are one-based, and spans are
half-open. A Stage-2 lexer can compute them using string byte operations while
scanning. Flat fields are currently required because nested struct fields are
not supported. There is not yet a runtime stderr or formatted-print API;
diagnostic text can be written to a chosen file with `writeFile` or used as a
process result alongside `exit`.
