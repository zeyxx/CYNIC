//! Interactive chat TUI facade over cynic-inference.

use std::process::ExitCode;

/// Run fullscreen interactive chat. Returns process exit code.
pub(crate) fn run_chat_tui(
    _mock: bool,
    _prefer_ods: bool,
    _stream: bool,
    _model: Option<&str>,
) -> ExitCode {
    eprintln!(
        "Interactive chat TUI is currently undergoing stabilization. Please provide a prompt: `cynic chat --prompt \"...\"`."
    );
    ExitCode::FAILURE
}
