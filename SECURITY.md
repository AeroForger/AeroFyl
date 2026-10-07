# Security Policy

## Supported versions

AeroFyl is pre-alpha. Only the current `main` branch is supported. There
are no stable releases and no backported fixes.

## Reporting a vulnerability

**Do not open a public issue for security problems.** This includes
memory-safety bugs in the bootstrap backend, codegen flaws that could
produce unsafe executables, and standard-library issues that bypass
documented checks.

Report privately to the maintainer (AeroForger) with:

- Affected component (`bootstrap/rust/...`, `compiler/frontend/...`, …)
- Concrete reproducer: source file plus exact commands
- Observed vs expected behavior, including exit statuses
- Whether the issue affects compiled programs at runtime or only the
  compiler itself

You should receive an initial response within 7 days. If the report is
accepted, a fix is developed privately and credited on release unless you
prefer anonymity.

## Scope notes

- The executable backend targets Linux x86-64 only; other platforms are
  out of scope until supported.
- `experimental/` is exploration space and out of scope for security
  response, though severe issues there are still welcome as private
  reports.
- Social engineering, spam, and theoretical issues without a reproducer
  are not handled under this policy.
