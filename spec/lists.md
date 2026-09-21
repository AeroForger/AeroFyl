# Lists

Lists use an element type and a bracketed literal.

```fyl
list int nums = [1, 2, 3];
```

An empty literal requires a declared list type. Elements use the declared exact
type. Known operations are `values.length`, `values.push(value)`,
`values.pop()`, `values[index]`, and `values[index] = value;`. `.length` is
nonnegative, `push` appends one element, and `pop` removes and returns the last
element. Indexing requires `0 <= index < length`. Invalid indexing and popping
an empty list terminate with runtime failure status 70.

Stage 1 supports nested supported list and array types in addition to `int`,
`byte`, `bool`, `char`, `string`, payload-free enum, optional, and supported
struct elements. It grows full storage while preserving all existing values.
Scalar and immutable-string elements copy by value. Struct elements
are copied completely on push, load, indexed assignment, and pop, so mutating a
loaded struct does not mutate the stored element.

`list byte` stores each element in exactly one byte of contiguous element
storage. Its header still records length, capacity, and stride. Other scalar
list elements currently use eight-byte slots.

Standalone list locals have one owner in the current subset. A function may
construct and return a list, and a caller may initialize a local from that
returned value. A list handle stored in a struct field is copied with the field
slot and may therefore share indexed mutations; this does not enable relocation
operations through the field. List
parameters provide read-only access (`.length` and indexing); indexed
assignment, `.push`, and `.pop` through a parameter are rejected. Initializing
from another list local and whole-list assignment are also rejected, avoiding
implicit aliasing while growth may relocate storage.

Nested indexing and `.length` work on list-valued expressions. `.push` and
`.pop` still require an owning local list because growth may relocate the list;
they are not available directly through a nested expression or struct field.

Storage is retained for the process lifetime in Stage 1. General inference,
explicit ownership transfer beyond function results, reclamation, and stable
representation remain unspecified.
