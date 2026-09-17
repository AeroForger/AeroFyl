# Current Limitations

The executable backend supports Linux x86-64 only. Floating-point execution is absent. The bootstrap supports only selected aggregate layouts and does not reclaim process-lifetime allocations.

The following areas remain unspecified or incomplete:

- exact identifier rules
- comments and string escape syntax
- formatted standard-error diagnostics
- floating-point execution
- conversions beyond the documented integer/byte/character/enum subset, coercions, and `dynamic`
- qualified user-module paths, aliases, and packages
- project manifest naming and keys
- tuple values
- general collection inference and representation
- entry-point forms beyond the two documented signatures
- reclamation, user-visible references, and general ownership

The complete boundary is maintained in the repository `spec/` directory and the Rust bootstrap README.
