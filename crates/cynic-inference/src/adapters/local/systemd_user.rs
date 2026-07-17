//! systemd --user adapter for native on-demand llama-server instances.
//!
//! Unit template: `llama-server@<alias>.service`
//! EnvironmentFile: `%h/.config/cynic/models/<alias>.env` (declared by unit).
//!
//! Uses argv-based `systemctl` only — no shell composition.

use std::process::Command;
use std::thread;
use std::time::Duration;

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
                        if let Ok(p) = v.parse::<u32>() {
                            if p > 0 {
                                main_pid = Some(p);
                            }
                        }
                    }
                    "MemoryCurrent" => {
                        if v != "[not set]" {
                            if let Ok(m) = v.parse::<u64>() {
                                memory = Some(m);
                            }
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
            alias: alias.clone(),
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

        if let Some(ref a) = status.alias {
            if let Ok(profile) = self.inventory.get_profile(a) {
                status.listen_host = Some(profile.host.clone());
                status.listen_port = Some(profile.port);
                status.endpoint = Some(format!("http://{}:{}/v1", profile.host, profile.port));
            }
        }
        Ok(status)
    }

    fn list_running_aliases(&self) -> Result<Vec<String>, LocalRuntimeError> {
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

    fn stop_legacy_fixed_unit(&self) {
        let _ = Self::systemctl(&["--user", "stop", "llama-server.service"]);
    }

    /// Poll until OpenAI-compat `/v1/models` returns success (model finished loading).
    fn wait_model_ready(
        &self,
        profile: &crate::domain::local_runtime::LocalModelProfile,
        timeout: Duration,
    ) -> Result<bool, LocalRuntimeError> {
        let endpoint = format!("http://{}:{}/v1/models", profile.host, profile.port);
        let start = std::time::Instant::now();
        let api_key_path = dirs_api_key_path();
        let api_key = std::fs::read_to_string(&api_key_path).ok();
        let mut wait = crate::progress::WaitProgress::new(
            format!("model HTTP {endpoint} (load can take 1–3 min)"),
            5,
        );

        while start.elapsed() < timeout {
            // Unit failed?
            if let Ok(st) = self.status_for_unit(&Self::unit_name(&profile.alias)) {
                if st.state == LocalRuntimeState::Failed {
                    wait.done(false);
                    return Ok(false);
                }
                // crash loop: inactive after having started
                if st.state == LocalRuntimeState::Inactive
                    && start.elapsed() > Duration::from_secs(3)
                {
                    // may be restarting — check a few more times
                }
            }

            let mut cmd = Command::new("curl");
            cmd.args(["-sS", "-m", "2", "-o", "/dev/null", "-w", "%{http_code}"]);
            if let Some(ref key) = api_key {
                cmd.arg("-H")
                    .arg(format!("Authorization: Bearer {}", key.trim()));
            }
            cmd.arg(&endpoint);
            if let Ok(out) = cmd.output() {
                let code = String::from_utf8_lossy(&out.stdout);
                if code.trim() == "200" {
                    wait.done(true);
                    return Ok(true);
                }
                // 503 Loading model — keep waiting
            }
            wait.tick();
            thread::sleep(Duration::from_millis(500));
        }
        wait.done(false);
        Ok(false)
    }
}

fn dirs_api_key_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".config")
        });
    base.join("cynic/llama-api-key")
}

impl LocalInferenceRuntimePort for SystemdUserLocalRuntime {
    fn status(&self) -> Result<LocalRuntimeStatus, LocalRuntimeError> {
        let running = self.list_running_aliases()?;
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
        let gguf_meta = crate::domain::gguf_meta::preflight_gguf_for_engine(
            model_path,
            crate::domain::gguf_meta::default_supported_arches_for_llama_b9444(),
        )
        .map_err(|e| LocalRuntimeError::Rejected(e.to_string()))?;

        // Probe-driven plan: regenerate runtime env + resource drop-in every start.
        crate::progress::progress("plan", "probe hardware + materialize runtime env/drop-in");
        let planner = crate::application::runtime_planner::RuntimePlanner::from_env();
        let plan = planner.plan_for_alias(alias)?;
        let materialize = planner.materialize(&plan)?;

        self.stop_legacy_fixed_unit();
        for other in self.list_running_aliases()? {
            if other != alias {
                crate::progress::progress("stop", format!("unit {}", Self::unit_name(&other)));
                let _ = Self::systemctl(&["--user", "stop", &Self::unit_name(&other)]);
            }
        }

        let _ = Self::systemctl(&["--user", "daemon-reload"]);
        // Clear failed state from previous crash loops
        let _ = Self::systemctl(&["--user", "reset-failed", &Self::unit_name(alias)]);
        crate::progress::progress("spawn", format!("systemctl --user start {}", Self::unit_name(alias)));
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
        let ready = self.wait_model_ready(&profile, Duration::from_secs(180))?;
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
                let running = self.list_running_aliases()?;
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
            alias: alias.clone(),
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
