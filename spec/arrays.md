# Fixed arrays

Fixed array types include an element type and length.

```fyl
int[4] nums = [1, 2, 3, 4];
```

The literal element count must match the declared length in the bootstrap. Indexes have type `int`; indexed reads, indexed writes, and `.length` are supported. Runtime bounds checks terminate with the defined failure status 70.

Whole-array copying, array type inference, multidimensional arrays, supported element types beyond the bootstrap subset, and language-level bounds failure behavior are not specified yet.
