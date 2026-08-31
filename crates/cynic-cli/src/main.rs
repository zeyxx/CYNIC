#![allow(clippy::print_stdout, clippy::print_stderr)]
//! `cynic` binary — thin CLI facade conforming to Library-First Pattern (§472).
//!
//! All domain code, command definitions, and dispatch logic live in `cynic_cli` (lib.rs).

use std::process::ExitCode;

fn main() -> ExitCode {
    cynic_cli::run_main()
}
