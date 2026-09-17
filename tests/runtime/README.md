# Runtime fixtures

The test runner compiles and executes the fixtures in this directory. It checks
normal execution of compiler-oriented strings, structs, enums, and growing
lists; integer wrapping and checked division; command-line mapping; file
writing; text and binary filesystem round trips; standard output/error, line
input and typed input failures; explicit conversions; and the observable status
from `exit(23)`.
More detailed boundary and failure coverage lives in
`bootstrap/rust/src/driver.rs`.
