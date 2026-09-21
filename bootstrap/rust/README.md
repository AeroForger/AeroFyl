# Aerofyl Rust bootstrap compiler

This crate is a temporary, dependency-free compiler skeleton. It deliberately
implements only the language facts currently recorded in the repository task.

## Implemented pipeline

The driver loads a root `.fyl` file and its recursive `use name;` dependencies,
then runs source mapping, lexing, handwritten
parsing, name resolution, exact-match semantic checks, HIR construction, and
lowering to a small backend-independent IR. The middle-end verifies that IR
before `CompileOptions::check()` returns or x86-64 lowering begins.

The x86-64 package lowers the milestone integer subset using the System V AMD64
ABI, encodes machine instructions directly, generates Linux startup code, and
wraps it in a complete ELF64 executable. No assembler, linker, libc, or external
compiler is used.

## IR verification

`middle::verify::verify_module` runs immediately after HIR-to-IR lowering. The
normal compilation path is therefore:

```text
source -> AST -> HIR -> IR -> verify IR -> VerifiedIrModule -> x86-64 -> ELF
```

Verification checks that function, block, value, and local identifiers are
valid; entry blocks and branch targets exist; every block has a terminator; and
all blocks are reachable. Unreachable blocks are hard verification failures in
this milestone. A terminator is stored separately from a block's instruction
list, so the representation cannot express instructions after a terminator.

The verifier also checks that values are defined on every incoming control-flow
path before use, locals exist and have consistent types, operations have the
required operand and result types, branch conditions are Boolean, calls resolve
to matching function signatures, and returns match their function type.
Short-circuit expressions use one logical result defined in path-disjoint
predecessors of the same merge block; other multiple definitions are rejected.

Successful verification returns a borrowing `VerifiedIrModule` capability whose
constructor and raw-module accessor are private to the compiler. The x86-64
lowering API accepts only this verified wrapper, so raw IR cannot cross the
backend boundary. Verification returns all reasonably independent problems it
finds as structured `IrVerificationError` values. Driver diagnostics label these failures as
`internal compiler error: invalid IR`: they indicate a compiler bug after valid
source has passed semantic analysis, and are distinct from source diagnostics
that include source locations. A failed verification stops compilation before
the backend is called.

## Supported source forms

- `public` and `private` function declarations
- relative file imports with `use module;` and built-in `use std.io;`
- the documented basic types
- type/name parameters
- fixed arrays such as `int[4]`
- lists such as `list int`
- tuple return types containing types only
- initialized local declarations
- `return` statements and semicolon-terminated call expressions
- identifier, decimal integer, decimal float, string, character, call, indexed,
  member, method-call, and bracketed collection expressions
- signed 64-bit integer literals, unary negation, and precedence-aware `+`, `-`,
  `*`, and `/`
- explicit `int(value)`, `char(value)`, and checked `byte(value)` conversions
  for supported scalar and enum values
- `true` and `false`, unary `!`, short-circuit `&&` and `||`
- integer `==`, `!=`, `<`, `<=`, `>`, and `>=`; Boolean and character `==` and `!=`
- `if`/`else`, `while`, `break`, `continue`, and exact-type variable assignment
- fixed-layout structs with complete named-field construction, field access,
  field assignment, and exact-type value copying
- payload-free enums, qualified variants, and enum `==`/`!=`
- fixed-array and list literals, indexed loads/stores, and `.length`
- list `.push(value)` and `.pop()`, including lists of strings and supported structs
- UTF-8 string `.length`, content `==`/`!=`, and concatenation with `+`
- UTF-8 byte access with `string.byte(int)`, byte-range copying with
  `string.slice(start, end)`
- explicit `std.io` output for strings, characters, integers, and Booleans,
  plus line input parsed as string, integer, Boolean, or character
- explicit `std.fs` text and binary whole-file operations plus path existence
- executable entry points `public void main()` and `public void main(string[] args)`
- intentional process termination with `exit(int)`

## Bootstrap modules

`use lexer;` loads `lexer.fyl` from the importing file's directory. `use
std.io;` and `use std.fs;` are recognized as built-in standard-library modules
and do not read a package or relative file; their symbols remain unavailable
until the corresponding import is present. Imports are
recursive, canonical file identity prevents duplicate loading through diamond
dependencies, and cycles are rejected. All loaded declarations share one
program-wide namespace. Public functions are visible across files; private
functions are restricted to references originating in their declaring file.
Structs and enums have no visibility syntax and are shared throughout the
loaded graph. Duplicate direct imports, missing modules, cycles, and top-level
symbol collisions are diagnosed. Qualified user paths, aliases, selective
imports, separate compilation, packages, and `using` are not implemented.

## Bootstrap structs and enums

Struct declarations and values use the following forms:

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

point.x = point.x + 1;
```

Every field must appear exactly once in a literal and field types match exactly.
The bootstrap layout preserves declaration order, aligns the struct to eight
bytes, and gives each supported scalar, enum, string, collection, or optional field
one eight-byte slot. Field zero has byte offset zero, field one offset eight,
and so on. Locals hold pointers to process-lifetime heap records using this one
reusable layout. This is an internal bootstrap representation, not a permanent
Aerofyl ABI.

Whole-struct initialization and assignment require the exact same struct type
and copy every field slot into a fresh record. Mutating the destination therefore
does not mutate the source. A copied string field shares its pointer to immutable
string storage; the bytes do not need to be duplicated for each struct copy.
List, array, and optional fields occupy handle slots. Their underlying storage
is shared when a struct record is copied; no collection clone is implied.
Supported structs may be passed and returned through explicit record copies.
Direct nested structs, float, dynamic, and other unsupported fields remain
rejected. There are no user-visible references,
aliases, field visibility, user-defined methods, inheritance, interfaces,
generics, or default/omitted fields.

Enums are payload-free and use qualified values:

```fyl
enum State
{
    idle,
    running,
    stopped
}

State state = State.running;
bool active = state != State.idle;
```

The x86-64 bootstrap represents each enum as one eight-byte integer slot.
Variants receive zero-based discriminants in declaration order, which are
observable through `int(value)`. The memory layout is not a stable external ABI. Enum payloads,
explicit discriminants, unqualified variants, methods, tagged unions, and enum
ordering comparisons are unsupported. Enums can be stored in struct fields.

Executable lowering currently supports integer, byte, Boolean, character, enum,
string, fixed-array, list, optional, and supported struct values, integer
arithmetic, and calls. Fixed-array parameters and returns are not supported. An executable must have exactly one
root-module `public void main()` or `public void main(string[] args)`. Linux startup calls it
and then exits with status zero through the x86-64 `exit` syscall.
`exit(code)` invokes that syscall immediately with the supplied integer; Linux
exposes its low eight bits as the process status. Runtime failures use status
70.

For this milestone, `int` is a signed 64-bit value and `byte` is an unsigned
value restricted to `0...255`. The reserved primitive
representations are IEEE-754 binary64 for `float` and a `u32` Unicode scalar
value for `char`; floating-point executable lowering is intentionally not
present. Boolean values are canonical `0` (false) or `1` (true) in registers and
occupy one eight-byte bootstrap stack slot, matching the simple IR value layout.

`int(char)` returns the Unicode scalar value, `int(byteValue)` returns the
unsigned byte value, `int(enumValue)` returns the
zero-based declaration-order discriminant, and `char(int)` validates the
Unicode scalar range at runtime. Invalid scalar values fail with status 70.
`byte(int)` checks the range and fails with status 70 rather than truncating.
Integer-to-enum and unrelated conversions are rejected.

## Bootstrap arrays, lists, and strings

Fixed arrays are heap-backed contiguous eight-byte slots behind an internal
one-word handle. Their length is part
of the type and must exactly match the literal. Index expressions must have type
`int`; both indexed reads and writes perform signed runtime bounds checks, so a
negative index or an index greater than or equal to the length terminates the
process with bootstrap failure status 70. `.length` is the compile-time array
length. This handle representation permits nested arrays and array-valued
struct fields. Whole-array copies and fixed-array function ABI values are unsupported.

Lists are heap-backed and represented internally by a pointer to a header
containing eight-byte length, capacity, and element stride fields, followed by
contiguous element storage. Byte elements have stride one; other scalar and
enum strides are eight bytes; a struct's stride is its complete fixed layout
size. A literal starts with capacity of at
least four. `.push(value)` grows full storage by doubling it through Linux
`mremap`, which preserves the header and every byte of every element. Aggregate
push, pop, indexed load, and indexed store copy all field slots. In particular,
mutating a struct loaded from a list does not mutate the stored element; an
explicit indexed assignment is required to write it back. Indexed access uses
the current length; out-of-bounds access and popping an empty list terminate
with status 70. Storage is valid for the life of the process and is
intentionally not reclaimed. There is no whole-list cloning operation.
Functions may return a newly owned list, and callers may bind that result.
List parameters are read-only: they support length and indexed loads but not
indexed assignment, push, or pop. This avoids stale aliases when growth uses
`mremap` and relocates storage.

Strings are immutable UTF-8 byte sequences represented by a heap pointer to an
eight-byte byte length followed by the bytes. `.length` therefore reports bytes,
not Unicode scalar values or grapheme clusters. Equality and inequality compare
contents rather than pointers. `+` allocates a new concatenated string; operands
are unchanged. String indexing is rejected. Deduplicated string literals live
in an eight-byte-aligned, read-only ELF load segment, separate from executable
code; code reaches them with RIP-relative addresses.

`string.slice(start, end)` allocates an independent length-prefixed string and
copies the selected bytes. `start` is inclusive, `end` is exclusive, and both
are `int` UTF-8 byte offsets satisfying
`0 <= start <= end <= source.length`. Invalid bounds terminate with status 70;
indexes are never clamped. Empty and full slices are valid, and equality with an
empty literal works. Boundaries need not align to Unicode scalar values, so a
slice may contain arbitrary bytes rather than valid UTF-8. This operation is
byte-oriented intentionally for compiler source processing; it does not provide
character or grapheme slicing.

Arrays and lists accept supported nested collections, strings, optionals,
payload-free enums, scalars, and structs whose fields satisfy the bootstrap
layout restrictions above. Collection literals require an expected
array/list type; standalone inference remains intentionally unspecified. The
dependency-free runtime obtains storage directly with Linux `mmap`; it uses no libc, assembler,
linker, or external compiler.

## Bootstrap source and filesystem access

`string.byte(index)` returns the unsigned integer value `0..255` at a UTF-8 byte
offset. The index must be `int`. Negative indexes and indexes greater than or
equal to `.length` terminate the process with status 70. This does not enable
`text[index]`: ordinary string indexing and Unicode character/grapheme indexing
remain unsupported. For example, `"é"` has length 2 and byte values 195 and 169.

After `use std.fs;`, `readFile(path)` accepts exactly one string and reads the complete regular file
as raw bytes into the existing length-prefixed string representation. The Linux
runtime creates a temporary NUL-terminated path, calls `openat`, obtains the
length with `lseek`, reads until that length or EOF, and calls `close`. File data
is allocated with the shared `mmap` allocator and lives until process exit.
Open, seek, read, close, and allocation failures report `FileReadError` and
terminate with status 70.

`writeFile(path, data)` creates or truncates a regular file and writes all data
bytes before closing it. Creation requests mode `0666` subject to the process
umask. `readBytes` and `writeBytes` use `list byte`, preserving every byte with
one-byte element storage and no file metadata. `exists` uses `newfstatat`, is
true for directories and other existing objects, and is false only for a
normally missing path. Filesystem failures report a small diagnostic category
and terminate with status 70. Interrupted data reads/writes and partial writes
are retried. Append, random access, deletion, directory iteration, and file
objects are unsupported.

Runtime helpers are selected by call-graph reachability. Merely importing a
standard-library module emits no implementation; each used operation brings in
only its helper and transitive dependencies.

The `string[]` spelling is an entry-point-only, read-only command-line argument
type. It is valid solely as the one parameter of `public void main`; ordinary
string arrays, local `string[]` declarations, mutation, and whole-value copies
remain unsupported. Startup reads Linux `argc`/`argv`, excludes native
`argv[0]`, copies each user argument into an Aerofyl length-prefixed string, and
materializes a generalized list-compatible
`{length, capacity, stride, string pointers...}` block.
Consequently `args.length` is the number of user arguments and `args[0]` is
native `argv[1]`. These process-lifetime allocations are a temporary bootstrap
ABI, not Aerofyl's final command-line or collection representation.

## Bootstrap memory boundary

Scalars and payload-free enums copy by value. Strings are immutable handles and
may share read-only byte storage. Supported structs copy every field slot when
assigned, passed, or returned; string and collection handle fields share their
underlying storage. Standalone list locals have one owner and may be returned
to transfer that owned handle, and are read-only through function parameters.
List growth may relocate storage and updates the owning local. Fixed arrays are
heap-backed handles and cannot be copied or passed as function ABI values.
Optionals use tagged two-slot heap records. All heap and file-buffer allocations
live until process exit; the bootstrap performs no reclamation.

The deliberately simple backend gives every parameter, local, and IR temporary
an eight-byte stack slot. It uses fixed caller-saved registers for operations,
passes the first six integer arguments in the SysV registers, passes remaining
arguments on the stack, and preserves 16-byte call-site stack alignment.

Integer addition, subtraction, multiplication, and negation wrap modulo `2^64`
and are interpreted as signed two's-complement results. Signed division
truncates toward zero. The backend explicitly checks division by zero and
`INT64_MIN / -1`; both terminate with status 70 instead of relying on a machine
exception.

Conditions require `bool`; integers are not converted implicitly. Logical `&&`
and `||` lower into CFG branches, so their right operand is evaluated only when
required. The backend-independent IR uses basic blocks with conditional branch,
unconditional jump, and return terminators. The encoder resolves calls and both
forward and backward branches with signed rel32 fixups.

Integer return analysis recognizes direct returns and `if`/`else` where both
branches return. It is deliberately conservative for loops because Aerofyl has
not specified unreachable-code or infinite-loop rules. Chained `else if` and
`+=`, `-=`, `*=`, and `/=` are supported. There is no `for`, `foreach`, or
`switch` in this milestone.

Strings support `\n`, `\r`, `\t`, `\0`, `\\`, and `\"` while preserving
byte-oriented storage. Character literals contain one Unicode scalar value or one of
`\n`, `\r`, `\t`, `\0`, `\\`, and `\'`. A character literal used in an `int`
context contributes its Unicode scalar value, allowing byte-oriented code such
as `source.byte(i) == '0'` while keeping `char` distinct from `int` elsewhere.

## Specification still required

The following decisions are intentionally not made by the bootstrap:

- the exact identifier alphabet (the lexer currently uses a minimal Unicode
  alphabetic/underscore convention, marked with a source TODO)
- alternate integer spellings
- floating-point code generation and runtime representation
- additional string and character escapes
- module paths, aliases, qualification, packages, and `using`
- tuple value syntax
- standalone collection inference and the eventual stable collection ABI
- additional conversions, coercions, promotion, `dynamic` behavior, and other
  type-system relationships
- richer control flow such as `for`/`foreach` and conditionals beyond the forms
  documented above
- project manifest file name and TOML key schema

Unsupported constructs retain their frontend/IR interfaces but executable
lowering returns structured diagnostics or backend errors. Assignment and the
four arithmetic compound assignments target existing assignable values.

Check a source file without emitting an executable:

```text
cargo run -p aerofyl-bootstrap -- <source.fyl>
```

The bootstrap-only executable command is:

```text
cargo run -p aerofyl-bootstrap -- compile <source.fyl> -o <executable>
```
