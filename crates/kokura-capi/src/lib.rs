//! Public C ABI entry points.
//!
//! Expose core and debug functionality through opaque handles, explicit release
//! functions, owned UTF-8 strings and fixed-layout FFI types, keeping Rust
//! ownership details behind the ABI boundary.

pub mod bridge;
pub mod core;
pub mod debug;
pub mod ffi_types;
pub mod strings;
