# Memory and value behavior

The current model is deliberately small and uses process-lifetime allocation.
There is no raw pointer arithmetic, destructor, or garbage collector. `ref T`
is the explicit indirection described in [references.md](references.md).

`int`, `byte`, `bool`, `char`, and payload-free enum values copy independently on
assignment and when passed or returned. Strings are immutable handles to
length-prefixed byte storage. Copying a string may alias the same bytes, which
is unobservable because strings cannot be mutated. Concatenation and slicing
allocate new storage.

Supported structs are heap records with value-copy behavior for local
initialization, local assignment, function parameters and returns, and list
insertion/loading/removal. Every field slot is copied. Immutable strings and
collection fields are handles: copying their field slot shares the underlying
immutable bytes or mutable collection storage. This is explicit shallow handle
copying, not an implicit collection conversion or clone.

Standalone list locals have one owner in the current subset. List literals allocate a header and
contiguous element storage. Local indexed assignment, push, and pop mutate that
owned list. Growth doubles capacity and may relocate the complete allocation;
the owning local is updated to the new address and all elements are preserved.
A function may return a newly owned list and the caller may bind that result.
List parameters are read-only and therefore cannot create a stale mutable alias
during relocation. Initializing from another list local and whole-list
assignment are rejected. Struct field-slot copying can share a list handle, but
push and pop require an owning list local and cannot relocate through that
shared field expression.

A `byte` has the value range `0...255`. Byte-list element storage uses one byte
per value. Standalone expression and local evaluation may use machine-sized
registers or stack slots internally, but no value outside that range can be
created through a valid `byte` operation.

Fixed arrays use process-lifetime heap storage behind a one-word handle. This
allows nested arrays and array fields without exposing a user-visible pointer.
Copying, passing, or returning an array copies this handle and shares indexed
storage; no array clone is implied.
Optional values use a process-lifetime two-slot record containing an explicit
presence tag and one value slot.

References use a one-slot allocation. Payload enums use a two-slot allocation
containing a tag and payload. Nested structs remain separate records connected
by one-word handles. These explicit shallow-handle rules avoid hidden recursive
copying and give recursive compiler data finite layouts.

String data, lists, arrays, references, payload enums, optionals, struct records, file buffers, command-line argument copies,
and temporary runtime buffers remain allocated until process exit. Nothing is
reclaimed early. This lifetime policy is defined for Stage 0 but is not a
stable layout or ABI promise.
