//! Command-line entry point for KOKURA.
//!
//! Delegate argument parsing to args, JSON job loading to json_io, and ROM
//! execution/artifact generation to run. Direct commands and job files share
//! the same execution facilities.

mod args;
mod json_io;
mod run;

use anyhow::Result;
use args::Args;
use clap::Parser;

// Let clap handle command-line syntax and help, then propagate execution errors to the process entry point.
fn main() -> Result<()> {
    let args = Args::parse();
    run::run(args)
}
