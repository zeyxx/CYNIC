//! Process-level local inference supervisor (no systemd required).
//!
//! Spawns `llama-server` (or `LLAMA_SERVER_BIN`) under the principal with a
//! pidfile + log under principal state. Suitable for lab, containers without
//! systemd, and build_runner hosts where user bus is awkward.
//!
//! Select with `CYNIC_INFERENCE_SUPERVISOR=process` (aliases: `argv`, `direct`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::adapters::local::vault_inventory::FsLocalModelInventory;
use crate::domain::local_runtime::{
    LocalModelProfile, LocalRuntimeMutationReport, LocalRuntimeState, LocalRuntimeStatus,
};
use crate::domain::principal_paths::PrincipalPaths;
use crate::ports::local_runtime_port::{
    LocalInferenceRuntimePort, LocalModelInventoryPort, LocalRuntimeError,
};

const SUPERVISOR_SURFACE: &str = "process";

#[derive(Debug)]
pub struct ProcessLocalRuntime {
    inventory: FsLocalModelInventory,
    state_dir: PathBuf,
}

impl ProcessLocalRuntime {
    pub fn from_env() -> Self {
        let principal = PrincipalPaths::resolve();
        let state_dir = principal
            .home_dir
            .join(".local/state/cynic/inference/process");
        Self {
            inventory: FsLocalModelInventory::from_env(),
            state_dir,
        }
    }

    pub fn with_inventory(inventory: FsLocalModelInventory) -> Self {
        let mut s = Self::from_env();
        s.inventory = inventory;
        s
    }

    fn ensure_state_dir(&self) -> Result<(), LocalRuntimeError> {
        fs::create_dir_all(&self.state_dir).map_err(|e| {
            LocalRuntimeError::Control(format!("mkdir {}: {e}", self.state_dir.display()))
        })
    }

    fn pid_path(&self, alias: &str) -> PathBuf {
        self.state_dir.join(format!("{alias}.pid"))
    }

    fn log_path(&self, alias: &str) -> PathBuf {
        self.state_dir.join(format!("{alias}.log"))
    }

    fn read_pid(&self, alias: &str) -> Option<u32> {
        let raw = fs::read_to_string(self.pid_path(alias)).ok()?;
        let pid: u32 = raw.trim().parse().ok()?;
        if pid == 0 {
            return None;
        }
        if Path::new(&format!("/proc/{pid}")).is_dir() {
            Some(pid)
        } else {
            let _ = fs::remove_file(self.pid_path(alias));
            None
        }
    }

    fn write_pid(&self, alias: &str, pid: u32) -> Result<(), LocalRuntimeError> {
        self.ensure_state_dir()?;
        fs::write(self.pid_path(alias), format!("{pid}\n")).map_err(|e| {
            LocalRuntimeError::Control(format!("write pid {}: {e}", self.pid_path(alias).display()))
        })
    }

    fn kill_pid(pid: u32) {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
        thread::sleep(Duration::from_millis(400));
        if Path::new(&format!("/proc/{pid}")).is_dir() {
            let _ = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status();
        }
    }

    fn list_running_aliases(&self) -> Vec<String> {
        let Ok(rd) = fs::read_dir(&self.state_dir) else {
            return vec![];
        };
        let mut out = Vec::new();
        for e in rd.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if let Some(alias) = name.strip_suffix(".pid")
                && self.read_pid(alias).is_some()
            {
                out.push(alias.to_string());
            }
        }
        out
    }

    fn status_for_alias(&self, alias: &str, profile: &LocalModelProfile) -> LocalRuntimeStatus {
        let log_path = self.log_path(alias).display().to_string();
        match self.read_pid(alias) {
            Some(pid) => LocalRuntimeStatus {
                schema: LocalRuntimeStatus::SCHEMA,
                surface: SUPERVISOR_SURFACE,
                alias: Some(alias.to_string()),
                state: LocalRuntimeState::Active,
                unit: Some(format!("process:{alias}")),
                main_pid: Some(pid),
                memory_current_bytes: None,
                endpoint: Some(format!("http://{}:{}/v1", profile.host, profile.port)),
                listen_host: Some(profile.host.clone()),
                listen_port: Some(profile.port),
                message: Some(format!("log={log_path}")),
            },
            None => LocalRuntimeStatus::inactive(format!("no process for {alias}; log={log_path}")),
        }
    }

    fn build_argv(
        profile: &LocalModelProfile,
    ) -> Result<(PathBuf, Vec<String>), LocalRuntimeError> {
        // Prefer probe-generated runtime env (principal layout), then static profile.
        let inventory = FsLocalModelInventory::from_env();
        let full = inventory.get_profile(&profile.alias)?;
        let layout = crate::domain::model_layout::ModelLayout::from_env();
        let runtime_env = layout
            .profiles_dir
            .join("runtime")
            .join(format!("{}.env", profile.alias));
        let static_env = layout.profiles_dir.join(format!("{}.env", profile.alias));
        let env_path = if runtime_env.is_file() {
            runtime_env
        } else {
            static_env
        };
        let raw = fs::read_to_string(&env_path).map_err(|e| {
            LocalRuntimeError::Inventory(format!("read {}: {e}", env_path.display()))
        })?;
        let mut bin = PathBuf::from("/usr/local/bin/llama-server");
        let mut extra = String::new();
        let mut model_path = full.model_path.clone();
        let mut host = full.host.clone();
        let mut port = full.port.to_string();
        let mut ctx = full.ctx_size.to_string();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "LLAMA_SERVER_BIN" => bin = PathBuf::from(v.trim()),
                    "LLAMA_EXTRA_ARGS" => extra = v.trim().to_string(),
                    "LLAMA_MODEL_PATH" => model_path = v.trim().to_string(),
                    "LLAMA_HOST" => host = v.trim().to_string(),
                    "LLAMA_PORT" => port = v.trim().to_string(),
                    "LLAMA_CTX_SIZE" => ctx = v.trim().to_string(),
                    _ => {}
                }
            }
        }
        let mut argv = vec![
            "-m".into(),
            model_path,
            "--host".into(),
            host,
            "--port".into(),
            port,
            "--ctx-size".into(),
            ctx,
        ];
        if !extra.is_empty() {
            for tok in extra.split_whitespace() {
                argv.push(tok.to_string());
            }
        }
        // API key file convention when present (principal membrane).
        let principal = PrincipalPaths::resolve();
        for key_path in [
            principal.config_dir.join("llama-api-key"),
            PathBuf::from("/etc/cynic/llama-api-key"),
        ] {
            if key_path.is_file() {
                argv.push("--api-key-file".into());
                argv.push(key_path.display().to_string());
                break;
            }
        }
        let _ = profile;
        Ok((bin, argv))
    }

    fn expected_model_ids(profile: &LocalModelProfile) -> Vec<String> {
        let mut ids = Vec::new();
        ids.push(profile.alias.clone());

        let model_path = Path::new(&profile.model_path);
        if let Some(name) = model_path.file_name().and_then(|p| p.to_str()) {
            let stem = Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(name);
            ids.push(name.to_string());
            ids.push(stem.to_string());
        }

        ids.into_iter()
            .map(|s| s.to_lowercase())
            .collect::<std::collections::HashSet<_>>() // dedupe + stable dedupe later
            .into_iter()
            .collect()
    }

    fn parse_model_ids(body: &str) -> Vec<String> {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
            return Vec::new();
        };

        let mut ids = Vec::new();
        if let Some(models) = v.get("data").and_then(|d| d.as_array()) {
            for m in models {
                if let Some(id) = m
                    .get("id")
                    .and_then(|x| x.as_str())
                    .filter(|x| !x.is_empty())
                {
                    ids.push(id.to_string());
                }
            }
        }

        if let Some(models) = v.get("models").and_then(|d| d.as_array()) {
            for m in models {
                let Some(id) = m
                    .get("id")
                    .or_else(|| m.get("name"))
                    .and_then(|x| x.as_str())
                    .filter(|x| !x.is_empty())
                else {
                    continue;
                };
                ids.push(id.to_string());
            }
        }

        ids.sort();
        ids.dedup();
        ids
    }

    fn model_matches_target(loaded: &[String], expected: &[String]) -> bool {
        if loaded.is_empty() || expected.is_empty() {
            return false;
        }
        let loaded_lc: Vec<String> = loaded.iter().map(|s| s.to_lowercase()).collect();
        for e in expected {
            let e_lc = e.to_lowercase();
            for l in &loaded_lc {
                if l == &e_lc || l.contains(&e_lc) || e_lc.contains(l) {
                    return true;
                }
            }
        }
        false
    }

    fn fetch_models_for_wait(url: &str) -> (String, Vec<String>) {
        let out = Command::new("curl")
            .args([
                "-sS",
                "-m",
                "2",
                "-o",
                "-",
                "-H",
                "Accept: application/json",
            ])
            .arg(url)
            .output()
            .ok();

        let body = out
            .as_ref()
            .and_then(|o| String::from_utf8(o.stdout.clone()).ok())
            .unwrap_or_default();

        if out.is_none() {
            return ("".to_string(), Vec::new());
        }

        let code = Command::new("curl")
            .args(["-sS", "-m", "2", "-o", "/dev/null", "-w", "%{http_code}"])
            .arg(url)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_else(|| "".into())
            .trim()
            .to_string();

        let ids = Self::parse_model_ids(&body);
        (code, ids)
    }

    fn estimate_model_ready_timeout(profile: &LocalModelProfile) -> Duration {
        let size_gb = profile.size_bytes.unwrap_or(0) as f64 / (1024.0 * 1024.0 * 1024.0);
        let extra = (size_gb * 45.0).round() as u64;
        let timeout_secs = 180u64 + extra;
        Duration::from_secs(timeout_secs.min(1200))
    }
    fn wait_http_ready(&self, alias: &str, profile: &LocalModelProfile, timeout: Duration) -> bool {
        let url = format!("http://{}:{}/v1/models", profile.host, profile.port);
        let start = std::time::Instant::now();
        let expected = Self::expected_model_ids(profile);
        let mut last_seen = Vec::<String>::new();
        let mut wait = crate::progress::WaitProgress::new(
            format!(
                "model HTTP {url} (load can take ~{}s for large GGUF)",
                timeout.as_secs()
            ),
            5,
        );
        while start.elapsed() < timeout {
            if self.read_pid(alias).is_none() {
                crate::progress::progress(
                    "dead",
                    format!(
                        "llama-server exited early — see log {}",
                        self.log_path(alias).display()
                    ),
                );
                if let Ok(tail) = fs::read_to_string(self.log_path(alias)) {
                    let last: String = tail
                        .lines()
                        .rev()
                        .take(8)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join(" | ");
                    if !last.is_empty() {
                        crate::progress::progress(
                            "log",
                            last.chars().take(500).collect::<String>(),
                        );
                    }
                }
                wait.done(false);
                return false;
            }

            let (code, models) = Self::fetch_models_for_wait(&url);
            if !models.is_empty() {
                last_seen = models.clone();
            }

            let code_ok = matches!(code.as_str(), "200" | "401");
            if code_ok {
                if Self::model_matches_target(&models, &expected) {
                    wait.done(true);
                    return true;
                }
                if models.is_empty() {
                    crate::progress::progress(
                        "loading",
                        format!("HTTP {code} but /v1/models returned no model id yet; waiting"),
                    );
                } else {
                    let preview = last_seen.join(", ");
                    let shown = preview.chars().take(180).collect::<String>();
                    crate::progress::progress(
                        "mismatch",
                        format!("HTTP {code} but loaded models={shown}"),
                    );
                }
            }

            wait.tick();
            thread::sleep(Duration::from_millis(500));
        }
        wait.done(false);
        false
    }
}

impl LocalInferenceRuntimePort for ProcessLocalRuntime {
    fn status(&self) -> Result<LocalRuntimeStatus, LocalRuntimeError> {
        let running = self.list_running_aliases();
        if running.is_empty() {
            return Ok(LocalRuntimeStatus {
                schema: LocalRuntimeStatus::SCHEMA,
                surface: SUPERVISOR_SURFACE,
                alias: None,
                state: LocalRuntimeState::Inactive,
                unit: None,
                main_pid: None,
                memory_current_bytes: None,
                endpoint: None,
                listen_host: None,
                listen_port: None,
                message: Some("no process-supervised llama-server instance running".into()),
            });
        }
        let alias = &running[0];
        let profile = self.inventory.get_profile(alias)?;
        Ok(self.status_for_alias(alias, &profile))
    }

    fn start(&self, alias: &str) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.start_with_readiness(alias, true)
    }

    fn start_with_readiness(
        &self,
        alias: &str,
        wait_for_ready: bool,
    ) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        crate::progress::progress("start", format!("alias={alias} supervisor=process"));
        let profile = self.inventory.get_profile(alias)?;
        if !profile.model_exists {
            return Err(LocalRuntimeError::Rejected(format!(
                "model file missing for profile {alias}: {}",
                profile.model_path
            )));
        }
        if !profile.loopback_only {
            return Err(LocalRuntimeError::Rejected(format!(
                "refusing to start {alias}: host {} is not loopback",
                profile.host
            )));
        }

        crate::progress::progress("preflight", format!("gguf {}", profile.model_path));
        let model_path = Path::new(&profile.model_path);
        let gguf_meta = crate::domain::gguf_meta::preflight_gguf_for_engine(
            model_path,
            crate::domain::gguf_meta::default_supported_arches_for_llama_b9444(),
        )
        .map_err(|e| LocalRuntimeError::Rejected(e.to_string()))?;
        crate::progress::progress(
            "preflight",
            format!("arch={}", gguf_meta.architecture.as_deref().unwrap_or("?")),
        );

        // Probe plan → principal runtime env (same materialization membrane as systemd).
        crate::progress::progress("plan", "probe hardware + materialize runtime env");
        let planner = crate::application::runtime_planner::RuntimePlanner::from_env();
        let plan = planner.plan_for_alias(alias)?;
        let materialize = planner.materialize(&plan)?;
        crate::progress::progress("plan", format!("env={}", materialize.runtime_env));

        // Stop others on same port policy (one local engine)
        for other in self.list_running_aliases() {
            if other != alias {
                crate::progress::progress("stop", format!("previous instance {other}"));
                if let Some(pid) = self.read_pid(&other) {
                    Self::kill_pid(pid);
                    let _ = fs::remove_file(self.pid_path(&other));
                }
            }
        }
        if let Some(pid) = self.read_pid(alias) {
            Self::kill_pid(pid);
            let _ = fs::remove_file(self.pid_path(alias));
        }

        let (bin, argv) = Self::build_argv(&profile)?;
        self.ensure_state_dir()?;
        let log = fs::File::create(self.log_path(alias)).map_err(|e| {
            LocalRuntimeError::Control(format!("log {}: {e}", self.log_path(alias).display()))
        })?;
        let log_err = log
            .try_clone()
            .map_err(|e| LocalRuntimeError::Control(format!("log clone: {e}")))?;

        // llama-server loads ggml plugins relative to CWD (see systemd unit WorkingDirectory).
        let plugin_cwd = PathBuf::from("/usr/local/lib/llama");
        let mut cmd = Command::new(&bin);
        cmd.args(&argv)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err));
        if plugin_cwd.is_dir() {
            cmd.current_dir(&plugin_cwd);
        }
        crate::progress::progress(
            "spawn",
            format!("{} (log {})", bin.display(), self.log_path(alias).display()),
        );
        let child = cmd
            .spawn()
            .map_err(|e| LocalRuntimeError::Control(format!("spawn {}: {e}", bin.display())))?;
        let pid = child.id();
        self.write_pid(alias, pid)?;
        crate::progress::progress("spawn", format!("pid={pid}"));
        // detach: drop child without wait — process stays
        std::mem::drop(child);

        let ready_timeout = Self::estimate_model_ready_timeout(&profile);
        let ready = if wait_for_ready {
            crate::progress::progress(
                "load",
                "waiting for llama-server to finish loading weights into memory/VRAM",
            );
            self.wait_http_ready(alias, &profile, ready_timeout)
        } else {
            crate::progress::progress(
                "ready",
                "readiness probe skipped by CLI (inference start --no-wait)",
            );
            true
        };
        let st = self.status_for_alias(alias, &profile);
        let ok = ready && st.state == LocalRuntimeState::Active;
        let log_hint = self.log_path(alias).display().to_string();
        if !ok {
            if let Some(p) = self.read_pid(alias) {
                Self::kill_pid(p);
            }
            let _ = fs::remove_file(self.pid_path(alias));
        }
        let st = if ok {
            st
        } else {
            LocalRuntimeStatus::inactive(format!(
                "process start failed or model not ready (log {log_hint})"
            ))
        };

        let fail_tail = if ok {
            String::new()
        } else {
            fs::read_to_string(self.log_path(alias))
                .ok()
                .map(|t| {
                    let lines: Vec<&str> = t.lines().rev().take(5).collect();
                    let joined = lines.into_iter().rev().collect::<Vec<_>>().join(" | ");
                    if joined.is_empty() {
                        String::new()
                    } else {
                        format!("; log_tail={joined}")
                    }
                })
                .unwrap_or_default()
        };

        let ready_label = if wait_for_ready { "true" } else { "deferred" };
        Ok(LocalRuntimeMutationReport {
            schema: LocalRuntimeMutationReport::SCHEMA,
            surface: SUPERVISOR_SURFACE,
            operation: "start".into(),
            alias: alias.to_string(),
            ok,
            unit: format!("process:{alias}"),
            status: st,
            message: Some(format!(
                "process supervisor: arch={} pid={} log={} env={} ready={}{}",
                gguf_meta.architecture.as_deref().unwrap_or("?"),
                pid,
                log_hint,
                materialize.runtime_env,
                ready_label,
                fail_tail,
            )),
        })
    }

    fn stop(&self, alias: Option<&str>) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        let alias = match alias {
            Some(a) => a.to_string(),
            None => self
                .list_running_aliases()
                .into_iter()
                .next()
                .ok_or_else(|| {
                    LocalRuntimeError::Rejected("no running process instance to stop".into())
                })?,
        };
        let profile = self.inventory.get_profile(&alias).ok();
        if let Some(pid) = self.read_pid(&alias) {
            Self::kill_pid(pid);
        }
        let _ = fs::remove_file(self.pid_path(&alias));
        Ok(LocalRuntimeMutationReport {
            schema: LocalRuntimeMutationReport::SCHEMA,
            surface: SUPERVISOR_SURFACE,
            operation: "stop".into(),
            alias: alias.clone(),
            ok: true,
            unit: format!("process:{alias}"),
            status: profile
                .as_ref()
                .map(|p| self.status_for_alias(&alias, p))
                .unwrap_or_else(|| LocalRuntimeStatus::inactive("stopped")),
            message: Some("process stopped".into()),
        })
    }
}

/// Which process supervisor backend to use for native local inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceSupervisorKind {
    SystemdUser,
    Process,
    Docker,
}

impl InferenceSupervisorKind {
    pub fn from_env() -> Self {
        let raw = std::env::var("CYNIC_INFERENCE_SUPERVISOR")
            .unwrap_or_default()
            .to_lowercase();
        match raw.as_str() {
            "process" | "argv" | "direct" | "pidfile" => Self::Process,
            "systemd" | "systemd-user" | "user" => Self::SystemdUser,
            "docker" | "container" | "podman" => Self::Docker,
            // Default durable path: process supervisor (principal pidfile).
            // Opt into systemd explicitly when linger + user bus is intentional.
            "" => Self::Process,
            _ => Self::Process,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SystemdUser => "systemd-user",
            Self::Process => "process",
            Self::Docker => "docker",
        }
    }
}
