# AeroFyl compiler

The frontend is written in AeroFyl and built with the Rust bootstrap. It loads
and lexes a source file, then parses declarations and function bodies into an
AST with byte spans. It does not yet resolve imports, check types, or generate
an executable from that input. A successful frontend run means parsing succeeded.

Build and run from the repository root:

```sh
cargo run --quiet -p aerofyl-bootstrap -- compile compiler/frontend/parse_main.fyl -o /tmp/aerofyl-parser
/tmp/aerofyl-parser examples/hello.fyl
```

Run the parser tests:

```sh
python3 compiler/tests/parser.py
```

The tests build native parser and AST-test executables in a unique temporary
folder. They check AST structure, source spans, syntax failures, nesting limits,
truncated inputs, long expression chains, and existing non-experimental fixtures.
They compare syntax with `bootstrap/rust/src/frontend/parser.rs` separately from
semantic analysis. No experimental modules are loaded or copied.

See [frontend/README.md](frontend/README.md) for the parser API and AST layout.
The lexer supports ASCII identifiers, Unicode character literals, byte-oriented
strings, and interpolated literals. Unicode identifiers remain a lexer limitation.
