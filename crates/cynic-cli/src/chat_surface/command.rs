use std::process::ExitCode;
use std::sync::Arc;

use cynic_inference::{
    BuiltinToolInvoker, ChatCompletionClient, ChatService, ChatSessionStore, MockChatClient,
    ToolCallRequest,
};

use crate::chat_tui;

use super::runtime::chat_tool_policy;
use super::{
    ChatArgs, blocked_not_ready, build_chat_runtime, emit_chat_json, export_session, list_sessions,
    print_human,
};

pub(crate) fn execute_chat(args: &ChatArgs) -> ExitCode {
    let paths = cynic_inference::PrincipalPaths::resolve();

    // Interactive TUI: default when no prompt and not list/export/json/no-tui
    let want_tui = args.prompt.is_none()
        && !args.list_sessions
        && args.export.is_none()
        && !args.json
        && !args.no_tui;

    if want_tui {
        return chat_tui::run_chat_tui(args.mock, args.ods, args.stream, args.model.as_deref());
    }

    // execute_chat runs inside the CLI tokio runtime: never nest block_on.
    // Blocking turns run on a helper thread against the current handle.
    let handle = tokio::runtime::Handle::current();
    let build_args = (args.mock, args.ods, args.model.clone());
    let setup_handle = handle.clone();
    let runtime = match std::thread::spawn(move || {
        setup_handle.block_on(build_chat_runtime(
            build_args.0,
            build_args.1,
            build_args.2.as_deref(),
            false,
        ))
    })
    .join()
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            eprintln!("chat: {e}");
            return ExitCode::FAILURE;
        }
        Err(_) => {
            eprintln!("chat setup thread failed");
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
            let _ = emit_chat_json(&serde_json::json!({
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
            eprintln!(
                "  cynic chat \"prompt\" [--mock|--ods] [--model <alias|id>] [--json] [--resume ID]"
            );
            eprintln!("  cynic chat --list-sessions [--json]");
            eprintln!("  cynic chat --export <session-id>");
        }
        return ExitCode::FAILURE;
    };

    if !runtime.ready {
        let report = blocked_not_ready(&runtime, &paths);
        if args.json {
            let _ = emit_chat_json(&report);
        } else {
            print_human(&report, &paths);
        }
        return ExitCode::FAILURE;
    }

    let Some(service) = runtime.service else {
        eprintln!("chat service missing");
        return ExitCode::FAILURE;
    };

    // Rebuild mock client per-prompt when mock so tool keyword works
    let service = if runtime.use_mock {
        let client: Arc<dyn ChatCompletionClient> = if prompt.to_lowercase().contains("tool") {
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
            chat_tool_policy().0,
            cynic_inference::ChatServiceConfig::new(
                runtime.endpoint.clone(),
                runtime.model.clone(),
                4,
                true,
            ),
        )
    } else {
        service
    };

    let sid = args.resume.clone().or(args.session_id.clone());
    let turn_args = (sid, prompt.clone(), args.stream);
    let turn_handle = handle.clone();
    let report = match std::thread::spawn(move || {
        turn_handle.block_on(service.turn(turn_args.0.as_deref(), &turn_args.1, turn_args.2))
    })
    .join()
    {
        Ok(report) => report,
        Err(_) => {
            eprintln!("chat turn thread failed");
            return ExitCode::FAILURE;
        }
    };

    if args.json {
        let code = emit_chat_json(&report);
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
