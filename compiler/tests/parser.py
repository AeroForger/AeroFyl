#!/usr/bin/env python3
"""Build the AeroFyl parser and exercise syntax, ASTs, errors, and termination.

Uses only compiler/, the Rust bootstrap, and non-experimental fixtures. Every
build and parser invocation has a timeout, and artifacts live in a unique temp
folder. The Rust probe compares syntax only, avoiding semantic/codegen checks.
"""
from pathlib import Path
import random
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def run(command, expected=0, timeout=30):
    result = subprocess.run([str(arg) for arg in command], cwd=ROOT, text=True,
                            capture_output=True, timeout=timeout)
    if result.returncode != expected:
        raise AssertionError(f"{command}: expected status {expected}, got {result.returncode}\n"
                             f"{result.stdout}{result.stderr}")
    return result


POSITIVE = [
    "", "// empty\n/* module */", "use std.io;", "using helper: run, Node;",
    "struct Node { optional ref Node next; list int values; }",
    "enum Result { ok(int), error(string), empty, }",
    "enum State { first second }", "struct Empty {} enum Empty {}",
    "public void main() {} private int add(int a, int b) { return a + b; }",
    "public (int, string) f() { return; }",
    "public void main(string[] args) { println(args[0]); }",
    "public void f() { list optional ref Node nodes = []; }",
    "public void f() { int[2][3] grid = [[1, 2], [3, 4], [5, 6]]; grid[0][1] = 9; }",
    "public void f() { int x = -1 + 2 * 3 - 4 / 2; bool b = x < 2 && true || !false; }",
    "public void f() { int n = 0; if (n > 0) { return; } else if (n == 0) { return; } else {} }",
    "public void f() { while (true) { break; continue; } }",
    "public void f() { for (int i = 0; i < 3; i += 1) { continue; } }",
    "public void f() { for (i = 0; i < 3; int next = i + 1) { i = next; } }",
    "public void f() { Node n = Node { value: some(1), next: none() }; n.value = 2; }",
    "public void f() { nodes[0].items.push(3); nodes[0].items[1] += 2; }",
    'public void f() { string s = "abc"; int b = s.byte(0); s.slice(0, 2); }',
    "public void f() { Result r = Result.ok(42); r.is(Result.ok); r.payload(Result.ok); }",
    "public void f() { ref Node n = reference(Node {}); n.value = Node {}; }",
    "public void f() { optional int n = none(); n.hasValue; n.value; }",
    "public void f() { int(a); byte(a); float(a); bool(a); char(a); string(a); input(); }",
    r'''public void f() { char c = '\n'; string s = "a\t\0\\\""; float x = 1.25; }''',
    r'''use std.io; public void f() { println(\v"value={x} \{ok\}\n"); }''',
    r'''public void f() { println(\v""); println(\v"{a}{b}"); println(\v"text"); }''',
    "public void f() { int[18446744073709551615] items = []; }",
    "public void f() { int[00000000000000000000000000001] items = []; }",
    "public void f() { float x = 1.0; true; 'a'; []; Node {}; }",
    "public void f() { int x = 1; x += 1; x -= 1; x *= 2; x /= 2; }",
    "public void f() { x = Node { value: 1, }; }",
    "public void f() { char a = 'é'; char b = '€'; char c = '🙂'; }",
]

# Correctly parsed by this parser; the current Rust parser's type lookahead
# misclassifies named arrays, conversion statements, and parenthesized statements.
IMPROVED = [
    "public void f() { Node[2] nodes = []; }",
    "public void f() { int x = 1; (x + 2); }",
    "public void f() { int(x); }",
]

NEGATIVE = [
    "public", "public void", "public void f", "public void f(",
    "public void f()", "public void f() {", "void f() {}",
    "public struct S {}", "private enum E { one }", "int value = 1;",
    "use ;", "use std.;", "use foo", "using foo;", "using foo:;",
    "using foo: name,;", "using foo: x, x;", "use foo; using foo: x;",
    "struct S { int x }", "struct S { int; }", "struct S {",
    "enum E { one(int, string) }", "enum E { one; }", "enum E { one(int }",
    "public () f() {}", "public (int,) f() {}",
    "public void f(int x,) {}", "public void f(int) {}",
    "public void f() { int x = 1 }", "public void f() { x = ; }",
    "public void f() { int x; }", "public void f() { return }",
    "public void f() { else {} }", "public void f() { if true {} }",
    "public void f() { if (true) return; }", "public void f() { while (true {} }",
    "public void f() { for (; true; i += 1) {} }",
    "public void f() { for (int i = 0; ; i += 1) {} }",
    "public void f() { for (int i = 0; true; ) {} }",
    "public void f() { for (f(); true; i += 1) {} }",
    "public void f() { for (i = 0; true; f()) {} }",
    "public void f() { break }", "public void f() { continue }",
    "public void f() { x = [1,]; }", "public void f() { f(1,); }",
    "public void f() { x = Node { value 1 }; }",
    "public void f() { x = (); }", "public void f() { x = (1, 2); }",
    "public void f() { x = a.; }", "public void f() { x = a[]; }",
    "public void f() { x = [1; }", "public void f() { x = a[1; }",
    "public void f() { int[] values = []; }",
    "public void f() { int[-1] values = []; }",
    "public void f() { int[1.0] values = []; }",
    "public void f() { int[18446744073709551616] values = []; }",
    "public void f() { 1(); }", "public void f() { f()(); }",
    "public void f() { 1 = 2; }", "public void f() { f().value = 2; }",
    "public void f() { 1 + ; }", "public void f() { ++x; }",
    'public void f() { string s = "unterminated; }',
    r'''public void f() { string s = "\q"; }''',
    "public void f() { char c = 'ab'; }", "/* unterminated", "@",
    r'''public void f() { println(\v"{x + 1}"); }''',
    r'''public void f() { println(\v"{}"); }''',
    r'''public void f() { println(\v"{x"); }''',
    r'''public void f() { println(\v"}"); }''',
    r'''public void f() { println(\v"\q"); }''',
    r'''public void f() { println(\v"unterminated); }''',
]

# Reference accepts assignment-shaped expressions and leaves target checking to
# semantic analysis. This parser rejects them according to assignable grammar.
STRICT_TARGETS = {
    "public void f() { 1 = 2; }",
    "public void f() { f().value = 2; }",
}

REFERENCE = r'''
use aerofyl_bootstrap::frontend::{lexer, parser, source::FileId};
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    let accepted = lexer::lex(FileId(0), &text).and_then(parser::parse).is_ok();
    std::process::exit(if accepted { 0 } else { 1 });
}
'''


def main():
    run(["cargo", "build", "--quiet", "--release", "-p", "aerofyl-bootstrap"], timeout=120)
    bootstrap = ROOT / "target/release/aerofyl-bootstrap"
    with tempfile.TemporaryDirectory(prefix="aerofyl-parser-tests-") as directory:
        temporary = Path(directory)
        parser = temporary / "parser"
        ast_tests = temporary / "ast-tests"
        run([bootstrap, "compile", "compiler/frontend/parse_main.fyl", "-o", parser])
        run([bootstrap, "compile", "compiler/frontend/parser_tests.fyl", "-o", ast_tests])
        print(run([ast_tests]).stdout.strip(), flush=True)
        rust = temporary / "reference.rs"
        rust.write_text(REFERENCE)
        reference = temporary / "reference"
        run(["rustc", "--edition=2024", rust, "--extern",
             f"aerofyl_bootstrap={ROOT / 'target/release/libaerofyl_bootstrap.rlib'}",
             "-L", ROOT / "target/release/deps", "-o", reference])
        fixture = temporary / "input.fyl"
        comparisons = 0
        for expected, sources in [(0, POSITIVE + IMPROVED), (1, NEGATIVE)]:
            for text in sources:
                fixture.write_text(text)
                result = run([parser, fixture], expected, timeout=5)
                if expected == 1:
                    assert f"{fixture}:" in result.stderr and ": error: " in result.stderr
                if text not in IMPROVED and text not in STRICT_TARGETS:
                    # Conversion expressions also appear in the wider positive
                    # corpus; the Rust parser has the documented lookahead bug.
                    reference_result = subprocess.run([reference, fixture], timeout=5)
                    if expected == 0 and "int(a);" in text:
                        assert reference_result.returncode == 1
                    else:
                        assert reference_result.returncode == expected, repr(text)
                        comparisons += 1
        print(f"{len(POSITIVE) + len(IMPROVED)} syntax passes, {len(NEGATIVE)} rejections, "
              f"{comparisons} Rust syntax comparisons passed", flush=True)

        # Parse the new parser and every other frontend file as real input.
        frontend_files = sorted((ROOT / "compiler/frontend").glob("*.fyl"))
        for path in frontend_files:
            run([parser, path], timeout=10)
        print(f"{len(frontend_files)} frontend sources parsed", flush=True)

        # Compare existing fixtures at the syntax boundary; most compile-fail
        # files are valid syntax and must be accepted by a standalone parser.
        fixture_files = sorted((ROOT / "examples").glob("*.fyl"))
        fixture_files += sorted((ROOT / "tests").rglob("*.fyl"))
        for path in fixture_files:
            expected = subprocess.run([reference, path], timeout=5).returncode
            assert expected in (0, 1)
            run([parser, path], expected, timeout=5)
        print(f"{len(fixture_files)} repository fixture syntax comparisons passed", flush=True)

        deep_sources = [
            "public void f() { x = " + "(" * 110 + "1" + ")" * 110 + "; }",
            "public void f() { x = " + "!" * 110 + "true; }",
            "public void f() { " + "list " * 110 + "int values = []; }",
            "public void f() { " + "if (true) { " * 110 + "return;" + "}" * 110 + " }",
            "public void f() { " + "while (true) { " * 110 + "return;" + "}" * 110 + " }",
            "public void f() { if (true) {} " + "else if (true) {} " * 110 + "}",
        ]
        for text in deep_sources:
            fixture.write_text(text)
            assert "nesting exceeds parser limit" in run([parser, fixture], 1, timeout=5).stderr
        print(f"{len(deep_sources)} nesting guards passed", flush=True)

        # Truncate declarations at every byte boundary and compare with Rust.
        # These examples avoid intentional grammar fixes above.
        truncation_sources = [POSITIVE[8], POSITIVE[14], POSITIVE[17], POSITIVE[18], POSITIVE[26]]
        truncations = 0
        for text in truncation_sources:
            for end in range(len(text)):
                fixture.write_text(text[:end])
                expected = subprocess.run([reference, fixture], timeout=5).returncode
                run([parser, fixture], expected, timeout=5)
                truncations += 1
        print(f"{truncations} truncated source comparisons passed", flush=True)

        # Long iterative paths must stay bounded without recursive traversal.
        fixture.write_text("public void f() { x" + ".field" * 2000 + " = 1; }")
        run([parser, fixture], timeout=10)
        fixture.write_text("public void f() { return " + "1 + " * 2000 + "1; }")
        run([parser, fixture], timeout=10)
        run([parser], 2)
        run([parser, fixture, fixture], 2)
        print("long postfix/binary chains and CLI statuses passed", flush=True)

        # UTF-8 decoding must reject invalid scalar encodings inside characters.
        invalid_scalars = [b"\x80", b"\xc0\x80", b"\xe0\x80\x80", b"\xed\xa0\x80",
                           b"\xf4\x90\x80\x80", b"\xf0\x80\x80\x80", b"\xe2\x82"]
        for scalar in invalid_scalars:
            fixture.write_bytes(b"public void f() { char c = '" + scalar + b"'; }")
            assert "character literal" in run([parser, fixture], 1).stderr
        print(f"{len(invalid_scalars)} invalid UTF-8 character checks passed", flush=True)

        # Deterministic token-noise smoke: no crashes, hangs, or silent statuses.
        randomizer = random.Random(20261002)
        alphabet = "abc012(){}[];,.=+-*/!&|:\\\"' \n"
        for _ in range(150):
            fixture.write_text("".join(randomizer.choice(alphabet) for _ in range(randomizer.randrange(100))))
            result = subprocess.run([parser, fixture], capture_output=True, timeout=5)
            assert result.returncode in (0, 1), result
        print("150 malformed-input termination checks passed", flush=True)


if __name__ == "__main__":
    main()
