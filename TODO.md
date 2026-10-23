# Plan: spec remake through self-hosting compiler

Locked in 2026-10-08. Order is sequential unless marked parallel.

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

## Parallel track: bootstrap compile hang

Investigate to root cause with evidence. Outcome is a report only; no changes to the frozen Rust bootstrap. Findings feed the avoidance list for the new backend.
