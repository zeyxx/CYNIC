use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc;

use super::super::actions::{TuiAction, TuiEffect};
use super::helpers::*;
use super::types::{App, ChatLine, Focus, TuiTaskEvent, TuiWindow};
use crate::chat_surface::{ChatRuntime, resolve_selector_to_alias};
use cynic_inference::{
    ChatEvent, ChatEventKind, ChatService, ChatSessionStore, LocalRuntimeStatus, PrincipalPaths,
};

impl App {
    pub(super) fn handle_slash(&mut self, cmd: &str, rt: &tokio::runtime::Runtime) -> bool {
        let c = cmd.trim();
        match c {
            "/quit" | "/q" | "/exit" => {
                self.should_quit = true;
                true
            }
            "/new" | "/clear" => {
                self.new_session();
                true
            }
            "/help" | "/?" => {
                self.help_open = !self.help_open;
                true
            }
            "/status" => {
                self.lines.push(ChatLine {
                    role: "system".into(),
                    text: format!(
                        "source={} endpoint={} model={} principal={} session={} root={}",
                        self.runtime.source,
                        self.runtime.endpoint.as_deref().unwrap_or("-"),
                        self.runtime.model.as_deref().unwrap_or("-"),
                        self.paths.runtime_user,
                        self.session_id,
                        self.runtime.session_root
                    ),
                });
                let tail = chat_tui_log_tail_lines(CHAT_TUI_LOG_TAIL_STATUS_DEFAULT);
                if let Some(lines) = local_runtime_status_tail(&self.runtime, tail) {
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: format!("runtime log tail (latest {}):", lines.len()),
                    });
                    for line in lines {
                        self.lines.push(ChatLine {
                            role: "system".into(),
                            text: format!("  {line}"),
                        });
                    }
                }
                true
            }
            _ if let Some(value) = c.strip_prefix("/tail") => {
                let value = value.trim();
                match parse_tail_lines_from_command(value, CHAT_TUI_LOG_TAIL_STATUS_DEFAULT) {
                    Ok(tail_lines) => {
                        self.task_window = TuiWindow::AgentLog;
                        self.push_runtime_status_tail_to_agent_log(tail_lines);
                    }
                    Err(error) => {
                        self.error = Some(error);
                    }
                }
                true
            }
            "/model" | "/models" => {
                self.open_model_picker();
                true
            }
            _ if let Some(selector) = c.strip_prefix("/model ") => {
                self.apply_model_with_selector(selector, rt);
                true
            }
            _ if let Some(value) = c.strip_prefix("/context") => {
                let value = value.trim();
                if value.is_empty() {
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: format!(
                            "context = {} (0 = full history)",
                            self.runtime
                                .context_turns
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "off".into()),
                        ),
                    });
                    true
                } else {
                    match value.trim().parse::<usize>() {
                        Ok(v) => {
                            self.runtime.context_turns = if v == 0 { None } else { Some(v) };
                            self.runtime.context_source = "tui command".into();
                            if self.runtime.service.is_some() {
                                if let Ok(rebound) = crate::chat_surface::rebind_chat_service(
                                    Arc::clone(&self.store),
                                    self.runtime
                                        .endpoint
                                        .as_deref()
                                        .unwrap_or("http://127.0.0.1:8080/v1"),
                                    self.runtime.model.as_deref().unwrap_or("default"),
                                    None,
                                    self.runtime.context_turns,
                                    self.runtime.max_tokens,
                                    self.runtime.temperature,
                                    self.runtime.top_p,
                                    &self.runtime.reasoning_profile,
                                ) {
                                    self.service = rebound;
                                }
                            }
                            self.runtime.service = Some(self.service.clone());
                            self.status =
                                format_status(&self.runtime, &self.paths, &self.session_id, false);
                            self.lines.push(ChatLine {
                                role: "system".into(),
                                text: format!("context window set to {}", v),
                            });
                        }
                        Err(_) => {
                            self.error = Some(
                                "/context expects integer message-count (0=full history)".into(),
                            );
                        }
                    }
                    true
                }
            }
            _ if let Some(value) = c.strip_prefix("/temp") => {
                if value.trim().is_empty() {
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: format!(
                            "temperature = {}",
                            self.runtime
                                .temperature
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "auto".into())
                        ),
                    });
                    return true;
                }
                match value.trim().parse::<f32>() {
                    Ok(v) => {
                        if !(0.0..=2.0).contains(&v) {
                            self.error = Some("/temp outside [0,2]".into());
                        } else {
                            self.runtime.temperature = Some(v);
                            self.runtime.context_source = "tui command".into();
                            if self.runtime.service.is_some() {
                                if let Ok(rebound) = crate::chat_surface::rebind_chat_service(
                                    Arc::clone(&self.store),
                                    self.runtime
                                        .endpoint
                                        .as_deref()
                                        .unwrap_or("http://127.0.0.1:8080/v1"),
                                    self.runtime.model.as_deref().unwrap_or("default"),
                                    None,
                                    self.runtime.context_turns,
                                    self.runtime.max_tokens,
                                    Some(v),
                                    self.runtime.top_p,
                                    &self.runtime.reasoning_profile,
                                ) {
                                    self.service = rebound;
                                }
                            }
                            self.runtime.service = Some(self.service.clone());
                            self.lines.push(ChatLine {
                                role: "system".into(),
                                text: format!("temperature set to {v}"),
                            });
                            self.status =
                                format_status(&self.runtime, &self.paths, &self.session_id, false);
                        }
                    }
                    Err(_) => self.error = Some("/temp expects a decimal value".into()),
                }
                true
            }
            _ if let Some(value) = c.strip_prefix("/top_p") => {
                if value.trim().is_empty() {
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: format!(
                            "top_p = {}",
                            self.runtime
                                .top_p
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "auto".into())
                        ),
                    });
                    return true;
                }
                match value.trim().parse::<f32>() {
                    Ok(v) => {
                        if !(0.0..=1.0).contains(&v) {
                            self.error = Some("/top_p outside [0,1]".into());
                        } else {
                            self.runtime.top_p = Some(v);
                            self.runtime.context_source = "tui command".into();
                            if self.runtime.service.is_some() {
                                if let Ok(rebound) = crate::chat_surface::rebind_chat_service(
                                    Arc::clone(&self.store),
                                    self.runtime
                                        .endpoint
                                        .as_deref()
                                        .unwrap_or("http://127.0.0.1:8080/v1"),
                                    self.runtime.model.as_deref().unwrap_or("default"),
                                    None,
                                    self.runtime.context_turns,
                                    self.runtime.max_tokens,
                                    self.runtime.temperature,
                                    Some(v),
                                    &self.runtime.reasoning_profile,
                                ) {
                                    self.service = rebound;
                                }
                            }
                            self.runtime.service = Some(self.service.clone());
                            self.lines.push(ChatLine {
                                role: "system".into(),
                                text: format!("top_p set to {v}"),
                            });
                            self.status =
                                format_status(&self.runtime, &self.paths, &self.session_id, false);
                        }
                    }
                    Err(_) => self.error = Some("/top_p expects a decimal value".into()),
                }
                true
            }
            _ if let Some(value) = c.strip_prefix("/max_tokens") => {
                if value.trim().is_empty() {
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: format!(
                            "max_tokens = {}",
                            self.runtime
                                .max_tokens
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "auto".into())
                        ),
                    });
                    return true;
                }
                match value.trim().parse::<u32>() {
                    Ok(v) => {
                        self.runtime.max_tokens = Some(v);
                        self.runtime.context_source = "tui command".into();
                        if self.runtime.service.is_some() {
                            if let Ok(rebound) = crate::chat_surface::rebind_chat_service(
                                Arc::clone(&self.store),
                                self.runtime
                                    .endpoint
                                    .as_deref()
                                    .unwrap_or("http://127.0.0.1:8080/v1"),
                                self.runtime.model.as_deref().unwrap_or("default"),
                                None,
                                self.runtime.context_turns,
                                Some(v),
                                self.runtime.temperature,
                                self.runtime.top_p,
                                &self.runtime.reasoning_profile,
                            ) {
                                self.service = rebound;
                            }
                        }
                        self.runtime.service = Some(self.service.clone());
                        self.lines.push(ChatLine {
                            role: "system".into(),
                            text: format!("max_tokens set to {v}"),
                        });
                        self.status =
                            format_status(&self.runtime, &self.paths, &self.session_id, false);
                    }
                    Err(_) => self.error = Some("/max_tokens expects integer".into()),
                }
                true
            }
            _ if let Some(profile) = c.strip_prefix("/profile") => {
                let profile = profile.trim();
                if profile.is_empty() {
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: format!("active profile = {}", self.runtime.reasoning_profile),
                    });
                    true
                } else {
                    match apply_reasoning_profile(
                        profile,
                        self.runtime.context_turns,
                        self.runtime.max_tokens,
                        self.runtime.temperature,
                        self.runtime.top_p,
                    ) {
                        Ok((context_turns, max_tokens, temperature, top_p)) => {
                            self.runtime.context_turns = context_turns;
                            self.runtime.max_tokens = max_tokens;
                            self.runtime.temperature = temperature;
                            self.runtime.top_p = top_p;
                            self.runtime.reasoning_profile = profile.to_string();
                            self.runtime.context_source = "tui command".into();
                            if self.runtime.service.is_some() {
                                if let Ok(rebound) = crate::chat_surface::rebind_chat_service(
                                    Arc::clone(&self.store),
                                    self.runtime
                                        .endpoint
                                        .as_deref()
                                        .unwrap_or("http://127.0.0.1:8080/v1"),
                                    self.runtime.model.as_deref().unwrap_or("default"),
                                    None,
                                    self.runtime.context_turns,
                                    self.runtime.max_tokens,
                                    self.runtime.temperature,
                                    self.runtime.top_p,
                                    profile,
                                ) {
                                    self.service = rebound;
                                }
                            }
                            self.runtime.service = Some(self.service.clone());
                            self.lines.push(ChatLine {
                                role: "system".into(),
                                text: format!(
                                    "profile '{profile}' applied: context={:?}, max_tokens={:?}, temp={:?}, top_p={:?}",
                                    self.runtime.context_turns,
                                    self.runtime.max_tokens,
                                    self.runtime.temperature,
                                    self.runtime.top_p
                                ),
                            });
                            self.status =
                                format_status(&self.runtime, &self.paths, &self.session_id, false);
                        }
                        Err(msg) => self.error = Some(msg),
                    }
                    true
                }
            }
            "/sessions" => {
                match self.store.list_sessions() {
                    Ok(list) => {
                        if list.is_empty() {
                            self.lines.push(ChatLine {
                                role: "system".into(),
                                text: "(no sessions)".into(),
                            });
                        }
                        for s in list.into_iter().take(12) {
                            self.lines.push(ChatLine {
                                role: "system".into(),
                                text: format!(
                                    "{}  events={}  {}",
                                    s.id,
                                    s.event_count,
                                    s.model.as_deref().unwrap_or("-")
                                ),
                            });
                        }
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
                true
            }
            _ if c.starts_with('/') => {
                self.lines.push(ChatLine {
                    role: "system".into(),
                    text: format!("unknown command {c} — try /help"),
                });
                true
            }
            _ => false,
        }
    }
}
