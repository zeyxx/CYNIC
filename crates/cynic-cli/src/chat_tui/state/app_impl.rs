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

    fn start_model_selector_alias(&mut self, alias: String) {
        let tx = self.task_tx.clone();
        let store = Arc::clone(&self.store);
        let context_turns = self.runtime.context_turns;
        let max_tokens = self.runtime.max_tokens;
        let temperature = self.runtime.temperature;
        let top_p = self.runtime.top_p;
        let reasoning_profile = self.runtime.reasoning_profile.clone();

        self.busy = true;
        self.error = None;
        self.status = format!("starting {alias}…");
        self.push_agent_line(format!("agent: local start queued · {alias}"));

        std::thread::spawn(move || {
            if let Err(error) =
                cynic_inference::pin_preferred_alias(&alias, "cynic chat tui /model")
            {
                let _ = tx.send(TuiTaskEvent::TaskError(format!(
                    "pin preferred local alias failed for {alias}: {error}"
                )));
                return;
            }

            let service = cynic_inference::LocalRuntimeServiceAny::from_env();
            let profile_size = service.list_inventory().ok().and_then(|inventory| {
                inventory
                    .models
                    .iter()
                    .find(|p| p.alias == alias)
                    .and_then(|p| p.size_bytes)
            });

            let wait_for_ready =
                !crate::chat_surface::runtime::should_defer_local_readiness_for_chat(profile_size);
            let report = match service.start_with_readiness(&alias, wait_for_ready) {
                Ok(report) => report,
                Err(error) => {
                    let _ = tx.send(TuiTaskEvent::TaskError(format!(
                        "start {alias} failed: {error}"
                    )));
                    return;
                }
            };
            let readiness_deferred = report
                .message
                .as_ref()
                .is_some_and(|message| message.contains("ready=deferred"));
            let report_log_path = report
                .message
                .as_deref()
                .and_then(extract_log_path_from_message);

            if !report.ok {
                let _ = tx.send(TuiTaskEvent::TaskError(
                    report
                        .message
                        .unwrap_or_else(|| format!("start failed for {alias}")),
                ));
                return;
            }

            let inventory = service.list_inventory().ok();
            let profile_endpoint = inventory
                .as_ref()
                .and_then(|inventory| profile_endpoint_for_alias(inventory, &alias));
            let endpoint = report
                .status
                .endpoint
                .or(profile_endpoint)
                .or_else(|| service.status().ok().and_then(|status| status.endpoint))
                .or_else(|| detect_local_runtime_endpoint(&alias));

            let Some(endpoint) = endpoint else {
                let _ = tx.send(TuiTaskEvent::TaskError(format!(
                    "could not resolve endpoint for {alias}"
                )));
                return;
            };

            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(r) => r,
                Err(error) => {
                    let _ = tx.send(TuiTaskEvent::TaskError(format!(
                        "runtime init failed during bind for {alias}: {error}"
                    )));
                    return;
                }
            };

            let bind = match rt.block_on(cynic_inference::bind_preferred_chat_model(
                &alias,
                Some(&endpoint),
                "cynic chat tui /model",
            )) {
                Ok(bind) => bind,
                Err(error) => {
                    let _ = tx.send(TuiTaskEvent::TaskError(format!(
                        "bind_preferred_chat_model failed for {alias}: {error}"
                    )));
                    return;
                }
            };

            let service = match crate::chat_surface::rebind_chat_service(
                store,
                &endpoint,
                &bind.model_id,
                bind.api_key.clone(),
                context_turns,
                max_tokens,
                temperature,
                top_p,
                &reasoning_profile,
            ) {
                Ok(service) => service,
                Err(error) => {
                    let _ = tx.send(TuiTaskEvent::TaskError(format!(
                        "rebind local model failed for {alias}: {error}"
                    )));
                    return;
                }
            };

            let mut detail = format!("local alias pinned {}", bind.alias);
            if readiness_deferred {
                detail.push_str(" (readiness deferred)");
            }
            if let Some(path) = report_log_path.as_deref() {
                detail.push_str(&format!(" log={path}"));
            }

            let _ = tx.send(TuiTaskEvent::ModelAliasDone {
                alias: alias.clone(),
                endpoint: endpoint.clone(),
                model: bind.model_id,
                detail,
                log_path: report_log_path,
                service,
            });
        });
    }

    pub(super) fn start_chat_turn(&mut self, prompt: String, stream: bool) {
        let tx = self.task_tx.clone();
        let service = self.service.clone();
        let session_id = self.session_id.clone();
        self.busy = true;
        self.error = None;
        self.status = format_status(&self.runtime, &self.paths, &self.session_id, true);
        self.push_agent_line("agent: queued prompt turn".into());

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = tx.send(TuiTaskEvent::TaskError(format!(
                        "prompt runtime init failed: {error}"
                    )));
                    return;
                }
            };
            let report = runtime.block_on(service.turn(Some(&session_id), &prompt, stream));
            let _ = tx.send(TuiTaskEvent::ChatDone(report));
        });
    }

    pub(super) fn poll_agent_events(&mut self) {
        while let Ok(event) = self.task_rx.try_recv() {
            match event {
                TuiTaskEvent::ChatDone(report) => {
                    self.busy = false;
                    if report.status != "ok" {
                        self.error = Some(report.blocked_reason.unwrap_or(report.message));
                    }
                    self.push_agent_line(format!(
                        "agent: turn {} ({} events)",
                        report.status, report.events_appended
                    ));
                    self.reload_from_store();
                    self.status =
                        format_status(&self.runtime, &self.paths, &self.session_id, false);
                    self.scroll = self.lines.len().saturating_sub(1) as u16;
                }
                TuiTaskEvent::ModelAliasDone {
                    alias,
                    endpoint,
                    model,
                    detail,
                    log_path,
                    service,
                } => {
                    self.busy = false;
                    self.service = service;
                    self.runtime.endpoint = Some(endpoint.clone());
                    self.runtime.model = Some(model.clone());
                    self.runtime.source = "local".into();
                    self.runtime.detail = detail;
                    self.runtime.service = Some(self.service.clone());
                    if !self.runtime.available_models.iter().any(|m| m == &model) {
                        self.runtime.available_models.push(model.clone());
                    }
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: format!(
                            "alias started: {alias} · live model={model} · endpoint={endpoint}"
                        ),
                    });
                    if let Some(log_path) = log_path.as_deref() {
                        self.lines.push(ChatLine {
                            role: "system".into(),
                            text: format!(
                                "runtime log tail (latest {}):",
                                chat_tui_log_tail_lines(CHAT_TUI_LOG_TAIL_ALIAS_EVENT_DEFAULT)
                            ),
                        });
                        for line in read_log_tail(
                            log_path,
                            chat_tui_log_tail_lines(CHAT_TUI_LOG_TAIL_ALIAS_EVENT_DEFAULT),
                        ) {
                            self.lines.push(ChatLine {
                                role: "system".into(),
                                text: format!("  {line}"),
                            });
                        }
                        self.push_runtime_file_tail_to_agent_log(
                            log_path,
                            chat_tui_log_tail_lines(CHAT_TUI_LOG_TAIL_ALIAS_EVENT_DEFAULT),
                        );
                    }
                    self.push_agent_line(format!(
                        "agent: local model ready · alias={alias} model={model}"
                    ));
                    self.status =
                        format_status(&self.runtime, &self.paths, &self.session_id, false);
                    self.scroll = self.lines.len().saturating_sub(1) as u16;
                }
                TuiTaskEvent::ModelRemoteDone {
                    model,
                    endpoint,
                    service,
                } => {
                    self.busy = false;
                    self.service = service;
                    self.runtime.endpoint = Some(endpoint.clone());
                    self.runtime.model = Some(model.clone());
                    self.runtime.source = "remote".into();
                    self.runtime.detail = format!("remote model selected `{model}`");
                    self.runtime.service = Some(self.service.clone());
                    if !self.runtime.available_models.iter().any(|m| m == &model) {
                        self.runtime.available_models.push(model.clone());
                    }
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: format!("remote model bound: {model} via {endpoint}"),
                    });
                    self.push_agent_line(format!("agent: remote model bound · {model}"));
                    self.status =
                        format_status(&self.runtime, &self.paths, &self.session_id, false);
                    self.scroll = self.lines.len().saturating_sub(1) as u16;
                }
                TuiTaskEvent::TaskError(message) => {
                    self.busy = false;
                    self.error = Some(message);
                    self.status =
                        format_status(&self.runtime, &self.paths, &self.session_id, false);
                }
            }
        }
    }

    fn push_agent_line(&mut self, line: String) {
        self.agent_log.push_front(line);
        while self.agent_log.len() > 120 {
            self.agent_log.pop_back();
        }
    }

    fn push_runtime_file_tail_to_agent_log(&mut self, log_path: &str, tail: usize) {
        let lines = read_log_tail(log_path, tail);
        self.agent_log.clear();
        if lines.is_empty() {
            self.agent_log
                .push_front("runtime log tail unavailable".into());
            return;
        }
        self.agent_log
            .push_front(format!("runtime log tail (latest {}):", lines.len()));
        for line in lines.iter().rev() {
            self.push_agent_line(format!("  {}", line));
        }
    }

    fn push_runtime_status_tail_to_agent_log(&mut self, tail: usize) {
        self.agent_log.clear();
        if let Some(lines) = local_runtime_status_tail(&self.runtime, tail) {
            self.agent_log
                .push_front(format!("runtime log tail (latest {}):", lines.len()));
            for line in lines.iter().rev() {
                self.push_agent_line(format!("  {}", line));
            }
        } else {
            self.agent_log
                .push_front("runtime log tail unavailable".into());
        }
    }

    pub(super) fn reload_from_store(&mut self) {
        match self.store.load_events(&self.session_id) {
            Ok(events) => {
                self.lines = events_to_lines(&events);
                if self.lines.is_empty() {
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: "empty session — type a message".into(),
                    });
                }
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    pub(super) fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub(super) fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.input[..self.cursor]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        let start = self.cursor - prev;
        self.input.replace_range(start..self.cursor, "");
        self.cursor = start;
    }

    pub(super) fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.input[..self.cursor]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        self.cursor -= prev;
    }

    pub(super) fn move_right(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let next = self.input[self.cursor..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        self.cursor += next;
    }

    pub(super) fn new_session(&mut self) {
        let id = format!("tui-{}", now_ms());
        if let Err(e) = self.store.create_session(
            &id,
            Some("tui"),
            self.runtime.model.as_deref(),
            self.runtime.endpoint.as_deref(),
        ) {
            self.error = Some(e.to_string());
            return;
        }
        self.session_id = id;
        self.lines = vec![ChatLine {
            role: "system".into(),
            text: "new session".into(),
        }];
        self.scroll = 0;
        self.status = format_status(&self.runtime, &self.paths, &self.session_id, self.busy);
    }

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

