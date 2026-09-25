//! systemd --user adapter for native on-demand llama-server instances.
//!
//! Unit template: `llama-server@<alias>.service`
//! EnvironmentFile: `%h/.config/cynic/models/<alias>.env` (declared by unit).
//!
//! Uses argv-based `systemctl` only — no shell composition.

use std::process::Command;
use std::thread;
use std::time::Duration;

use serde_json;

use crate::adapters::local::vault_inventory::FsLocalModelInventory;
use crate::domain::local_runtime::{
    LocalRuntimeMutationReport, LocalRuntimeState, LocalRuntimeStatus,
};
use crate::ports::local_runtime_port::{
    LocalInferenceRuntimePort, LocalModelInventoryPort, LocalRuntimeError,
};

const UNIT_PREFIX: &str = "llama-server@";

#[derive(Debug)]
pub struct SystemdUserLocalRuntime {
    inventory: FsLocalModelInventory,
}

impl SystemdUserLocalRuntime {
    pub fn from_env() -> Self {
        Self {
            inventory: FsLocalModelInventory::from_env(),
        }
    }

    pub fn with_inventory(inventory: FsLocalModelInventory) -> Self {
        Self { inventory }
    }

    fn unit_name(alias: &str) -> String {
        format!("{UNIT_PREFIX}{alias}.service")
    }

    fn systemctl(args: &[&str]) -> Result<String, LocalRuntimeError> {
        let output = Command::new("systemctl")
            .args(args)
            .output()
            .map_err(|e| LocalRuntimeError::Control(format!("systemctl spawn failed: {e}")))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            return Err(LocalRuntimeError::Control(format!(
                "systemctl {} failed: {stderr}{stdout}",
                args.join(" ")
            )));
        }
        Ok(stdout)
    }

    fn parse_show(blob: &str) -> (String, LocalRuntimeState, Option<u32>, Option<u64>, String) {
        let mut active = String::new();
        let mut main_pid: Option<u32> = None;
        let mut memory: Option<u64> = None;
        let mut id = String::new();
        for line in blob.lines() {
            if let Some((k, v)) = line.split_once('=') {
                match k {
                    "ActiveState" => active = v.to_string(),
                    "MainPID" => {
                        if let Ok(p) = v.parse::<u32>()
                            && p > 0
                        {
                            main_pid = Some(p);
                        }
                    }
                    "MemoryCurrent" => {
                        if v != "[not set]"
                            && let Ok(m) = v.parse::<u64>()
                        {
                            memory = Some(m);
                        }
                    }
                    "Id" => id = v.to_string(),
                    _ => {}
                }
            }
        }

        let state = match active.as_str() {
            "active" => LocalRuntimeState::Active,
            "activating" => LocalRuntimeState::Activating,
            "failed" => LocalRuntimeState::Failed,
            "inactive" | "dead" => LocalRuntimeState::Inactive,
            _ => LocalRuntimeState::Unknown,
        };

        (id, state, main_pid, memory, active)
    }

    fn status_for_unit(&self, unit: &str) -> Result<LocalRuntimeStatus, LocalRuntimeError> {
        let blob = match Self::systemctl(&[
            "--user",
            "show",
            unit,
            "--property=Id,ActiveState,MainPID,MemoryCurrent",
            "--no-page",
        ]) {
            Ok(b) => b,
            Err(e) => {
                return Ok(LocalRuntimeStatus {
                    schema: LocalRuntimeStatus::SCHEMA,
                    surface: LocalRuntimeStatus::SURFACE,
                    alias: unit
                        .strip_prefix(UNIT_PREFIX)
                        .and_then(|s| s.strip_suffix(".service"))
                        .map(|s| s.to_string()),
                    state: LocalRuntimeState::Unknown,
                    unit: Some(unit.to_string()),
                    main_pid: None,
                    memory_current_bytes: None,
                    endpoint: None,
                    listen_host: None,
                    listen_port: None,
                    message: Some(e.to_string()),
                });
            }
        };

        let (id, state, main_pid, memory, _) = Self::parse_show(&blob);
        let alias = id
            .strip_prefix(UNIT_PREFIX)
            .and_then(|s| s.strip_suffix(".service"))
            .map(|s| s.to_string());

        let mut status = LocalRuntimeStatus {
            schema: LocalRuntimeStatus::SCHEMA,
            surface: LocalRuntimeStatus::SURFACE,
            alias,
            state,
            unit: if id.is_empty() {
                Some(unit.to_string())
            } else {
                Some(id)
            },
            main_pid,
            memory_current_bytes: memory,
            endpoint: None,
            listen_host: None,
            listen_port: None,
            message: None,
        };

        if let Some(ref a) = status.alias
            && let Ok(profile) = self.inventory.get_profile(a)
        {
            status.listen_host = Some(profile.host.clone());
            status.listen_port = Some(profile.port);
            status.endpoint = Some(format!("http://{}:{}/v1", profile.host, profile.port));
        }
        Ok(status)
    }

    fn list_running_aliases() -> Result<Vec<String>, LocalRuntimeError> {
        let out = Command::new("systemctl")
            .args([
                "--user",
                "list-units",
                "--type=service",
                "--state=running",
                "--no-legend",
                "--no-pager",
                "llama-server@*.service",
            ])
            .output()
            .map_err(|e| LocalRuntimeError::Control(format!("list-units failed: {e}")))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut aliases = Vec::new();
        for line in stdout.lines() {
            let unit = line.split_whitespace().next().unwrap_or("");
            if let Some(a) = unit
                .strip_prefix(UNIT_PREFIX)
                .and_then(|s| s.strip_suffix(".service"))
            {
                aliases.push(a.to_string());
            }
        }
        Ok(aliases)
    }

    fn stop_legacy_fixed_unit() {
        let _ = Self::systemctl(&["--user", "stop", "llama-server.service"]);
    }

    fn expected_model_ids(
        profile: &crate::domain::local_runtime::LocalModelProfile,
    ) -> Vec<String> {
        let mut ids = Vec::new();
        ids.push(profile.alias.clone());

        if let Some(name) = std::path::Path::new(&profile.model_path)
            .file_name()
            .and_then(|p| p.to_str())
        {
            let stem = std::path::Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(name);
            ids.push(name.to_string());
            ids.push(stem.to_string());
        }

        ids.into_iter()
            .map(|s| s.to_lowercase())
            .collect::<std::collections::HashSet<_>>()
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
                    .or_else(|| m.get("model"))
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

    fn fetch_models_for_wait(endpoint: &str) -> (String, Vec<String>) {
        let body = Command::new("curl")
            .args([
                "-sS",
                "-m",
                "2",
                "-o",
                "-",
                "-H",
                "Accept: application/json",
                endpoint,
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        let code = Command::new("curl")
            .args([
                "-sS",
                "-m",
                "2",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                endpoint,
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default()
            .trim()
            .to_string();

        (code, Self::parse_model_ids(&body))
    }

    fn estimate_model_ready_timeout(
        profile: &crate::domain::local_runtime::LocalModelProfile,
    ) -> Duration {
        let size_gb = profile.size_bytes.unwrap_or(0) as f64 / (1024.0 * 1024.0 * 1024.0);
        let extra = (size_gb * 45.0).round() as u64;
        let timeout_secs = 180u64 + extra;
        Duration::from_secs(timeout_secs.min(1200))
    }

    fn wait_model_ready(
        &self,
        profile: &crate::domain::local_runtime::LocalModelProfile,
        timeout: Duration,
    ) -> Result<bool, LocalRuntimeError> {
        let endpoint = format!("http://{}:{}/v1/models", profile.host, profile.port);
        let start = std::time::Instant::now();

        let expected = Self::expected_model_ids(profile);
        let mut last_seen = Vec::<String>::new();
        let mut last_state = String::from("initializing");
        let mut wait = crate::progress::WaitProgress::new(
            format!(
                "model HTTP {endpoint} (load can take ~{}s for large GGUF)",
                timeout.as_secs()
            ),
            5,
        );

        while start.elapsed() < timeout {
            if let Ok(st) = self.status_for_unit(&Self::unit_name(&profile.alias)) {
                if st.state == LocalRuntimeState::Failed {
                    crate::progress::progress(
                        "failed",
                        format!(
                            "llama-server unit failed for {} — check journalctl --user -u {}",
                            profile.alias,
                            Self::unit_name(&profile.alias)
                        ),
                    );
                    wait.done(false);
                    return Ok(false);
                }
                last_state = st.state.as_str().into();
            }

            let (code, models) = Self::fetch_models_for_wait(&endpoint);
            if !models.is_empty() {
                last_seen = models.clone();
            }

            let code_ok = matches!(code.as_str(), "200" | "401");
            if code_ok {
                if Self::model_matches_target(&models, &expected) {
                    wait.done(true);
                    return Ok(true);
                }

                if models.is_empty() {
                    crate::progress::progress(
                        "loading",
                        format!("HTTP {code} ({last_state}) but no model ids yet"),
                    );
                } else {
                    let shown = last_seen.join(", ");
                    crate::progress::progress(
                        "loading",
                        format!(
                            "HTTP {code} ({last_state}) loading different models={}",
                            shown.chars().take(160).collect::<String>()
                        ),
                    );
                }
            } else {
                crate::progress::progress("wait", format!("HTTP {code} ({last_state})"));
            }

            wait.tick();
            thread::sleep(Duration::from_millis(500));
        }

        wait.done(false);
        Ok(false)
    }
}

impl LocalInferenceRuntimePort for SystemdUserLocalRuntime {
    fn status(&self) -> Result<LocalRuntimeStatus, LocalRuntimeError> {
        let running = Self::list_running_aliases()?;
        if running.is_empty() {
            return Ok(LocalRuntimeStatus::inactive(
                "no native llama-server@ instance running",
            ));
        }
        let alias = &running[0];
        self.status_for_unit(&Self::unit_name(alias))
    }

    fn start(&self, alias: &str) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        crate::progress::progress("start", format!("alias={alias} supervisor=systemd-user"));
        let profile = self.inventory.get_profile(alias)?;
        if !profile.model_exists {
            return Err(LocalRuntimeError::Rejected(format!(
                "model file missing for profile {alias}: {}",
                profile.model_path
            )));
        }
        if !profile.loopback_only {
            return Err(LocalRuntimeError::Rejected(format!(
                "refusing to start {alias}: host {} is not loopback (native sovereign policy)",
                profile.host
            )));
        }

        // GGUF architecture preflight — fail closed before systemd restart storms.
        crate::progress::progress("preflight", format!("gguf {}", profile.model_path));
        let model_path = std::path::Path::new(&profile.model_path);
<<<<<<< HEAD
        let store =
            crate::adapters::local::engine_capability_store::EngineCapabilityStore::load_default();
        let gguf_meta =
            crate::infra::gguf_meta::preflight_gguf_with_store(model_path, store.as_ref())
                .map_err(|e| LocalRuntimeError::Rejected(e.to_string()))?;
=======
        let gguf_meta = crate::domain::gguf_meta::preflight_gguf_for_engine(
            model_path,
            crate::domain::gguf_meta::default_supported_arches_for_llama_b9444(),
        )
        .map_err(|e| LocalRuntimeError::Rejected(e.to_string()))?;
>>>>>>> github/main

        // Probe-driven plan: regenerate runtime env + resource drop-in every start.
        crate::progress::progress("plan", "probe hardware + materialize runtime env/drop-in");
        let planner = crate::application::runtime_planner::RuntimePlanner::from_env();
        let plan = planner.plan_for_alias(alias)?;
<<<<<<< HEAD
        let materialize = planner.materialize(&plan)?;
=======
        let materialize = planner.materialize_systemd_user(&plan)?;
>>>>>>> github/main

        Self::stop_legacy_fixed_unit();
        for other in Self::list_running_aliases()? {
            if other != alias {
                crate::progress::progress("stop", format!("unit {}", Self::unit_name(&other)));
                let _ = Self::systemctl(&["--user", "stop", &Self::unit_name(&other)]);
            }
        }

        let _ = Self::systemctl(&["--user", "daemon-reload"]);
        // Clear failed state from previous crash loops
        let _ = Self::systemctl(&["--user", "reset-failed", &Self::unit_name(alias)]);
        crate::progress::progress(
            "spawn",
            format!("systemctl --user start {}", Self::unit_name(alias)),
        );
        Self::systemctl(&["--user", "start", &Self::unit_name(alias)])?;

        let plan_msg = format!(
            "probe plan: arch={} ngl={} threads={} mem={} cpu_quota={} env={} dropin={}",
            gguf_meta.architecture.as_deref().unwrap_or("?"),
            plan.n_gpu_layers,
            plan.threads,
            plan.memory_max,
            plan.cpu_quota,
            materialize.runtime_env,
            materialize.systemd_dropin
        );

        // Wait for unit active, then for model readiness (HTTP /v1/models != Loading).
        crate::progress::progress(
            "load",
            "waiting for llama-server to finish loading weights into memory/VRAM",
        );
        let ready_timeout = Self::estimate_model_ready_timeout(&profile);
        let ready = self.wait_model_ready(&profile, ready_timeout)?;
        let st = self.status_for_unit(&Self::unit_name(alias))?;
        let ok = ready && st.state == LocalRuntimeState::Active;
        if !ok {
            // Stop crash loops on load failure
            let _ = Self::systemctl(&["--user", "stop", &Self::unit_name(alias)]);
            let _ = Self::systemctl(&["--user", "reset-failed", &Self::unit_name(alias)]);
        }
        let st = self.status_for_unit(&Self::unit_name(alias)).unwrap_or(st);
        Ok(LocalRuntimeMutationReport {
            schema: LocalRuntimeMutationReport::SCHEMA,
            surface: LocalRuntimeMutationReport::SURFACE,
            operation: "start".into(),
            alias: alias.to_string(),
            ok,
            unit: Self::unit_name(alias),
            status: st,
            message: Some(if ok {
                format!("{plan_msg}; model ready on endpoint")
            } else {
                format!(
                    "{plan_msg}; model NOT ready — check journalctl --user -u {}",
                    Self::unit_name(alias)
                )
            }),
        })
    }

    fn stop(&self, alias: Option<&str>) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        let alias = match alias {
            Some(a) => a.to_string(),
            None => {
                let running = Self::list_running_aliases()?;
                running.into_iter().next().ok_or_else(|| {
                    LocalRuntimeError::Rejected("no running native instance to stop".into())
                })?
            }
        };

        let unit = Self::unit_name(&alias);
        let _ = Command::new("systemctl")
            .args(["--user", "stop", &unit])
            .output();
        let st = self.status_for_unit(&unit)?;
        Ok(LocalRuntimeMutationReport {
            schema: LocalRuntimeMutationReport::SCHEMA,
            surface: LocalRuntimeMutationReport::SURFACE,
            operation: "stop".into(),
            alias,
            ok: matches!(
                st.state,
                LocalRuntimeState::Inactive | LocalRuntimeState::Unknown
            ),
            unit,
            status: st,
            message: None,
        })
    }
}
