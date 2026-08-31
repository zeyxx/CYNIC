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
    pub(super) fn new(runtime: ChatRuntime, paths: PrincipalPaths) -> Result<Self, String> {
        let store = Arc::clone(&runtime.store);
        let service = runtime
            .service
            .clone()
            .ok_or_else(|| "chat service not ready".to_string())?;
        let session_id = format!("tui-{}", now_ms());
        store
            .create_session(
                &session_id,
                Some("tui"),
                runtime.model.as_deref(),
                runtime.endpoint.as_deref(),
            )
            .map_err(|e| e.to_string())?;

        let status = format_status(&runtime, &paths, &session_id, false);
        let (task_tx, task_rx) = mpsc::channel();

        Ok(Self {
            paths,
            runtime,
            store,
            service,
            session_id,
            lines: vec![
                ChatLine {
                    role: "system".into(),
                    text: "CYNIC chat · config via inference SSOT + synapse DNS".into(),
                },
                ChatLine {
                    role: "system".into(),
                    text: "Tab focus · Enter send · /model [alias|remote-id|path] · /context|/temp|/top_p|/max_tokens|/profile · /status · /help · Esc quit".into(),
                },
            ],
            input: String::new(),
            cursor: 0,
            scroll: 0,
            focus: Focus::Prompt,
            status,
            busy: false,
            should_quit: false,
            error: None,
            help_open: false,
            model_picker_open: false,
            model_picker_items: Vec::new(),
            model_picker_idx: 0,
            task_tx,
            task_rx,
            task_window: TuiWindow::Chat,
            agent_log: VecDeque::new(),
        })
    }

    pub(crate) fn dispatch_action(
        &mut self,
        action: TuiAction,
        rt: &tokio::runtime::Runtime,
        stream: bool,
    ) -> Vec<TuiEffect> {
        let mut effects = Vec::new();

        match action {
            TuiAction::Quit => {
                self.should_quit = true;
            }
            TuiAction::ClearOrQuit => {
                if self.input.is_empty() {
                    self.should_quit = true;
                } else {
                    self.input.clear();
                    self.cursor = 0;
                }
            }
            TuiAction::NewSession => {
                self.new_session();
            }
            TuiAction::ToggleHelp => {
                self.help_open = !self.help_open;
            }
            TuiAction::ToggleTaskWindow => {
                self.task_window = match self.task_window {
                    super::state::TuiWindow::Chat => super::state::TuiWindow::AgentLog,
                    super::state::TuiWindow::AgentLog => super::state::TuiWindow::Chat,
                };
            }
            TuiAction::OpenModelPicker => {
                self.open_model_picker();
            }
            TuiAction::FocusPrompt => self.focus = Focus::Prompt,
            TuiAction::FocusScrollback => self.focus = Focus::Scrollback,
            TuiAction::ScrollScrollbackUp => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            TuiAction::ScrollScrollbackDown => {
                self.scroll = self.scroll.saturating_add(1);
            }
            TuiAction::ScrollScrollbackPageUp => {
                self.scroll = self.scroll.saturating_sub(8);
            }
            TuiAction::ScrollScrollbackPageDown => {
                self.scroll = self.scroll.saturating_add(8);
            }
            TuiAction::ScrollScrollbackTop => {
                self.scroll = 0;
            }
            TuiAction::ScrollScrollbackBottom => {
                self.scroll = self.lines.len().saturating_sub(1) as u16;
            }
            TuiAction::PromptInsertChar(c) => {
                self.insert_char(c);
            }
            TuiAction::PromptBackspace => self.backspace(),
            TuiAction::PromptLeft => self.move_left(),
            TuiAction::PromptRight => self.move_right(),
            TuiAction::PromptHome => {
                self.cursor = 0;
            }
            TuiAction::PromptEnd => {
                self.cursor = self.input.len();
            }
            TuiAction::PromptClear => {
                self.input.clear();
                self.cursor = 0;
            }
            TuiAction::PromptSubmit { prompt } => {
                let trimmed = prompt.trim().to_string();
                self.input.clear();
                self.cursor = 0;

                if trimmed.is_empty() {
                    return Vec::new();
                }

                let is_slash = self.handle_slash(&trimmed, rt);
                if is_slash {
                    return Vec::new();
                }

                effects.push(TuiEffect::SendPrompt {
                    prompt: trimmed,
                    stream,
                });
            }
            TuiAction::ModelPickerClose => {
                self.model_picker_open = false;
            }
            TuiAction::ModelPickerPrev => {
                self.model_picker_idx = self.model_picker_idx.saturating_sub(1);
            }
            TuiAction::ModelPickerNext => {
                if self.model_picker_idx + 1 < self.model_picker_items.len() {
                    self.model_picker_idx += 1;
                }
            }
            TuiAction::ModelPickerSubmit => {
                if let Some(selector) = self.model_picker_items.get(self.model_picker_idx).cloned()
                {
                    effects.push(TuiEffect::ApplyModelSelection { selector });
                }
            }
            TuiAction::PasteText(text) => {
                for ch in text.chars() {
                    if ch != '\n' && ch != '\r' {
                        self.insert_char(ch);
                    }
                }
            }
        }

        effects
    }

    pub(super) fn open_model_picker(&mut self) {
        self.model_picker_items.clear();

        let local_had_profiles = match cynic_inference::ensure_profiles_from_vault() {
            Ok(ensured) => {
                self.model_picker_items = ensured.startable_aliases;
                if let Some(pref) = ensured.preferred_alias {
                    if let Some(i) = self
                        .model_picker_items
                        .iter()
                        .position(|alias| alias == &pref)
                    {
                        self.model_picker_idx = i;
                    }
                }
                !self.model_picker_items.is_empty()
            }
            Err(e) => {
                self.error = Some(format!("model list: {e}"));
                false
            }
        };

        let mut remote = self.runtime.available_models.clone();
        remote.retain(|m| !self.model_picker_items.iter().any(|a| a == m));
        self.model_picker_items.extend(remote);

        self.model_picker_items.sort_unstable();
        self.model_picker_items.dedup();

        if self.model_picker_items.is_empty() {
            self.lines.push(ChatLine {
                role: "system".into(),
                text: "no local startable profile and no discovered remote model id; place GGUF in vault or point inference config/host to a serving model"
                    .into(),
            });
            return;
        }

        let source_tag = if local_had_profiles {
            "local+discovered"
        } else {
            "discovered"
        };
        self.model_picker_open = true;
        self.status = format!(
            "model picker · {} {} · Enter select · Esc cancel",
            self.model_picker_items.len(),
            source_tag
        );
    }

    pub(super) fn apply_model_with_selector(
        &mut self,
        selector: &str,
        rt: &tokio::runtime::Runtime,
    ) {
        self.model_picker_open = false;
        let selector = selector.trim();
        if selector.is_empty() {
            self.open_model_picker();
            return;
        }

        let service = cynic_inference::LocalRuntimeServiceAny::from_env();
        let inventory = match service.list_inventory() {
            Ok(inventory) => inventory,
            Err(e) => {
                self.error = Some(format!("model list: {e}"));
                self.status = format_status(&self.runtime, &self.paths, &self.session_id, false);
                return;
            }
        };

        if let Some(alias) = resolve_selector_to_alias(selector, &inventory) {
            self.start_model_selector_alias(alias);
            return;
        }

        let selector_matches_known_remote = {
            self.runtime
                .available_models
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(selector))
                || self.runtime.endpoint.as_deref().is_some_and(|endpoint| {
                    rt.block_on(cynic_inference::probe_live_model_ids(
                        endpoint,
                        cynic_inference::resolve_chat_api_key(None).as_deref(),
                    ))
                    .into_iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(selector))
                })
        };
        if selector_matches_known_remote {
            self.start_model_selector_remote(selector.to_string());
            return;
        }

        if looks_like_local_selector(selector) {
            let available: Vec<&str> = inventory.models.iter().map(|m| m.alias.as_str()).collect();
            let hint = if available.is_empty() {
                "cynic runtime inference-models --ensure-profiles --json".to_string()
            } else {
                format!(
                    "cynic runtime inference-vault --json (aliases: {})",
                    available.join(", ")
                )
            };
            self.error = Some(format!(
                "local selector `{selector}` not found in vault; {hint}; then cynic models start <alias> --yes"
            ));
            self.status = format_status(&self.runtime, &self.paths, &self.session_id, false);
            return;
        }

        self.error = Some(format!(
            "model `{selector}` not found in local aliases and not discovered on current chat endpoint; run /model to list available local aliases + remote model ids"
        ));
        self.status = format_status(&self.runtime, &self.paths, &self.session_id, false);
    }

    fn start_model_selector_remote(&mut self, model: String) {
        let tx = self.task_tx.clone();
        let store = Arc::clone(&self.store);
        let context_turns = self.runtime.context_turns;
        let max_tokens = self.runtime.max_tokens;
        let temperature = self.runtime.temperature;
        let top_p = self.runtime.top_p;
        let reasoning_profile = self.runtime.reasoning_profile.clone();
        let Some(endpoint) = self.runtime.endpoint.clone() else {
            self.error = Some("no endpoint for remote model bind".into());
            self.status = format_status(&self.runtime, &self.paths, &self.session_id, false);
            return;
        };

        self.busy = true;
        self.error = None;
        self.status = format!("binding remote model {model}…");
        self.push_agent_line(format!("agent: remote bind queued · {model}"));

        std::thread::spawn(move || {
            match crate::chat_surface::rebind_chat_service(
                store,
                &endpoint,
                &model,
                cynic_inference::resolve_chat_api_key(None),
                context_turns,
                max_tokens,
                temperature,
                top_p,
                &reasoning_profile,
            ) {
                Ok(service) => {
                    let _ = tx.send(TuiTaskEvent::ModelRemoteDone {
                        model: model.clone(),
                        endpoint: endpoint.clone(),
                        service,
                    });
                }
                Err(error) => {
                    let _ = tx.send(TuiTaskEvent::TaskError(format!(
                        "remote model bind failed for `{}`: {error}",
                        model
                    )));
                }
            }
        });
    }
}
