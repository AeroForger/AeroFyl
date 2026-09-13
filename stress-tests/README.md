# Stress fixtures

These fixtures exercise known compiler behavior at bounded sizes. They are checked by `tests/run_tests.sh`. Runtime-heavy cases remain in Rust tests where exit status can be asserted.

The `lexer/`, `parser/`, and `backend/` directories are retained for focused fixtures. The added directories cover control flow, functions, structs, lists, and strings.
