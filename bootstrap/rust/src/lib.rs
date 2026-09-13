//! Temporary Rust bootstrap compiler for Aerofyl.
//!
//! The implementation intentionally covers only language rules documented by the
//! repository.  Missing language-design decisions are reported as diagnostics or
//! backend errors instead of being guessed here.

pub mod backend;
pub mod driver;
pub mod frontend;
pub mod middle;
pub mod project;
