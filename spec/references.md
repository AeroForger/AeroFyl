# References

`ref T` is an explicit, non-null reference to one process-lifetime allocation
containing exactly one `T` value. A reference is created with
`reference(expression)`, read through `.value`, and mutated by assigning to
`.value`.

```fyl
ref Node node = reference(Node { value: 1 });
Node copy = node.value;
node.value = Node { value: 2 };
```

Copying, passing, or returning a reference copies its one-word handle and
therefore aliases the same allocation. No pointee clone is implied. References
are never null; `optional ref T` represents a nullable relationship explicitly.
All referenced allocations live until process exit in the bootstrap. There is
no pointer arithmetic, address exposure, manual deallocation, borrow checker,
or implicit dereference.

Direct value-recursive struct cycles are rejected. A recursive edge must pass
through an explicit `ref`, optional reference, list, array, or payload enum so
every record has a finite layout.
