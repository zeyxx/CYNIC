//! Interactive chat TUI facade over cynic-inference.
//!
//! Builds the same chat runtime as the `--prompt` path, then hands the
//! terminal to the cynic-tui conversation kit. Each submitted line runs one
//! blocking (non-streaming) turn through the chat service; the service owns
//! session persistence, the kit only renders.

use std::process::ExitCode;
use std::sync::Arc;

use crate::chat_surface::build_chat_runtime;

/// Run fullscreen interactive chat. Returns process exit code.
pub(crate) fn run_chat_tui(
    mock: bool,
    prefer_ods: bool,
    _stream: bool,
    model: Option<&str>,
) -> ExitCode {
    let title = format!(
        "cynic chat — {} (Esc quits)",
        model.unwrap_or("default model")
    );
    let handle = tokio::runtime::Handle::current();
    let model_owned = model.map(str::to_string);
    let setup = std::thread::spawn(move || {
        handle.block_on(build_chat_runtime(
            mock,
            prefer_ods,
            model_owned.as_deref(),
            true,
        ))
    })
    .join()
    .map_err(|_| "chat setup thread failed".to_string());
    let runtime = match setup {
        Ok(Ok(runtime)) => runtime,
        Ok(Err(e)) => {
            eprintln!("chat: {e}");
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("chat: {e}");
            return ExitCode::FAILURE;
        }
    };
    let Some(service) = runtime.service else {
        eprintln!("chat service missing");
        return ExitCode::FAILURE;
    };
    let service = Arc::new(service);
    // Turns execute on a helper thread that blocks on the current handle
    // (never nested block_on: the CLI already runs inside a runtime).
    let turn_handle = tokio::runtime::Handle::current();
    let session: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    cynic_tui::run_conversation(
        &title,
        vec![cynic_tui::Message::new(
            "system",
            "Type a message and press Enter. Esc on empty input quits.",
        )],
        &|line| {
            let resume = session.lock().ok().and_then(|guard| guard.clone());
            let worker = service.clone();
            let worker_handle = turn_handle.clone();
            let prompt = line.to_string();
            let turn = std::thread::spawn(move || {
                worker_handle.block_on(worker.turn(resume.as_deref(), &prompt, false))
            })
            .join()
            .map_err(|_| "chat turn thread failed".to_string())?;
            let report = turn;
            if report.status != "ok" {
                return Err(report
                    .blocked_reason
                    .clone()
                    .unwrap_or_else(|| report.message.clone()));
            }
            if let Ok(mut guard) = session.lock() {
                *guard = Some(report.session_id.clone());
            }
            Ok(report
                .content
                .clone()
                .unwrap_or_else(|| "(empty reply)".to_string()))
        },
    )
}
