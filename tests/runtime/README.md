# Runtime fixtures

Runtime execution coverage currently lives in `bootstrap/rust/src/driver.rs`, including bounds failures, list growth, strings, file reads, and command-line arguments. Add file-based fixtures here when the runner can express expected exit status and input files without duplicating those tests.
