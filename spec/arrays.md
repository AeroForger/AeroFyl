# Fixed arrays

Fixed array types include an element type and length.

```fyl
int[4] nums = [1, 2, 3, 4];
```

The literal element count must match the declared length in the bootstrap. Indexes have type `int`; indexed reads, indexed writes, and `.length` are supported. Runtime bounds checks terminate with the defined failure status 70.

Arrays may contain supported arrays, lists, structs, optionals, and scalar
values. Nested indexing is supported. The bootstrap represents an array value
as an owned process-lifetime handle so it can be stored in a struct or another
collection. Whole-array copying, array type inference, array parameters and
returns, reclamation, and a stable representation remain unspecified.
