#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::process::ExitCode;

fn main() -> ExitCode {
    cynic_host::lifecycle::evidence_cli::main_from_env()
}
