# Lists

A list declaration states its element type.

```fyl
list int values = [1, 2, 3];
values.push(4);
int last = values.pop();
```

Lists also support `.length` and indexed reads and writes. The current element-type and copy restrictions are described in the bootstrap README.
