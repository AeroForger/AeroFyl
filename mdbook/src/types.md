# Types

Known basic types are `int`, `float`, `bool`, `char`, `string`, `void`, and `dynamic`. Named structs and enums are also types. Fixed arrays include their length, such as `int[4]`, while lists use forms such as `list int`.

The executable bootstrap currently handles a narrower set than the parser. In particular, floating-point executable lowering and defined `dynamic` behavior are absent. The bootstrap generally requires exact type matches because conversion rules are not specified.
