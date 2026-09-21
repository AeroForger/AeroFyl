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
Fields use exact types. Field access works on struct-valued expressions; field
assignment currently requires a local struct variable.

Struct assignment, initialization from another struct local, list insertion,
list loading, and list removal use value-copy behavior. Mutating a loaded or
copied struct does not mutate scalar fields in the source value. String fields
may share their immutable string storage. Supported list and array fields copy
their handle slot; indexed mutations therefore affect the same collection
storage in both struct copies. No collection clone is implied. Struct equality
is not defined.

Struct declarations currently have no visibility modifier and are available
through the loaded import graph. Stage 1 supports fields of `int`, `byte`,
`bool`, `char`, `string`, payload-free enum, supported list, fixed-array, and
optional types. Supported structs can be function parameters and return values.
Calls copy the struct record before the callee receives it, and returning a
local struct copies the record. Collection handles inside that record retain
the sharing rule above. Direct nested struct fields, field visibility,
defaults, methods, inheritance, interfaces, and a stable memory layout are not
supported.
