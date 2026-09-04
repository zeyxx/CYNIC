#![allow(clippy::print_stdout, clippy::print_stderr)]
//! `runtime queue` thin alias — delegates to `crate::queue_surface` so
//! `cynic runtime queue --json` and `cynic queue --json` share one path.

use std::process::ExitCode;

pub(crate) fn execute_queue(json: bool, apply: bool) -> ExitCode {
    crate::queue_surface::execute_queue(json, apply)
}
