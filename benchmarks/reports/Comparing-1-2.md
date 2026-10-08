# Comparing Pre-Alpha bundle 1 and Pre-Alpha-2 (bundle 2)

Bundle 1: `benchmarks/Pre-Alpha-Benchmark-Bundle.zip` (folder `Pre-Alpha-Benchmark-Bundle/` inside), AeroFyl commit `10d75e7`, measured 2026-10-04.
Bundle 2: `benchmarks/Pre-Alpha-2-Benchmark-Bundle.zip` (folder `Pre-Alpha-2-Benchmark-Bundle/` inside), AeroFyl commit `9dda7a4` (final Rust bootstrap revision), measured 2026-10-08.

## Read this first: what this comparison can and cannot show

The two bundles ran on different machines with different toolchains, so absolute times are not comparable and ratio shifts cannot be assigned to the compiler change alone. The confounds are:

- Hardware and OS: bundle 1 ran on an Intel Xeon Platinum 8370C virtual machine (Linux 6.18, glibc 2.39); bundle 2 ran on an Intel Core 5 210H (CachyOS Linux 7.2, glibc 2.44).
- C toolchain: GCC 13.3.0 in bundle 1, GCC 16.2.1 in bundle 2. The newer GCC produces visibly faster code on some kernels (for example predictable branches at O3: 3.564 ms in bundle 1 vs 1.599 ms in bundle 2).
- Python: 3.12.14 in bundle 1, 3.14.7 in bundle 2. Go is 1.27.1 in both.
- Sydrogen: same pinned revision in both bundles, but bundle 1 built it fresh from a clone while bundle 2 used the local Alpha-7 binary.
- The AeroFyl revision itself differs, which is the variable of interest, but it sits on top of all the above.

The valid use of the tables below is to compare the pattern of within-bundle ratios, not to declare one compiler revision faster than the other.

## Geometric means of per-workload ratios (six kernels)

Higher means AeroFyl relatively slower, except Python where higher means AeroFyl relatively faster. Not a ratio of totals; startup control excluded.

| Comparison | Bundle 1 | Bundle 2 |
| --- | --- | --- |
| AeroFyl vs C-O3 | 5.01x slower | 2.68x slower |
| AeroFyl vs Python | 6.93x faster | 13.29x faster |
| AeroFyl vs Sydrogen Cranelift stdin | 3.05x slower | 1.49x slower |
| AeroFyl vs Go | 3.58x slower | 1.84x slower |
| AeroFyl vs Sydrogen direct native, fixed input | 1.92x slower | 1.14x slower |

## Per-workload ratios: AeroFyl time / baseline time

Values above 1 mean AeroFyl took longer in that bundle.

### Versus C-O3

| Workload | Bundle 1 | Bundle 2 |
| --- | --- | --- |
| Bounded arithmetic | 3.85x | 1.83x |
| Predictable branches | 9.11x | 10.88x |
| State-dependent branches | 4.24x | 1.89x |
| Function calls | 5.44x | 1.84x |
| Recursive Fibonacci | 6.75x | 3.49x |
| Prime counting | 2.88x | 1.54x |

### Versus Go

| Workload | Bundle 1 | Bundle 2 |
| --- | --- | --- |
| Bounded arithmetic | 3.27x | 1.42x |
| Predictable branches | 5.41x | 4.92x |
| State-dependent branches | 3.45x | 1.53x |
| Function calls | 4.47x | 1.59x |
| Recursive Fibonacci | 2.70x | 1.49x |
| Prime counting | 2.85x | 1.51x |

### Versus Sydrogen Cranelift, stdin phase

| Workload | Bundle 1 | Bundle 2 |
| --- | --- | --- |
| Bounded arithmetic | 2.74x | 1.22x |
| Predictable branches | 2.78x | 1.83x |
| State-dependent branches | 2.84x | 1.30x |
| Function calls | 3.49x | 1.23x |
| Recursive Fibonacci | 2.65x | 1.44x |
| Prime counting | 4.00x | 2.14x |

### Versus Sydrogen direct native, fixed-input phase

| Workload | Bundle 1 | Bundle 2 |
| --- | --- | --- |
| Bounded arithmetic | 1.81x | 1.08x |
| Predictable branches | 1.55x | 1.27x |
| State-dependent branches | 1.96x | 1.04x |
| Function calls | 2.11x | 1.12x |
| Recursive Fibonacci | 1.78x | 0.96x |
| Prime counting | 2.41x | 1.43x |

In bundle 2, fixed-input recursive Fibonacci is the single kernel where AeroFyl measured faster than direct-native Sydrogen (10.19 ms vs 10.60 ms). All other kernels still favor the baselines.

### Versus Python (AeroFyl speedup: Python time / AeroFyl time)

| Workload | Bundle 1 | Bundle 2 |
| --- | --- | --- |
| Bounded arithmetic | 5.40x | 12.52x |
| Predictable branches | 11.13x | 16.73x |
| State-dependent branches | 6.45x | 13.78x |
| Function calls | 5.32x | 14.12x |
| Recursive Fibonacci | 6.84x | 10.43x |
| Prime counting | 7.82x | 12.94x |

AeroFyl was faster than CPython in every kernel in both bundles. The larger bundle 2 speedups come mostly from lower AeroFyl absolutes on the newer machine, not from slower Python.

## Absolute medians for reference (milliseconds, not cross-comparable)

Bundle 1 on the left of each pair, bundle 2 on the right.

| Workload | AeroFyl | C-O3 | Python | Go | Syd CL |
| --- | --- | --- | --- | --- | --- |
| Bounded arithmetic | 97.962 / 31.242 | 25.464 / 17.088 | 529.007 / 391.943 | 29.944 / 22.020 | 35.728 / 25.642 |
| Predictable branches | 32.485 / 17.395 | 3.564 / 1.599 | 361.607 / 291.077 | 6.006 / 3.536 | 11.696 / 9.531 |
| State-dependent branches | 105.457 / 36.356 | 24.874 / 19.235 | 680.695 / 500.921 | 30.566 / 23.742 | 37.194 / 27.919 |
| Function calls | 80.137 / 21.169 | 14.719 / 11.480 | 426.659 / 298.932 | 17.922 / 13.348 | 22.943 / 17.184 |
| Recursive Fibonacci | 29.543 / 11.019 | 4.374 / 3.159 | 202.213 / 114.907 | 10.934 / 7.379 | 11.165 / 7.668 |
| Prime counting | 150.093 / 60.442 | 52.048 / 39.186 | 1174.068 / 782.049 | 52.679 / 40.030 | 37.539 / 28.192 |
| Startup control | 0.270 / 1.298 | 2.084 / 1.538 | 15.871 / 10.498 | 2.078 / 1.997 | 2.120 / 1.588 |

Every configuration measures faster in bundle 2 in absolute terms, which is a machine effect, not a compiler result. Note the startup control: AeroFyl stayed the fastest of the compiled configs in both bundles, but the margin shrank from about 2 ms vs 0.27 ms to about 1.5 ms vs 1.3 ms.

## Other differences between the runs

- Build reliability: bundle 1 logged a 120 second timeout on one fixed-input rebuild attempt with the older compiler. Bundle 2 hit the same family of behavior twice (once on `state_branches` in a discarded first attempt, once on fixed-input `predictable_branches`) and records a same-command retry rule with per-attempt logs. See the build notes in the bundle 2 report.
- Bundle 2 drops the restated first-run spike paragraph because there was no separate noisy first run this time; the single retry above is recorded in its retry table instead.
- Both bundles agree on the qualitative picture: AeroFyl ahead of CPython everywhere, behind optimizing C, Go, and Sydrogen outputs on these scalar integer kernels, with the gap narrowest against direct-native Sydrogen on fixed inputs.

## How the numbers above were produced

Ratio tables were computed from the `median_ms` values in each bundle's `results.json` and `native-control.json`, using the same geometric-mean definition as the reports. No samples were pooled across bundles.
