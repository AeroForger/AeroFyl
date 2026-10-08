# AeroFyl Pre-Alpha-2 benchmark (final Rust bootstrap measurement): C, Python, Sydrogen and Go

This is the final benchmark for the pre-alpha line. Rust bootstrap development has stopped; the AeroFyl revision measured here is the closing state of that compiler.

Measured run started: **2026-10-08T14:52:30Z**. All timings are from the execution environment described below, **not Davit's laptop**. This is a scalar integer microbenchmark suite at pinned compiler revisions; its results do not establish a universal language-speed ranking.

## Observed results

All main-suite outputs passed 336 validation executions (seven programs x six input fixtures x eight configurations), followed by output checks on every warmup and timed execution. There were no failed comparisons.

Across the **six selected workloads**, the geometric mean of per-workload ratios was:

| Requested comparison | Measured geometric mean |
| --- | --- |
| Test 1 - C | AeroFyl 2.68x slower |
| Test 2 - Python | AeroFyl 13.29x faster |
| Test 3 - Sydrogen (Cranelift, stdin) | AeroFyl 1.49x slower |
| Test 4 - Go | AeroFyl 1.84x slower |

A geometric mean gives each of these six workloads equal weight. It excludes the startup control and is not the ratio of total elapsed times. The per-workload numbers below are the primary result.

**Backend discovery:** at this Sydrogen revision, `Expr::Input(_)` and `Statement::Input(_)` cause native requests to fall back to the typed Cranelift path. Consequently, both stdin Sydrogen configurations in the main run used Cranelift. The separate fixed-input comparison below verifies actual direct-native output. The two main-run CLI configurations must not be presented as independent backend implementations.

## Environment and pinned sources

| Item | Observed configuration |
| --- | --- |
| CPU | Intel(R) Core(TM) 5 210H |
| Execution environment | Linux x86-64 (see OS and Affinity rows) |
| OS | Linux-7.2.9-1-cachyos-x86_64-with-glibc2.44 |
| Affinity | Runner and child processes pinned to logical CPU 0; original allowed set [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] |
| Resource limits | cpu.max=unavailable; memory.max=unavailable bytes |
| C | gcc (GCC) 16.2.1 20260810 |
| Python | 3.14.7 (main, Aug 14 2026, 06:38:32) [GCC 16.2.1 20260810] |
| Go | go version go1.27.1-X:nodwarf5 linux/amd64 |
| Rust compiler used to build both compilers | rustc 1.99.0 (b940084d7 2026-09-28) (Arch Linux rust 1:1.99.0-1) |
| AeroFyl | Stage 0 Rust bootstrap, final pre-alpha revision; commit `9dda7a4cecf60e9dafef87891589ccd54cf9b36f` |
| Sydrogen | Furnace Alpha-7; commit `8e8e31a448c1f98fa5e879f12a2bd916c4bb4bc6` |

No compiler-source patches were applied. The AeroFyl checkout was the pinned commit above with no tracked-file modifications (only untracked benchmark output, see `status` in `environment.json`). Both compiler executables were built using `cargo build --release --locked`. A release build optimizes the **compiler executable**; it does not add an optimization flag to the generated AeroFyl or Sydrogen program. The Sydrogen side reuses the same pinned revision as the first Pre-Alpha bundle; the local furnace binary reporting the same Alpha-7 line was used without a fresh clone, as recorded in `environment.json`.

Sources:

- [Pinned AeroFyl repository](https://github.com/AeroForger/AeroFyl/tree/9dda7a4cecf60e9dafef87891589ccd54cf9b36f)
- [AeroFyl CLI and available flags](https://github.com/AeroForger/AeroFyl/blob/9dda7a4cecf60e9dafef87891589ccd54cf9b36f/bootstrap/rust/src/main.rs)
- [AeroFyl 64-bit integer semantics](https://github.com/AeroForger/AeroFyl/blob/9dda7a4cecf60e9dafef87891589ccd54cf9b36f/bootstrap/rust/README.md)
- [Pinned Sydrogen repository](https://github.com/AeroForger/Sydrogen/tree/8e8e31a448c1f98fa5e879f12a2bd916c4bb4bc6)
- [Sydrogen backend dispatch and Input fallback](https://github.com/AeroForger/Sydrogen/blob/8e8e31a448c1f98fa5e879f12a2bd916c4bb4bc6/src/backend/mod.rs)
- [Sydrogen Cranelift settings and integer lowering](https://github.com/AeroForger/Sydrogen/blob/8e8e31a448c1f98fa5e879f12a2bd916c4bb4bc6/src/codegen.rs)

## Protocol

1. Compile all programs before timing runtime execution. Use the real compilers from the pinned repositories; no compiler-source patches.
2. Read `n` and `seed` from stdin in the primary suite. Print one integer result. GCC and Go may optimize the program normally.
3. Compare all implementations against independent Python oracles: `%` recurrences, a closed-form branch result, iterative Fibonacci, and a sieve for prime counts. Validate five small/edge fixtures and the timed fixture per workload. Small cases include zero iterations, one iteration and different seeds.
4. Run two untimed warmups per configuration. Run 11 measured rounds per workload, shuffling configuration order in each round with RNG seed 4201.
5. Measure with Python `time.perf_counter_ns()` around `subprocess.run`. This is **process-start-to-exit wall time**, including program startup, input, output and the runner's subprocess overhead. Compilation is excluded. Programs run sequentially, with the runner and children pinned to one logical CPU.
6. Keep every sample. Report the median, minimum, maximum and median absolute deviation (MAD). No outlier removal, baseline subtraction or fastest-run selection.
7. Record tool versions, compiler hashes, source hashes, exact build commands, build stdout/stderr, validation results and raw timing samples.

The main suite has eight configurations: AeroFyl, GCC `-O0`, `-O2`, `-O3`, CPython, Go, Sydrogen `--backend native` (fallback), and Sydrogen `--backend cranelift`.

- AeroFyl uses its existing bootstrap CLI, with no unsupported optimization flags.
- Go uses `go build`, followed by execution of the built binary; `go run` is not timed.
- Python uses CPython without NumPy, PyPy or a JIT.
- Sydrogen's Cranelift setup was left at its default settings. No compiler optimization settings were changed.
- C uses `int64_t`, Go uses `int64`, AeroFyl uses its signed 64-bit `int`, and Sydrogen Cranelift uses 32-bit `Int`. Every tested value fits signed 32-bit. Separate C32 and Go32 controls make the width difference visible.
- All operands involved in division are nonnegative and divisors are positive. Python `//` therefore agrees with integer division in the compiled versions for these fixtures.
- AeroFyl does not accept `%` in this bootstrap. All timed versions use the equivalent expression `x - (x / divisor) * divisor`. The reference oracle uses `%`.

## Workload definitions

| Workload | n | seed | Algorithm |
| --- | --- | --- | --- |
| Bounded arithmetic | 5000000 | 7 | Repeat x=(31x+17) mod 1,000,003 using division-based remainder |
| Predictable branches | 5000000 | 7 | First floor(n/2) iterations add 3; remaining iterations subtract 1 |
| State-dependent branches | 5000000 | 7 | Branch on x<500,001; use (31x+17) or (37x+11), then bounded remainder |
| Function calls | 3000000 | 7 | Call step(x), returning (3x+7) mod 1,000,003 |
| Recursive Fibonacci | 31 | 7 | Naive recursive fib(n), then add seed; no memoization |
| Prime counting | 80000 | 7 | Trial divide each candidate from 2 through n by every d with d*d<=candidate; no early exit; return count+seed |
| Startup control | 0 | 7 | Read inputs and print seed; no kernel work |

The prime algorithm is deliberately identical across implementations, including its lack of an early exit. It is not a recommendation for implementing a prime generator. The state-dependent branch test is not asserted to produce any particular branch-prediction rate. Function calls may be inlined; this test measures the complete program, not the cost of a single machine-level call.

## Median runtime: all configurations

Milliseconds; lower is faster. "Syd N->CL" means native requested, Cranelift fallback used.

| Workload | AeroFyl | C O0 | C O2 | C O3 | Python | Go | Syd N->CL | Syd CL |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Bounded arithmetic | 31.242 | 16.990 | 16.577 | 17.088 | 391.943 | 22.020 | 25.606 | 25.642 |
| Predictable branches | 17.395 | 4.935 | 2.899 | 1.599 | 291.077 | 3.536 | 9.405 | 9.531 |
| State-dependent branches | 36.356 | 23.663 | 19.482 | 19.235 | 500.921 | 23.742 | 27.880 | 27.919 |
| Function calls | 21.169 | 11.676 | 11.520 | 11.480 | 298.932 | 13.348 | 17.359 | 17.184 |
| Recursive Fibonacci | 11.019 | 10.425 | 3.306 | 3.159 | 114.907 | 7.379 | 7.795 | 7.668 |
| Prime counting | 60.442 | 41.849 | 39.128 | 39.186 | 782.049 | 40.030 | 28.134 | 28.192 |
| Startup control | 1.298 | 1.488 | 1.465 | 1.538 | 10.498 | 1.997 | 1.509 | 1.588 |

## Test 1 - AeroFyl versus C

Each ratio is **AeroFyl time / C time**. A value above 1 means AeroFyl took longer.

| Workload | vs O0 | vs O2 | vs O3 |
| --- | --- | --- | --- |
| Bounded arithmetic | 1.84x | 1.88x | 1.83x |
| Predictable branches | 3.52x | 6.00x | 10.88x |
| State-dependent branches | 1.54x | 1.87x | 1.89x |
| Function calls | 1.81x | 1.84x | 1.84x |
| Recursive Fibonacci | 1.06x | 3.33x | 3.49x |
| Prime counting | 1.44x | 1.54x | 1.54x |

GCC-generated assembly is included for bounded arithmetic, function calls and predictable branches. In the function-call kernel, GCC O3's `work` loop has no call to `step`; its body contains the inlined calculation. Predictable-branch assembly contains loop transformations and unrolling. These tests therefore cannot establish that one language "excels at branches" independently of the rest of its generated program.

## Test 2 - AeroFyl versus Python

The speedup column is **Python time / AeroFyl time**.

| Workload | AeroFyl ms | Python ms | Measured AeroFyl speedup |
| --- | --- | --- | --- |
| Bounded arithmetic | 31.242 | 391.943 | 12.55x |
| Predictable branches | 17.395 | 291.077 | 16.73x |
| State-dependent branches | 36.356 | 500.921 | 13.78x |
| Function calls | 21.169 | 298.932 | 14.12x |
| Recursive Fibonacci | 11.019 | 114.907 | 10.43x |
| Prime counting | 60.442 | 782.049 | 12.94x |

AeroFyl was faster in all six measured kernels. This comparison includes CPython's interpreter startup and execution; it does not represent Python code delegated to compiled numerical libraries.

## Test 3 - AeroFyl versus Sydrogen

The primary stdin table compares AeroFyl against Sydrogen's typed Cranelift output. AeroFyl took longer in all six kernels in that phase.

For the actual direct-native comparison, a separate phase replaces only the `Input` initializers with the same literal `n` and `seed` in **both** AeroFyl and Sydrogen. The work functions and algorithms are retained. Each result is checked on every execution. `readelf -l` confirms no `INTERP` segment in AeroFyl and direct-native Sydrogen executables, while the fixed-input Cranelift executables have an interpreter segment. Full program headers are stored in `native-control.json`.

The follow-up verifies the compiled executables again, checks outputs on every run, and uses **21 rounds**, two warmups, randomized order (seed 4203), on the same pinned CPU. All samples are retained. Compare within this phase, not against the earlier stdin phase.

## Build notes for the closing bootstrap

The closing Rust bootstrap shows a nondeterministic compile-time pathology: the identical AeroFyl build command usually finishes in milliseconds but sometimes exceeds the 120 second per-attempt budget (observed on `state_branches` in pre-run probes, roughly every second attempt). Compilation is excluded from measured runtimes, so the Pre-Alpha-2 harness retries the AeroFyl build command until it succeeds, up to 10 attempts. Every attempt is recorded in `build-record.json` and `native-control.json`.
| Workload | AeroFyl fixed ms | Syd direct native ms | Syd CL fixed ms | Aero / direct native |
| --- | --- | --- | --- | --- |
| Bounded arithmetic | 30.858 | 28.476 | 25.394 | 1.08x |
| Predictable branches | 15.366 | 12.082 | 8.576 | 1.27x |
| State-dependent branches | 33.304 | 32.077 | 25.632 | 1.04x |
| Function calls | 19.121 | 17.136 | 15.825 | 1.12x |
| Recursive Fibonacci | 10.190 | 10.595 | 7.840 | 0.96x |
| Prime counting | 57.544 | 40.127 | 26.848 | 1.43x |
| Startup control | 0.158 | 0.176 | 1.421 | 0.90x |

For these six fixed-input kernels, AeroFyl took **1.14x** as long as actual direct-native Sydrogen by the geometric mean. This is a separate experiment with fixed inputs.

Retry record: the following builds needed more than one attempt under the retry rule above.

| Build | Attempts |
| --- | --- |
| predictable_branches/AeroFyl-fixed | 2 attempts (timeout_120s, ok) |

## Test 4 - AeroFyl versus Go

Ratio: **AeroFyl time / Go time**.

| Workload | AeroFyl ms | Go ms | Aero / Go |
| --- | --- | --- | --- |
| Bounded arithmetic | 31.242 | 22.020 | 1.42x |
| Predictable branches | 17.395 | 3.536 | 4.92x |
| State-dependent branches | 36.356 | 23.742 | 1.53x |
| Function calls | 21.169 | 13.348 | 1.59x |
| Recursive Fibonacci | 11.019 | 7.379 | 1.49x |
| Prime counting | 60.442 | 40.030 | 1.51x |

Go was faster in all six measured kernels. Both implementations used signed 64-bit arithmetic in this main comparison.

## Supplement: integer-width controls

This is another separate phase: C `int32_t` at O3, Go `int32`, remeasured AeroFyl, and remeasured Sydrogen Cranelift. It uses the same stdin cases, six validation fixtures per program and configuration, two warmups, 11 measured rounds, and shuffled order with seed 4202. All values and outputs agree with the same oracles.

| Workload | AeroFyl ms | C32 O3 ms | Go32 ms | Syd CL32 ms |
| --- | --- | --- | --- | --- |
| Bounded arithmetic | 30.429 | 18.879 | 17.998 | 25.679 |
| Predictable branches | 15.778 | 1.675 | 4.258 | 8.900 |
| State-dependent branches | 34.727 | 20.321 | 20.329 | 26.262 |
| Function calls | 19.127 | 11.290 | 11.902 | 15.798 |
| Recursive Fibonacci | 10.485 | 3.058 | 7.361 | 7.289 |
| Prime counting | 57.830 | 22.614 | 23.570 | 26.980 |
| Startup control | 1.246 | 1.541 | 2.006 | 1.555 |

For this phase, AeroFyl also took longer than each of these three controls in all six kernels. The 32-bit controls do not turn the main comparison into a same-width comparison; they document an additional comparison instead.

## Timing spread

Main phase: median [minimum-maximum] milliseconds. All 11 samples, and MAD, are in `results.json`. Scheduling noise and short process runtimes limit precision. Affinity constrains CPU selection; it does not give this run exclusive use of a physical core.

| Workload | AeroFyl | C O3 | Python | Go | Syd CL |
| --- | --- | --- | --- | --- | --- |
| Bounded arithmetic | 31.242 [29.929-34.288] | 17.088 [16.249-17.736] | 391.943 [383.491-401.971] | 22.020 [20.519-22.831] | 25.642 [24.468-26.585] |
| Predictable branches | 17.395 [16.284-17.805] | 1.599 [1.416-1.918] | 291.077 [283.554-295.430] | 3.536 [2.502-3.886] | 9.531 [8.861-9.737] |
| State-dependent branches | 36.356 [34.107-38.051] | 19.235 [18.337-19.889] | 500.921 [489.664-502.400] | 23.742 [22.347-24.509] | 27.919 [26.055-30.573] |
| Function calls | 21.169 [19.887-21.689] | 11.480 [10.244-12.069] | 298.932 [297.912-304.427] | 13.348 [12.235-14.224] | 17.184 [16.137-17.692] |
| Recursive Fibonacci | 11.019 [10.036-11.388] | 3.159 [3.040-4.512] | 114.907 [113.398-116.474] | 7.379 [6.164-8.299] | 7.668 [6.633-8.133] |
| Prime counting | 60.442 [58.767-61.555] | 39.186 [37.538-40.901] | 782.049 [773.531-790.925] | 40.030 [38.310-41.943] | 28.192 [26.497-29.241] |
| Startup control | 1.298 [0.267-1.479] | 1.538 [1.397-1.726] | 10.498 [10.000-11.096] | 1.997 [0.925-2.234] | 1.588 [1.455-1.825] |

Fixed-input follow-up: median [minimum-maximum] ms.

| Workload | AeroFyl fixed | Syd direct native | Syd CL fixed |
| --- | --- | --- | --- |
| Bounded arithmetic | 30.858 [28.342-32.572] | 28.476 [27.405-29.794] | 25.394 [24.018-26.784] |
| Predictable branches | 15.366 [14.204-16.191] | 12.082 [11.650-13.826] | 8.576 [7.353-9.140] |
| State-dependent branches | 33.304 [32.154-35.871] | 32.077 [30.137-33.891] | 25.632 [24.171-26.965] |
| Function calls | 19.121 [17.829-21.757] | 17.136 [16.083-19.333] | 15.825 [14.641-18.235] |
| Recursive Fibonacci | 10.190 [8.633-10.588] | 10.595 [9.861-11.512] | 7.840 [6.790-8.547] |
| Prime counting | 57.544 [56.008-59.106] | 40.127 [39.480-42.855] | 26.848 [25.966-30.719] |
| Startup control | 0.158 [0.107-1.387] | 0.176 [0.115-1.355] | 1.421 [0.309-1.769] |

## Executable file sizes

Primary phase, minimum-maximum bytes across the seven compiled programs. These are on-disk executable sizes as emitted by the build commands, with no stripping step. Python's source size is not compared to executable sizes. Dynamically linked executables exclude external libraries; sizes are not complete installation footprints.

| Configuration | Min bytes | Max bytes |
| --- | --- | --- |
| AeroFyl | 8498 | 12594 |
| C-O0 | 16120 | 16160 |
| C-O2 | 16120 | 16160 |
| C-O3 | 16120 | 16160 |
| Go | 2546432 | 2555417 |
| Sydrogen-native | 16896 | 16968 |
| Sydrogen-cranelift | 16912 | 16984 |

The AeroFyl startup control was faster than all main-phase controls in the recorded median. Its ELF is statically linked without a program interpreter; the main-phase C, Go and Sydrogen programs are identified individually by the bundled `file` output. This observation is about tiny-process wall time and does not reverse the six kernel results.

## What this test covers and leaves open

**Measured:** these six scalar integer workloads, a startup control, eight main configurations, integer-width controls, verified direct-native Sydrogen output, output correctness for the stated fixtures, and executable file sizes.

**Not measured:** arrays/lists, allocation throughput, strings, floating point, SIMD, parallelism, files, networking, GPU execution, peak memory, energy, or performance on other machines. There is no claim about arbitrary programs or a distribution of real-world applications. These fixtures are finite checks, not proofs of compiler correctness.

The suite allows legitimate optimizations such as inlining and loop transformations. It does not force each implementation to execute the same number of machine instructions. Requiring that would change the question being tested.

Build wall times are recorded as single command observations in `results.json` and `build-record.json`, including warm Go build caches and system linking. They are **not** a controlled compiler-speed benchmark and are not ranked here.

## Relation to the Pre-Alpha bundle

The earlier Pre-Alpha-Benchmark-Bundle used AeroFyl commit `10d75e72f73158da9cc9897b5c273c70ef5a45c0` on different hardware and toolchains. Direct timing comparisons across the two bundles are not valid. The valid comparison is the pattern of ratios within each bundle.

## Reproduce

Prerequisites: Linux x86-64, GCC, Python 3, Git, `readelf`, rustup with Rust 1.99.0, and Go 1.27.1 on PATH. The package contains benchmark sources and scripts, not the language compilers or toolchains. `reproduce.sh` clones both repositories into a new `checkouts` directory and detaches them at the exact revisions above. It makes no remote writes.

```bash
rustup toolchain install 1.99.0 --profile minimal
# Install Go 1.27.1 using the official Go distribution for Linux amd64.
bash reproduce.sh
```

Equivalent main runtime commands, after building the compilers:

```bash
python3 generate.py
python3 run.py \
  --aero checkouts/AeroFyl/target/release/aerofyl-bootstrap \
  --furnace checkouts/Sydrogen/target/release/furnace \
  --go go
python3 width_control.py --go go
python3 native_control.py \
  --aero checkouts/AeroFyl/target/release/aerofyl-bootstrap \
  --furnace checkouts/Sydrogen/target/release/furnace
python3 make_report.py
```

Exact original commands and absolute source paths are retained in the raw JSON. The reproduction script resolves its own directory, so the archive can be extracted elsewhere. Source SHA-256 values in the primary results allow comparison with the measured source files.

## Bundle contents

- `sources/`: each workload in AeroFyl, C, Python, Go and Sydrogen, plus width and fixed-input controls.
- `run.py`, `width_control.py`, `native_control.py`: build, validation and timing harnesses.
- `generate.py`, `cases.json`, `reproduce.sh`, `capture_environment.py`, `make_report.py`: generation and reproduction.
- `results.json`, `build-record.json`, `validation.json`, `environment.json`: primary raw results and environment evidence.
- `width-control.json`, `native-control.json`: supplementary samples.
- Build/run logs, executable-format records and selected GCC disassemblies.

No benchmark binaries or downloaded toolchains are included in the archive.
