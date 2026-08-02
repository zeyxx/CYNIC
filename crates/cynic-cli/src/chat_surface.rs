//! Thin operator chat surface — history + stream/tools via cynic-inference.
//!
//! Modes:
//! - **TUI** (default when no prompt): interactive fullscreen (see `chat_tui`)
//! - **one-shot** when a prompt is given
//! - **list/export** management
//!
//! Fail-closed without `--mock` / live endpoint (env, ODS, or native probe).
//! Does not require cynic-kernel.

#[path = "chat_surface/args.rs"]
mod args;
#[path = "chat_surface/command.rs"]
mod command;
#[path = "chat_surface/handlers.rs"]
mod handlers;
#[path = "chat_surface/runtime.rs"]
pub(crate) mod runtime;
#[cfg(test)]
#[path = "chat_surface/tests.rs"]
mod tests;

pub(crate) use args::ChatArgs;
pub(crate) use command::execute_chat;
pub(crate) use handlers::{
    blocked_not_ready, emit_chat_json, export_session, list_sessions, print_human,
};
pub(crate) use runtime::{
    ChatRuntime, build_chat_runtime, rebind_chat_service, resolve_selector_to_alias,
};
