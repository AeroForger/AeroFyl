# Memory and value behavior

The current model is deliberately small and uses process-lifetime allocation.
There is no user-visible pointer, reference, allocator, destructor, or garbage
collector.

`int`, `byte`, `bool`, `char`, and payload-free enum values copy independently on
assignment and when passed or returned. Strings are immutable handles to
length-prefixed byte storage. Copying a string may alias the same bytes, which
is unobservable because strings cannot be mutated. Concatenation and slicing
allocate new storage.

Supported structs are heap records with value-copy behavior for local
initialization, local assignment, and list insertion/loading/removal. Every
field slot is copied. Immutable string fields may still share string storage.
Nested aggregate fields and struct parameters or returns are not supported, so
no aggregate aliasing is exposed through those operations.

Lists have one owner in the current subset. List literals allocate a header and
contiguous element storage. Local indexed assignment, push, and pop mutate that
owned list. Growth doubles capacity and may relocate the complete allocation;
the owning local is updated to the new address and all elements are preserved.
A function may return a newly owned list and the caller may bind that result.
List parameters are read-only and therefore cannot create a stale mutable alias
during relocation. Initializing from another list local and whole-list
assignment are rejected.

A `byte` has the value range `0...255`. Byte-list element storage uses one byte
per value. Standalone expression and local evaluation may use machine-sized
registers or stack slots internally, but no value outside that range can be
created through a valid `byte` operation.

Fixed arrays are stack-local values. Whole-array copying and array parameters
or returns are not supported. Function parameters and returns otherwise use
scalar values or immutable/owned handles as described above.

String data, lists, struct records, file buffers, command-line argument copies,
and temporary runtime buffers remain allocated until process exit. Nothing is
reclaimed early. This lifetime policy is defined for Stage 0 but is not a
stable layout or ABI promise.
