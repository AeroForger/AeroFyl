# Current Limitations

The executable backend supports Linux x86-64 only. Floating-point execution is absent. The bootstrap supports only selected aggregate layouts and does not reclaim process-lifetime allocations.

The following areas remain unspecified or incomplete:

- exact identifier rules
- comments and escape syntax
- integer overflow and language-level failure behavior
- floating-point execution
- conversions, coercions, and `dynamic`
- modules and imports
- project manifest naming and keys
- tuple values
- general collection inference and representation
- `print` and `input`
- permanent entry-point and command-line argument rules
- ownership, references, and memory management

The complete boundary is maintained in the repository `spec/` directory and the Rust bootstrap README.
