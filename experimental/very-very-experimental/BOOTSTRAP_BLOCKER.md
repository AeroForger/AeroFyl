# Existing bootstrap verifier blocker

The current Rust bootstrap can hang while checking a valid program containing a
loop. This blocks compiling the AeroFyl compiler itself, independently of parsing
or type checking. No language feature change is needed.

A minimal reproducer is:

```fyl
public int loop(int count) {
    int sum = 0;
    int index = 0;
    while (index < count) {
        if (index < 10) { sum = sum + index; }
        index += 1;
    }
    return sum;
}
public void main() { int result = loop(20); }
```

After building the current bootstrap in release mode, repeat its ordinary check
under a one-second timeout. The same source sometimes succeeds immediately
(status 0) and sometimes times out (status 124). An observed prefix was
`0 0 124 0`; this is independent of the new compiler implementation.

Read-only GDB profiling locates the hot loop in
`bootstrap/rust/src/middle/verify.rs::validate_definite_values`, around line 2443.
The definite-value analysis initializes outgoing entry values to the empty set
although the entry block defines values. Other blocks start at the complete value
universe. The first entry update then adds values, breaking the descending
fixed-point invariant. Cyclic control flow can oscillate according to randomized
`HashSet` block traversal order.

The narrowly scoped correction is to initialize every outgoing set to its
incoming set union the values defined by that block. The entry set therefore
contains its definitions before iteration starts, and all subsequent updates can
only remove values. This is an existing verification algorithm correction, not a
language extension or a compiler phase implemented outside AeroFyl.

This document records the blocker before any reference-implementation change.
The new compiler phases remain under `experimental/compiler/` and are `.fyl`.

## Source-only workaround

The existing compiler is preserved. A source-only workaround puts an empty
`while (false) { }` before any local initialization in a loop-containing function.
The resulting entry block immediately jumps to the condition block and defines
no SSA values. The empty loop body never runs. Moving initial state to function
parameters likewise avoids defining values in the entry block.

Both shapes were checked twenty times using the unmodified release bootstrap;
all forty checks succeeded. The original minimal source had thirteen successes
and seven timeouts in twenty checks. New frontend functions use the empty-loop
workaround rather than modifying the reference verifier.
