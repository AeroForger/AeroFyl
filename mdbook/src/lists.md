# Lists

A list declaration states its element type.

```fyl
list int values = [1, 2, 3];
values.push(4);
int last = values.pop();
```

Lists also support `.length` and indexed reads and writes. Functions can return
owned lists; list parameters are read-only. The bootstrap supports scalar,
string, enum, and supported struct elements and preserves them across growth.
`list byte` stores exactly one byte per element and is the binary-data type used
by `std.fs`.
