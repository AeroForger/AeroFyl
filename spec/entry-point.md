# Entry point

The known entry-point form is:

```fyl
public void main()
{
}
```

The bootstrap also accepts:

```fyl
public void main(string[] args)
{
}
```

An executable has exactly one of these forms in the bootstrap. `string[]` is a bootstrap-only read-only command-line argument collection, valid only as the sole `main` parameter. It excludes the native executable path from its elements.

The permanent command-line argument type, process exit model, and entry-point rules beyond these forms are not specified yet.
