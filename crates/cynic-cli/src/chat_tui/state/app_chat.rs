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
