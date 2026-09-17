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

Every declared field must appear exactly once in construction; construction
order is irrelevant, and unknown, missing, or duplicate fields are errors.
Fields use exact types. Field access and assignment are supported on local
struct variables.

Struct assignment, initialization from another struct local, list insertion,
list loading, and list removal use value-copy behavior. Mutating a loaded or
copied struct does not mutate the source value. String fields may share their
immutable string storage. Struct equality is not defined.

Struct declarations currently have no visibility modifier and are available
through the loaded import graph. Stage 0 supports fields of `int`, `bool`,
`char`, `string`, and payload-free enum types. Nested structs, collection
fields, struct parameters and returns, field visibility, defaults, methods,
inheritance, interfaces, and a stable memory layout are not supported.
