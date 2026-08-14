use std::process::ExitCode;

use cynic_inference::{ChatSessionStore, FsChatSessionStore, PrincipalPaths};
use serde::Serialize;

use super::runtime::ChatRuntime;

pub(crate) fn blocked_not_ready(
    runtime: &ChatRuntime,
    _paths: &PrincipalPaths,
) -> cynic_inference::ChatTurnReport {
    cynic_inference::ChatTurnReport {
        schema: "cynic.chat.turn.v1",
        status: "blocked".into(),
        session_id: String::new(),
        ready: false,
        content: None,
        stream_chunks: vec![],
        events_appended: 0,
        tool_turns: 0,
        endpoint: runtime.endpoint.clone(),
        model: runtime.model.clone(),
        session_root: runtime.session_root.clone(),
        message: "inference not ready".into(),
        blocked_reason: Some(runtime.detail.clone()),
    }
}

pub(crate) fn list_sessions(
    store: &FsChatSessionStore,
    json: bool,
    paths: &PrincipalPaths,
    root: &str,
) -> ExitCode {
    match store.list_sessions() {
        Ok(list) => {
            if json {
                return emit(&serde_json::json!({
                    "schema": "cynic.chat.sessions.v1",
                    "status": "ok",
                    "principal": paths.runtime_user,
                    "authority": paths.authority,
                    "session_root": root,
                    "sessions": list,
                }));
            }
            println!(
                "CYNIC chat sessions  principal={}  root={}",
                paths.runtime_user, root
            );
            if list.is_empty() {
                println!("  (none)");
            }
            for s in list {
                println!(
                    "  {}  events={}  model={}  updated_ms={}",
                    s.id,
                    s.event_count,
                    s.model.as_deref().unwrap_or("-"),
                    s.updated_at_unix_ms
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn export_session(store: &FsChatSessionStore, id: &str, json: bool) -> ExitCode {
    match store.export_jsonl(id) {
        Ok(raw) => {
            if json {
                return emit(&serde_json::json!({
                    "schema": "cynic.chat.export.v1",
                    "status": "ok",
                    "session_id": id,
                    "jsonl": raw,
                }));
            }
            print!("{raw}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn print_human(report: &cynic_inference::ChatTurnReport, paths: &PrincipalPaths) {
    println!("CYNIC chat");
    println!(
        "  status={}  session={}  principal={}",
        report.status, report.session_id, paths.runtime_user
    );
    println!("  ready={}  root={}", report.ready, report.session_root);
    if let Some(b) = &report.blocked_reason {
        println!("  blocked: {b}");
    }
    if report.tool_turns > 0 {
        println!("  tool_turns={}", report.tool_turns);
    }
    if !report.stream_chunks.is_empty() {
        print!("  stream: ");
        for c in &report.stream_chunks {
            if let Some(d) = &c.content_delta {
                print!("{d}");
            }
        }
        println!();
    }
    if let Some(c) = &report.content {
        println!("\n{c}\n");
    } else {
        println!("  message: {}", report.message);
    }
}

pub(crate) fn emit(value: &impl Serialize) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(s) => {
            println!("{s}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("json error: {e}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn emit_chat_json(value: &impl Serialize) -> ExitCode {
    emit(value)
}
