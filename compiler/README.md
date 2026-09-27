# AeroFyl compiler

This is the start of the compiler written in AeroFyl, compiled by the Rust
bootstrap. The implemented path loads a source file and lexes it, returning a
source offset and error for invalid lexical input. It does not yet parse, check,
or generate an executable from the input. A successful run means **lexing only**.

Build and run from the repository root:

```sh
cargo run --quiet -p aerofyl-bootstrap -- compile compiler/frontend/main.fyl -o /tmp/aerofyl-alpha-lexer
/tmp/aerofyl-alpha-lexer examples/hello.fyl
```

`frontend/token.fyl` defines token kinds, `source.fyl` loads input and defines
byte spans, `diagnostics.fyl` carries errors, and `lexer.fyl` tokenizes input.
The implementation follows `spec/lexical.md` and
`bootstrap/rust/src/frontend/{token,lexer}.rs`. Current gaps include Unicode
identifiers and characters, interpolated output literals, and later compiler
phases. It must not be used as a substitute for the Rust bootstrap yet.
