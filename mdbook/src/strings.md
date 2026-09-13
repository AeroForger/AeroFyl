# Strings

The bootstrap supports string concatenation, content equality, `.length`, `.byte(index)`, and `.slice(start, end)`.

```fyl
string name = "Aero" + "fyl";
string prefix = name.slice(0, 4);
int first = name.byte(0);
```

These operations use UTF-8 bytes. Escape syntax and higher-level Unicode operations are not specified yet.
