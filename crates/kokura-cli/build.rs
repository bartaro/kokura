//! Host-specific build configuration for the CLI.
//!
//! On Windows, request a larger main-thread stack for large debug reports
//! and state-restoration workflows.

// On Windows, emit a linker argument requesting a 16 MiB main-thread
// stack for large report/state workflows; other build hosts emit nothing.
fn main() {
    #[cfg(target_os = "windows")]
    {
        // The debug CLI keeps large report/session construction frames alive at once.
        // A larger main-thread stack keeps state-load/report workflows stable on Windows.
        println!("cargo:rustc-link-arg=/STACK:16777216");
    }
}
