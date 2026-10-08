# AeroFyl documented benchmark: C, Python, Sydrogen and Go

Measured run started: **2026-10-04T16:42:02Z**. All timings are from the execution environment described below, **not Davit's laptop**. This is a scalar integer microbenchmark suite at pinned compiler revisions; its results do not establish a universal language-speed ranking.

## Observed results

All main-suite outputs passed 336 validation executions (seven programs × six input fixtures × eight configurations), followed by output checks on every warmup and timed execution. There were no failed comparisons.

Across the **six selected workloads**, the geometric mean of per-workload ratios was:

| Requested comparison | Measured geometric mean |
| --- | --- |
| Test 1 - C | AeroFyl 5.01× slower |
| Test 2 - Python | AeroFyl 6.93× faster |
| Test 3 - Sydrogen (Cranelift, stdin) | AeroFyl 3.05× slower |
| Test 4 - Go | AeroFyl 3.58× slower |

A geometric mean gives each of these six workloads equal weight. It excludes the startup control and is not the ratio of total elapsed times. The per-workload numbers below are the primary result.

**Backend discovery:** at this Sydrogen revision, `Expr::Input(_)` and `Statement::Input(_)` cause native requests to fall back to the typed Cranelift path. Consequently, both stdin Sydrogen configurations in the main run used Cranelift. The separate fixed-input comparison below verifies actual direct-native output. The two main-run CLI configurations must not be presented as independent backend implementations.

## Environment and pinned sources

| Item | Observed configuration |
| --- | --- |
| CPU | Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz |
| Execution environment | KVM virtual machine; Linux x86-64 |
| OS | Linux-6.18.44-x86_64-with-glibc2.39 |
| Affinity | Runner and child processes pinned to logical CPU 0; original allowed set [0, 1, 2, 3, 4, 5, 6, 7, 8] |
| Resource limits | cpu.max=800000 100000; memory.max=8589934592 bytes |
| C | gcc (Ubuntu 13.3.0-6ubuntu2~24.04) 13.3.0 |
| Python | 3.12.14 (main, Aug 25 2026, 14:00:49) [Clang 22.1.3 ] |
| Go | go version go1.27.1 linux/amd64 |
| Rust compiler used to build both compilers | rustc 1.99.0 (b940084d7 2026-09-28) |
| AeroFyl | Stage 0 Rust bootstrap; commit `10d75e72f73158da9cc9897b5c273c70ef5a45c0` |
| Sydrogen | Furnace Alpha-7; commit `8e8e31a448c1f98fa5e879f12a2bd916c4bb4bc6` |

Both compiler checkouts were clean and unmodified. Both compiler executables were built using `cargo build --release --locked`. A release build optimizes the **compiler executable**; it does not add an optimization flag to the generated AeroFyl or Sydrogen program.

Sources:

- [Pinned AeroFyl repository](https://github.com/AeroForger/AeroFyl/tree/10d75e72f73158da9cc9897b5c273c70ef5a45c0)
- [AeroFyl CLI and available flags](https://github.com/AeroForger/AeroFyl/blob/10d75e72f73158da9cc9897b5c273c70ef5a45c0/bootstrap/rust/src/main.rs)
- [AeroFyl 64-bit integer semantics](https://github.com/AeroForger/AeroFyl/blob/10d75e72f73158da9cc9897b5c273c70ef5a45c0/bootstrap/rust/README.md)
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
- Sydrogen's Cranelift setup leaves `opt_level` at the dependency's default `none`; the installed Cranelift 0.135.2 source confirms this default. No compiler optimization settings were changed.
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

Milliseconds; lower is faster. “Syd N→CL” means native requested, Cranelift fallback used.

| Workload | AeroFyl | C O0 | C O2 | C O3 | Python | Go | Syd N→CL | Syd CL |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Bounded arithmetic | 97.962 | 26.686 | 26.476 | 25.464 | 529.007 | 29.944 | 36.352 | 35.728 |
| Predictable branches | 32.485 | 15.034 | 5.507 | 3.564 | 361.607 | 6.006 | 11.566 | 11.696 |
| State-dependent branches | 105.457 | 44.727 | 24.648 | 24.874 | 680.695 | 30.566 | 36.716 | 37.194 |
| Function calls | 80.137 | 30.367 | 14.377 | 14.719 | 426.659 | 17.922 | 23.242 | 22.943 |
| Recursive Fibonacci | 29.543 | 13.552 | 4.663 | 4.374 | 202.213 | 10.934 | 10.985 | 11.165 |
| Prime counting | 150.093 | 53.381 | 50.644 | 52.048 | 1174.068 | 52.679 | 36.305 | 37.539 |
| Startup control | 0.270 | 2.106 | 2.065 | 2.084 | 15.871 | 2.078 | 2.128 | 2.120 |

## Test 1 - AeroFyl versus C

Each ratio is **AeroFyl time / C time**. A value above 1 means AeroFyl took longer.

| Workload | vs O0 | vs O2 | vs O3 |
| --- | --- | --- | --- |
| Bounded arithmetic | 3.67× | 3.70× | 3.85× |
| Predictable branches | 2.16× | 5.90× | 9.11× |
| State-dependent branches | 2.36× | 4.28× | 4.24× |
| Function calls | 2.64× | 5.57× | 5.44× |
| Recursive Fibonacci | 2.18× | 6.34× | 6.75× |
| Prime counting | 2.81× | 2.96× | 2.88× |

GCC-generated assembly is included for bounded arithmetic, function calls and predictable branches. In the function-call kernel, GCC O3's `work` loop has no call to `step`; its body contains the inlined calculation. Predictable-branch assembly contains loop transformations and unrolling. These tests therefore cannot establish that one language “excels at branches” independently of the rest of its generated program.

## Test 2 - AeroFyl versus Python

The speedup column is **Python time / AeroFyl time**.

| Workload | AeroFyl ms | Python ms | Measured AeroFyl speedup |
| --- | --- | --- | --- |
| Bounded arithmetic | 97.962 | 529.007 | 5.40× |
| Predictable branches | 32.485 | 361.607 | 11.13× |
| State-dependent branches | 105.457 | 680.695 | 6.45× |
| Function calls | 80.137 | 426.659 | 5.32× |
| Recursive Fibonacci | 29.543 | 202.213 | 6.84× |
| Prime counting | 150.093 | 1174.068 | 7.82× |

AeroFyl was faster in all six measured kernels. This comparison includes CPython's interpreter startup and execution; it does not represent Python code delegated to compiled numerical libraries.

## Test 3 - AeroFyl versus Sydrogen

The primary stdin table compares AeroFyl against Sydrogen's typed Cranelift output. AeroFyl took longer in all six kernels in that phase.

For the actual direct-native comparison, a separate phase replaces only the `Input` initializers with the same literal `n` and `seed` in **both** AeroFyl and Sydrogen. The work functions and algorithms are retained. Each result is checked on every execution. `readelf -l` confirms no `INTERP` segment in AeroFyl and direct-native Sydrogen executables, while the fixed-input Cranelift executables have an interpreter segment. Full program headers are stored in `native-control.json`.

The first 11-round fixed-input run had substantial timing spikes. It is preserved in `native-control-first-run.json`. A rebuild attempt for that follow-up hit a 120-second timeout compiling `predictable_branches.fixed.fyl` with the unmodified AeroFyl compiler; the successful initial build is still retained. The timeout traceback is in `native-control-rebuild-timeout.log`. This documents a failed rebuild attempt, not a failure of the recorded runtime outputs. The follow-up reuses the compiled executables, verifies their ELF formats again, and checks outputs on every run. It uses **21 rounds**, two warmups, randomized order (seed 4203), on the same pinned CPU. All samples are retained. Compare within this phase, not against the earlier stdin phase.

| Workload | AeroFyl fixed ms | Syd direct native ms | Syd CL fixed ms | Aero / direct native |
| --- | --- | --- | --- | --- |
| Bounded arithmetic | 99.862 | 55.075 | 36.510 | 1.81× |
| Predictable branches | 28.351 | 18.259 | 11.558 | 1.55× |
| State-dependent branches | 105.505 | 53.857 | 37.784 | 1.96× |
| Function calls | 85.038 | 40.280 | 22.343 | 2.11× |
| Recursive Fibonacci | 29.377 | 16.497 | 10.506 | 1.78× |
| Prime counting | 144.945 | 60.034 | 36.354 | 2.41× |
| Startup control | 1.288 | 1.285 | 1.149 | 1.00× |

For these six fixed-input kernels, AeroFyl took **1.92×** as long as actual direct-native Sydrogen by the geometric mean. This is a separate experiment with fixed inputs.

## Test 4 - AeroFyl versus Go

Ratio: **AeroFyl time / Go time**.

| Workload | AeroFyl ms | Go ms | Aero / Go |
| --- | --- | --- | --- |
| Bounded arithmetic | 97.962 | 29.944 | 3.27× |
| Predictable branches | 32.485 | 6.006 | 5.41× |
| State-dependent branches | 105.457 | 30.566 | 3.45× |
| Function calls | 80.137 | 17.922 | 4.47× |
| Recursive Fibonacci | 29.543 | 10.934 | 2.70× |
| Prime counting | 150.093 | 52.679 | 2.85× |

Go was faster in all six measured kernels. Both implementations used signed 64-bit arithmetic in this main comparison.

## Supplement: integer-width controls

This is another separate phase: C `int32_t` at O3, Go `int32`, remeasured AeroFyl, and remeasured Sydrogen Cranelift. It uses the same stdin cases, six validation fixtures per program and configuration, two warmups, 11 measured rounds, and shuffled order with seed 4202. All values and outputs agree with the same oracles.

| Workload | AeroFyl ms | C32 O3 ms | Go32 ms | Syd CL32 ms |
| --- | --- | --- | --- | --- |
| Bounded arithmetic | 97.880 | 28.009 | 27.364 | 36.611 |
| Predictable branches | 33.704 | 2.831 | 5.831 | 12.471 |
| State-dependent branches | 111.601 | 26.479 | 27.102 | 38.063 |
| Function calls | 86.402 | 14.996 | 15.733 | 22.516 |
| Recursive Fibonacci | 30.281 | 5.213 | 11.461 | 11.628 |
| Prime counting | 149.767 | 32.191 | 32.273 | 37.323 |
| Startup control | 0.265 | 2.007 | 2.067 | 2.188 |

For this phase, AeroFyl also took longer than each of these three controls in all six kernels. The 32-bit controls do not turn the main comparison into a same-width comparison; they document an additional comparison instead.

## Timing spread

Main phase: median [minimum–maximum] milliseconds. All 11 samples, and MAD, are in `results.json`. Shared virtual-machine scheduling and short process runtimes limit precision. Affinity constrains CPU selection; it does not give this run exclusive use of a physical core.

| Workload | AeroFyl | C O3 | Python | Go | Syd CL |
| --- | --- | --- | --- | --- | --- |
| Bounded arithmetic | 97.962 [93.794–106.630] | 25.464 [25.098–28.591] | 529.007 [515.930–695.729] | 29.944 [27.805–40.814] | 35.728 [34.067–38.224] |
| Predictable branches | 32.485 [31.142–34.138] | 3.564 [2.365–4.461] | 361.607 [353.522–439.043] | 6.006 [5.168–7.710] | 11.696 [10.356–13.395] |
| State-dependent branches | 105.457 [100.302–108.453] | 24.874 [23.203–26.951] | 680.695 [667.785–733.575] | 30.566 [29.552–34.555] | 37.194 [35.184–43.910] |
| Function calls | 80.137 [72.479–87.335] | 14.719 [13.261–15.886] | 426.659 [417.472–523.116] | 17.922 [16.817–19.673] | 22.943 [21.148–24.940] |
| Recursive Fibonacci | 29.543 [27.221–43.413] | 4.374 [3.583–5.728] | 202.213 [196.958–230.547] | 10.934 [9.520–14.927] | 11.165 [10.017–12.032] |
| Prime counting | 150.093 [141.460–164.256] | 52.048 [48.834–73.458] | 1174.068 [1149.670–1296.552] | 52.679 [49.933–58.738] | 37.539 [33.585–65.640] |
| Startup control | 0.270 [0.239–1.426] | 2.084 [0.916–2.745] | 15.871 [14.084–20.945] | 2.078 [1.199–2.612] | 2.120 [0.966–2.537] |

Fixed-input follow-up: median [minimum–maximum] ms.

| Workload | AeroFyl fixed | Syd direct native | Syd CL fixed |
| --- | --- | --- | --- |
| Bounded arithmetic | 99.862 [89.913–116.575] | 55.075 [52.013–81.781] | 36.510 [33.083–44.507] |
| Predictable branches | 28.351 [26.502–35.980] | 18.259 [15.290–33.867] | 11.558 [10.425–20.166] |
| State-dependent branches | 105.505 [99.848–111.041] | 53.857 [51.084–59.948] | 37.784 [35.331–43.304] |
| Function calls | 85.038 [81.824–114.688] | 40.280 [38.435–45.941] | 22.343 [20.955–23.975] |
| Recursive Fibonacci | 29.377 [27.831–36.293] | 16.497 [15.159–18.395] | 10.506 [9.314–11.860] |
| Prime counting | 144.945 [140.889–154.731] | 60.034 [55.028–65.992] | 36.354 [33.239–45.670] |
| Startup control | 1.288 [0.211–1.698] | 1.285 [0.235–1.540] | 1.149 [0.894–2.343] |

## Executable file sizes

Primary phase, minimum–maximum bytes across the seven compiled programs. These are on-disk executable sizes as emitted by the build commands, with no stripping step. Python's source size is not compared to executable sizes. Dynamically linked executables exclude external libraries; sizes are not complete installation footprints.

| Configuration | Min bytes | Max bytes |
| --- | --- | --- |
| AeroFyl | 8498 | 12594 |
| C-O0 | 16096 | 16136 |
| C-O2 | 16104 | 16136 |
| C-O3 | 16104 | 16136 |
| Go | 2491019 | 2491803 |
| Sydrogen-native | 16808 | 16880 |
| Sydrogen-cranelift | 16824 | 16896 |

The AeroFyl startup control was faster than all main-phase controls in the recorded median. Its ELF is statically linked without a program interpreter; the main-phase C, Go and Sydrogen programs are identified individually by the bundled `file` output. This observation is about tiny-process wall time and does not reverse the six kernel results.

## What this test covers and leaves open

**Measured:** these six scalar integer workloads, a startup control, eight main configurations, integer-width controls, verified direct-native Sydrogen output, output correctness for the stated fixtures, and executable file sizes.

**Not measured:** arrays/lists, allocation throughput, strings, floating point, SIMD, parallelism, files, networking, GPU execution, peak memory, energy, or performance on Davit's laptop. There is no claim about arbitrary programs or a distribution of real-world applications. These fixtures are finite checks, not proofs of compiler correctness.

The suite allows legitimate optimizations such as inlining and loop transformations. It does not force each implementation to execute the same number of machine instructions. Requiring that would change the question being tested.

Build wall times are recorded as single command observations in `results.json` and `build-record.json`, including warm Go build caches and system linking. They are **not** a controlled compiler-speed benchmark and are not ranked here.

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
- `width-control.json`, `native-control.json`, `native-control-first-run.json`: supplementary samples and original noisy run.
- Build/run logs, executable-format records and selected GCC disassemblies.

No benchmark binaries or downloaded toolchains are included in the archive.
