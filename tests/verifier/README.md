# IR verifier fixtures

IR verifier coverage currently uses Rust tests in `bootstrap/rust/src/middle/verify.rs`, where malformed IR can be constructed directly. Aerofyl source cannot represent invalid internal IR, so no `.fyl` verifier fixtures are added yet.
