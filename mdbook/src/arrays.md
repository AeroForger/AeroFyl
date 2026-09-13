# Arrays

A fixed array includes its length in the type.

```fyl
int[4] values = [1, 2, 3, 4];
values[0] = values[3];
int count = values.length;
```

The bootstrap checks indexes at runtime. Its failure behavior and storage layout are temporary implementation details.
