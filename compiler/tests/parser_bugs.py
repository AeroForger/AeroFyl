#!/usr/bin/env python3
"""Run the parser.fyl bug regression checks.

The probes live in compiler/frontend/parser_bug_tests.fyl. Each one asserts the
behavior described by the grammar, the spec, or compiler/frontend/README.md, so
the runner stays red until the parser bugs are fixed.
"""
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def run(command, timeout=120):
    return subprocess.run([str(argument) for argument in command], cwd=ROOT,
                          text=True, capture_output=True, timeout=timeout)


def main():
    build = run(["cargo", "build", "--quiet", "--release", "-p", "aerofyl-bootstrap"])
    if build.returncode != 0:
        sys.stderr.write(build.stdout)
        sys.stderr.write(build.stderr)
        return build.returncode
    bootstrap = ROOT / "target/release/aerofyl-bootstrap"
    with tempfile.TemporaryDirectory(prefix="aerofyl-parser-bugs-") as directory:
        tests = Path(directory) / "parser-bug-tests"
        built = run([bootstrap, "compile", "compiler/frontend/parser_bug_tests.fyl",
                     "-o", tests])
        if built.returncode != 0:
            sys.stderr.write(built.stdout)
            sys.stderr.write(built.stderr)
            return built.returncode
        result = subprocess.run([str(tests)], cwd=ROOT, text=True,
                                capture_output=True, timeout=30)
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        if result.returncode != 0:
            print(f"parser bug regression checks failed with status "
                  f"{result.returncode}", file=sys.stderr)
            return 1
    print("parser bug regression checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
