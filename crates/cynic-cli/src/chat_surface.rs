//! Thin operator chat surface — history + stream/tools via cynic-inference.
//!
//! Modes:
//! - **TUI** (default when no prompt): interactive fullscreen (see `chat_tui`)
//! - **one-shot** when a prompt is given
//! - **list/export** management
//!
//! Fail-closed without `--mock` / live endpoint (env, ODS, or native probe).
//! Does not require cynic-kernel.

use std::process::ExitCode;
use std::sync::Arc;

use cynic_inference::{
    BuiltinToolInvoker, ChatEndpointSource, ChatService, ChatSessionStore, FsChatSessionStore,
    HttpOpenAiChatClient, MockChatClient, PrincipalPaths, ToolCallRequest, default_tool_specs,
    resolve_chat_endpoint, resolve_chat_state_dir,
};
use serde::Serialize;

#[derive(Debug, Clone)]
pub(crate) struct ChatArgs {
    pub prompt: Option<String>,
    pub resume: Option<String>,
    pub session_id: Option<String>,
    pub stream: bool,
    pub json: bool,
    pub list_sessions: bool,
    pub export: Option<String>,
    pub mock: bool,
    /// Prefer ODS OpenAI-compat discovery for endpoint resolution.
    pub ods: bool,
    /// Force non-interactive path (even without prompt shows help / blocked JSON).
    pub no_tui: bool,
}

/// Shared runtime handle for one-shot and TUI.
#[derive(Clone)]
pub(crate) struct ChatRuntime {
    pub store: Arc<FsChatSessionStore>,
    pub service: Option<ChatService>,
    pub ready: bool,
    pub use_mock: bool,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub source: String,
    pub detail: String,
    pub session_root: String,
}

/// Build principal session store + chat service (async endpoint resolve).
pub(crate) async fn build_chat_runtime(
    mock: bool,
    prefer_ods: bool,
) -> Result<ChatRuntime, String> {
    let root = if let Ok(p) = std::env::var("CYNIC_CHAT_STATE") {
        std::path::PathBuf::from(p)
    } else {
        resolve_chat_state_dir()
    };
    let store = FsChatSessionStore::new(&root).map_err(|e| e.to_string())?;
    let store = Arc::new(store);

    let env_mock = std::env::var("CYNIC_CHAT_MOCK")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let use_mock = mock || env_mock;

    if use_mock {
        let client: Arc<dyn cynic_inference::ChatCompletionClient> =
            Arc::new(MockChatClient::plain("(mock ready)"));
        let tools = Arc::new(BuiltinToolInvoker);
        let service = ChatService::new(
            store.clone() as Arc<dyn ChatSessionStore>,
            client,
            tools,
            default_tool_specs(),
            Some("mock://cynic-chat".into()),
            Some("mock-model".into()),
            4,
            true,
        );
        return Ok(ChatRuntime {
            store,
            service: Some(service),
            ready: true,
            use_mock: true,
            endpoint: Some("mock://cynic-chat".into()),
            model: Some("mock-model".into()),
            source: "mock".into(),
            detail: "explicit mock".into(),
            session_root: root.display().to_string(),
        });
    }

    let resolution = resolve_chat_endpoint(prefer_ods).await;
    if !resolution.ready {
        return Ok(ChatRuntime {
            store,
            service: None,
            ready: false,
            use_mock: false,
            endpoint: None,
            model: None,
            source: format!("{:?}", resolution.source).to_lowercase(),
            detail: resolution.detail,
            session_root: root.display().to_string(),
        });
    }

    let endpoint = resolution
        .endpoint
        .clone()
        .ok_or_else(|| "endpoint missing despite ready".to_string())?;
    let model = resolution
        .model
        .clone()
        .or_else(|| std::env::var("CYNIC_CHAT_MODEL").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".into());

    let client: Arc<dyn cynic_inference::ChatCompletionClient> = Arc::new(
        HttpOpenAiChatClient::with_api_key(&endpoint, &model, resolution.api_key.clone())
            .map_err(|e| e.to_string())?,
    );
    let tools = Arc::new(BuiltinToolInvoker);
    let service = ChatService::new(
        store.clone() as Arc<dyn ChatSessionStore>,
        client,
        tools,
        default_tool_specs(),
        Some(endpoint.clone()),
        Some(model.clone()),
        4,
        true,
    );

    let source = match resolution.source {
        ChatEndpointSource::Env => "env",
        ChatEndpointSource::InferenceConfig => "inference_config",
        ChatEndpointSource::Dns => "dns",
        ChatEndpointSource::Ods => "ods",
        ChatEndpointSource::NativeProbe => "native",
        ChatEndpointSource::Mock => "mock",
        ChatEndpointSource::None => "none",
    };

    Ok(ChatRuntime {
        store,
        service: Some(service),
        ready: true,
        use_mock: false,
        endpoint: Some(endpoint),
        model: Some(model),
        source: source.into(),
        detail: resolution.detail,
        session_root: root.display().to_string(),
    })
}

pub(crate) fn execute_chat(args: ChatArgs) -> ExitCode {
    let paths = PrincipalPaths::resolve();

    // Interactive TUI: default when no prompt and not list/export/json/no-tui
    let want_tui = args.prompt.is_none()
        && !args.list_sessions
        && args.export.is_none()
        && !args.json
        && !args.no_tui;

    if want_tui {
        return crate::chat_tui::run_chat_tui(args.mock, args.ods, args.stream);
    }

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("tokio runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match rt.block_on(build_chat_runtime(args.mock, args.ods)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("chat: {e}");
            return ExitCode::FAILURE;
        }
    };

    if args.list_sessions {
        return list_sessions(
            runtime.store.as_ref(),
            args.json,
            &paths,
            &runtime.session_root,
        );
    }
    if let Some(id) = &args.export {
        return export_session(runtime.store.as_ref(), id, args.json);
    }

    let Some(prompt) = args.prompt.clone() else {
        if args.json {
            let _ = emit(&serde_json::json!({
                "schema": "cynic.chat.turn.v1",
                "status": "blocked",
                "ready": runtime.ready,
                "message": "missing prompt — pass a prompt, use interactive TUI (cynic chat), or --list-sessions",
                "blocked_reason": "missing_prompt",
                "session_root": runtime.session_root,
                "principal": paths.runtime_user,
                "endpoint_source": runtime.source,
            }));
        } else {
            eprintln!("Usage:");
            eprintln!("  cynic chat                         # interactive TUI");
            eprintln!("  cynic chat --mock                  # TUI offline");
            eprintln!("  cynic chat --ods                   # TUI via ODS discovery");
            eprintln!("  cynic chat \"prompt\" [--mock|--ods] [--json] [--resume ID]");
            eprintln!("  cynic chat --list-sessions [--json]");
            eprintln!("  cynic chat --export <session-id>");
        }
        return ExitCode::FAILURE;
    };

    if !runtime.ready {
        let report = blocked_not_ready(&runtime, &paths);
        if args.json {
            let _ = emit(&report);
        } else {
            print_human(&report, &paths);
        }
        return ExitCode::FAILURE;
    }

    let service = match runtime.service {
        Some(s) => s,
        None => {
            eprintln!("chat service missing");
            return ExitCode::FAILURE;
        }
    };

    // Rebuild mock client per-prompt when mock so tool keyword works
    let service = if runtime.use_mock {
        let client: Arc<dyn cynic_inference::ChatCompletionClient> =
            if prompt.to_lowercase().contains("tool") {
                Arc::new(MockChatClient::with_tool(
                    ToolCallRequest {
                        id: "call_cli".into(),
                        name: "echo".into(),
                        arguments_json: format!(
                            r#"{{"text":{}}}"#,
                            serde_json::to_string(&prompt).unwrap_or_else(|_| "\"ok\"".into())
                        ),
                    },
                    format!("(mock) completed after tool: {prompt}"),
                ))
            } else {
                Arc::new(MockChatClient::plain(format!("(mock) {prompt}")))
            };
        ChatService::new(
            runtime.store.clone() as Arc<dyn ChatSessionStore>,
            client,
            Arc::new(BuiltinToolInvoker),
            default_tool_specs(),
            runtime.endpoint.clone(),
            runtime.model.clone(),
            4,
            true,
        )
    } else {
        service
    };

    let sid = args.resume.clone().or(args.session_id.clone());
    let report = rt.block_on(service.turn(sid.as_deref(), &prompt, args.stream));

    if args.json {
        let code = emit(&report);
        return if report.status == "ok" {
            code
        } else {
            ExitCode::FAILURE
        };
    }

    print_human(&report, &paths);
    if report.status == "ok" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn blocked_not_ready(
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

fn list_sessions(
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

fn export_session(store: &FsChatSessionStore, id: &str, json: bool) -> ExitCode {
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

fn print_human(report: &cynic_inference::ChatTurnReport, paths: &PrincipalPaths) {
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

fn emit(value: &impl Serialize) -> ExitCode {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_not_ready_report_is_fail_closed() {
        let store =
            match FsChatSessionStore::new(std::env::temp_dir().join("cynic-chat-test-store")) {
                Ok(s) => Arc::new(s),
                Err(_) => return, // skip if temp unavailable
            };
        let runtime = ChatRuntime {
            store,
            service: None,
            ready: false,
            use_mock: false,
            endpoint: None,
            model: None,
            source: "none".into(),
            detail: "no endpoint".into(),
            session_root: "/tmp/x".into(),
        };
        let paths = PrincipalPaths::from_home(
            cynic_inference::PathAuthority::RuntimePrincipal,
            "cynic".into(),
            std::path::PathBuf::from("/var/lib/cynic"),
            vec![],
        );
        let r = blocked_not_ready(&runtime, &paths);
        assert_eq!(r.status, "blocked");
        assert!(!r.ready);
        assert!(r.content.is_none());
        assert!(r.blocked_reason.is_some());
    }
}
