# Syntax

Aerofyl files use `.fyl`. Keywords are lowercase and statements such as declarations, assignments, calls, `return`, `break`, and `continue` end with semicolons.

```fyl
public void main()
{
    int value = 40 + 2;
}
```

The bootstrap recognizes identifiers, decimal integer and float literals, string literals, character literals, Boolean literals, grouping, calls, collection literals, struct literals, member access, indexing, and the currently documented operators.

Comments are not specified yet. String and character escape rules are not specified. See the `spec/` directory for the partial grammar and lexical reference.
