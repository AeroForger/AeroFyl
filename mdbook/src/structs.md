# Structs

Structs contain named fields. Construction names every field.

```fyl
struct Point
{
    int x;
    int y;
}

Point point = Point { x: 10, y: 20 };
point.x = 11;
```

The bootstrap supports whole-struct copies for its supported layouts. This layout and copy mechanism are temporary.
