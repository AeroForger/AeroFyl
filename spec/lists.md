# Lists

Lists use an element type and a bracketed literal.

```fyl
list int nums = [1, 2, 3];
```

Known operations are `values.length`, `values.push(value)`, `values.pop()`, `values[index]`, and `values[index] = value;`. The bootstrap checks indexes at runtime and exits with status 70 for an out-of-bounds access or an empty pop.

The bootstrap supports selected scalar, enum, and struct element layouts. Storage growth, process-lifetime allocation, and the lack of whole-list copying are bootstrap behavior. General element-type rules, inference, ownership, reclamation, and stable representation are not specified yet.
