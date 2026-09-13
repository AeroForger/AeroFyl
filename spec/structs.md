# Structs

Structs declare named fields in a block. A value is constructed with every field named exactly once.

```fyl
struct Point
{
    int x;
    int y;
}

Point point = Point {
    x: 10,
    y: 20
};
```

The bootstrap supports field access, field assignment, whole-value copying, and lists of supported structs. Its restricted field layout and copy representation are temporary. Field visibility, omitted or default fields, nested aggregate layouts, user-defined methods, inheritance, and interfaces are not specified yet.
