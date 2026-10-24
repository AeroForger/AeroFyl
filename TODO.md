Locked in 2026-10-08. Order is sequential unless marked parallel.

## Phase 1: Spec remake is done

## Phase 2: Add FutureIdeas/ with its own docs

- New top-level `FutureIdeas/` directory with its own README describing what belongs there.
- Idea lifecycle: sketch, open questions, accepted or rejected with reasons.
- Seed it with current ideas so nothing lives only in chat history.
- Ideas stay out of the spec until they graduate through the spec-change process.

## Phase 3: Investigate \v

Runs after the spec remake, before self-hosting work:

- Confirm the deprecation warning fires end to end in the bootstrap and is covered by tests.
- Record the decision in the new spec: \f is the syntax, \v is deprecated, with a removal version.
- The self-hosted lexer implements \f correctly from the start and mirrors the warning.

## Phase 4: Self-hosting compiler

Pipeline order: resolution, semantic analysis, IR, backend.

Backend requirements from day one, so the Rust mistakes are not repeated:

- Keep values in registers instead of a stack slot per temporary.
- Strength-reduce division by constants instead of raw idiv.
- Inline small functions.
- Hoist loop invariants.
- Keep compilation deterministic.

Performance bar: generated code must at least match the bundle 2 medians, preferably beat them. Rerun the benchmark suite against the self-hosted compiler on the same machine for a fair read.

# Audits

## Audit findings: 2026-10-24

Findings from the bootstrap and self-hosting audit. Bootstrap changes go through
change control; self-hosting issues belong in the rewrite.

### Fix in the bootstrap (change control)

- Parser has no nesting depth guard, so deeply nested source stack overflows
  (release build aborts around 2900 nested parens, debug around 400) instead of
  printing a diagnostic. The self-host caps at 96. Add a depth counter to the
  parser and the semantic passes, matching the self-host limit.
- `starts_variable_declaration` uses naive one-token lookahead and rejects
  `Node[2] nodes = [];`, `(1 + 2);`, and `int(a);` as statements. Port the
  self-host `pVariableStart` lookahead.
- Statement-level error recovery is missing: the first bad statement in a
  function hides the rest of it. Port `pSynchronizeStatement` into
  `parse_block`.
- Interpolation placeholder errors: the lexer stops without scanning to the
  closing quote, so one bad `\f"..."` produces a second spurious
  "unterminated string" error. Scan to the closing quote on the error path.
- Remove or annotate the dead `PE_REGCOPY` constant in
  `backend/x86_64/runtime.rs` (dead-code build warning).

### Fix in the self-hosting plan

- Trap in the new parser: `pAdd` returns -1 on failure and callers only guard
  via the `failed` flag before indexing. Make that invariant self-checking so
  one missed guard cannot turn into an out-of-range failure inside the
  compiler.
- Self-host lexer builds string literals by repeated concatenation, O(n^2) per
  literal. Needs a builder or two-pass copy once std grows one.

### Avoid in the new backend (from the bootstrap)

- `backend/x86_64/lower.rs` has near-duplicate list code paths (`ListInit`
  inline vs `emit_list_value`, `ListPop` vs `ListPopValue`). One path only.
- The verifier widens to three full traversals and clones per-block value sets
  per fixpoint iteration; pathological input is quadratic in blocks. Keep the
  rewrite single-pass where possible.
- Add fixtures: deep nesting should produce a diagnostic, not a crash; the
  `Node[2] x = [];` statement form once the lookahead is fixed.
