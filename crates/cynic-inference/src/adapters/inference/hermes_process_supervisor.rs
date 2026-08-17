//! Process adapter for a CYNIC-owned Hermes gateway.
//!
//! This is an execution adapter only. Desired state and lifecycle semantics
//! stay in `domain::hermes_runtime` and `ports::hermes_runtime_port`.

use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::application::agent_harness::canonicalize_hermes_runtime_id;
use crate::application::chat_endpoint::resolve_chat_api_key;
use crate::domain::hermes_runtime::{
    HermesGatewayMutation, HermesGatewayProfile, HermesGatewayState, HermesGatewayStatus,
};
use crate::ports::hermes_runtime_port::{HermesGatewayRuntimeError, HermesGatewayRuntimePort};

#[derive(Debug, Clone)]
pub struct ProcessHermesGateway {
    profile: HermesGatewayProfile,
    state_dir: PathBuf,
    pid_path: PathBuf,
    log_path: PathBuf,
}

impl ProcessHermesGateway {
    pub fn from_env() -> Self {
        let home = std::env::var_os("CYNIC_PRINCIPAL_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."))
            });
        let runtime_id = canonicalize_hermes_runtime_id(
            &std::env::var("CYNIC_HERMES_RUNTIME_ID").unwrap_or_else(|_| "hermes-agent".into()),
        );
        let port = std::env::var("CYNIC_HERMES_PORT")
            .or_else(|_| std::env::var("API_SERVER_PORT"))
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(8642);
        let host = std::env::var("CYNIC_HERMES_HOST")
            .or_else(|_| std::env::var("API_SERVER_HOST"))
            .unwrap_or_else(|_| "127.0.0.1".into());
        let home_dir = std::env::var("HERMES_HOME")
            .unwrap_or_else(|_| home.join(".hermes").display().to_string());
        let executable = std::env::var("CYNIC_HERMES_BIN").unwrap_or_else(|_| "hermes".into());
        let state_dir = home.join(".local/state/cynic/inference/hermes");
        Self {
            profile: HermesGatewayProfile {
                runtime_id: runtime_id.clone(),
                executable,
                home_dir,
                host: host.clone(),
                port,
                api_key_present: api_key().is_some(),
                max_agent_turns: std::env::var("CYNIC_HERMES_MAX_TURNS")
                    .ok()
                    .and_then(|value| value.parse().ok()),
            },
            pid_path: state_dir.join(format!("{runtime_id}.pid")),
            log_path: state_dir.join(format!("{runtime_id}.log")),
            state_dir,
        }
    }

    pub fn with_profile(profile: HermesGatewayProfile, state_dir: PathBuf) -> Self {
        let runtime_id = profile.runtime_id.clone();
        Self {
            profile,
            pid_path: state_dir.join(format!("{runtime_id}.pid")),
            log_path: state_dir.join(format!("{runtime_id}.log")),
            state_dir,
        }
    }

    fn endpoint(&self) -> String {
        format!("http://{}:{}", self.profile.host, self.profile.port)
    }

    fn read_pid(&self) -> Option<u32> {
        let pid = fs::read_to_string(&self.pid_path)
            .ok()?
            .trim()
            .parse()
            .ok()?;
        if PathBuf::from(format!("/proc/{pid}")).is_dir() {
            Some(pid)
        } else {
            let _ = fs::remove_file(&self.pid_path);
            None
        }
    }

    fn write_pid(&self, pid: u32) -> Result<(), HermesGatewayRuntimeError> {
        fs::create_dir_all(&self.state_dir)
            .map_err(|error| HermesGatewayRuntimeError::Control(format!("mkdir_state:{error}")))?;
        fs::write(&self.pid_path, format!("{pid}\n"))
            .map_err(|error| HermesGatewayRuntimeError::Control(format!("write_pid:{error}")))
    }

    fn api_key(&self) -> Option<String> {
        read_hermes_api_key()
    }

    fn probe(&self) -> HermesGatewayStatus {
        let endpoint = self.endpoint();
        let pid = self.read_pid();
        let Some(pid) = pid else {
            return HermesGatewayStatus::inactive(
                self.profile.runtime_id.clone(),
                endpoint,
                format!("no process; log={}", self.log_path.display()),
            );
        };
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                return HermesGatewayStatus {
                    schema: crate::domain::hermes_runtime::HERMES_GATEWAY_STATUS_SCHEMA,
                    runtime_id: self.profile.runtime_id.clone(),
                    state: HermesGatewayState::Unknown,
                    pid: Some(pid),
                    endpoint,
                    health_ok: false,
                    capabilities_ok: false,
                    model_count: None,
                    message: Some(format!("runtime_probe:{error}")),
                };
            }
        };
        let (health_ok, capabilities_ok, model_count, message) = runtime.block_on(async {
            let client = reqwest::Client::new();
            let auth = self.api_key();
            let request = |url: String| {
                let client = client.clone();
                let auth = auth.clone();
                async move {
                    let mut request = client.get(url);
                    if let Some(auth) = auth {
                        request = request.bearer_auth(auth);
                    }
                    request.send().await
                }
            };
            let health_ok = request(format!("{endpoint}/health"))
                .await
                .is_ok_and(|response| response.status().is_success());
            let capabilities_ok = request(format!("{endpoint}/v1/capabilities"))
                .await
                .is_ok_and(|response| response.status().is_success());
            let model_count = match request(format!("{endpoint}/v1/models")).await {
                Ok(response) => response
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|value| value["data"].as_array().map(Vec::len)),
                Err(_) => None,
            };
            let message =
                (!health_ok || !capabilities_ok).then(|| "hermes_gateway_not_ready".to_string());
            (health_ok, capabilities_ok, model_count, message)
        });
        HermesGatewayStatus {
            schema: crate::domain::hermes_runtime::HERMES_GATEWAY_STATUS_SCHEMA,
            runtime_id: self.profile.runtime_id.clone(),
            state: if health_ok && capabilities_ok {
                HermesGatewayState::Active
            } else {
                HermesGatewayState::Failed
            },
            pid: Some(pid),
            endpoint,
            health_ok,
            capabilities_ok,
            model_count,
            message,
        }
    }

    fn mutation(
        &self,
        operation: &str,
        status: HermesGatewayStatus,
        ok: bool,
        message: Option<String>,
    ) -> HermesGatewayMutation {
        HermesGatewayMutation {
            schema: crate::domain::hermes_runtime::HERMES_GATEWAY_MUTATION_SCHEMA,
            operation: operation.into(),
            runtime_id: self.profile.runtime_id.clone(),
            ok,
            status,
            message,
        }
    }
}

impl HermesGatewayRuntimePort for ProcessHermesGateway {
    fn status(&self) -> Result<HermesGatewayStatus, HermesGatewayRuntimeError> {
        Ok(self.probe())
    }

    fn start(
        &self,
        wait_for_ready: bool,
    ) -> Result<HermesGatewayMutation, HermesGatewayRuntimeError> {
        if self.api_key().is_none() {
            return Err(HermesGatewayRuntimeError::Control(
                "missing Hermes API server key".into(),
            ));
        }
        if self.read_pid().is_some() {
            let status = self.probe();
            let ok = status.health_ok && status.capabilities_ok;
            return Ok(self.mutation(
                "adopt",
                status,
                ok,
                Some(
                    if ok {
                        "already_adopted"
                    } else {
                        "adopted_not_ready"
                    }
                    .into(),
                ),
            ));
        }
        fs::create_dir_all(&self.state_dir)
            .map_err(|error| HermesGatewayRuntimeError::Control(format!("mkdir_state:{error}")))?;
        fs::create_dir_all(&self.profile.home_dir).map_err(|error| {
            HermesGatewayRuntimeError::Control(format!(
                "hermes_home_not_writable:{}:{error}",
                self.profile.home_dir
            ))
        })?;
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .map_err(|error| HermesGatewayRuntimeError::Control(format!("open_log:{error}")))?;
        let mut command = Command::new(&self.profile.executable);
        command
            .args(["gateway", "run", "--force", "--no-supervise", "--quiet"])
            .env("HERMES_HOME", &self.profile.home_dir)
            .env("API_SERVER_ENABLED", "true")
            .env("API_SERVER_HOST", &self.profile.host)
            .env("API_SERVER_PORT", self.profile.port.to_string())
            .stdout(Stdio::from(log.try_clone().map_err(|error| {
                HermesGatewayRuntimeError::Control(format!("clone_log:{error}"))
            })?))
            .stderr(Stdio::from(log));
        if let Some(api_key) = self.api_key() {
            command.env("API_SERVER_KEY", api_key);
        }
        if let Some(local_key) = resolve_chat_api_key(None) {
            command.env("CYNIC_LLAMA_API_KEY", local_key);
        }
        let child = command.spawn().map_err(|error| {
            HermesGatewayRuntimeError::Control(format!("spawn_gateway:{error}"))
        })?;
        self.write_pid(child.id())?;
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut status = self.probe();
        if wait_for_ready {
            while Instant::now() < deadline {
                if status.health_ok && status.capabilities_ok {
                    break;
                }
                thread::sleep(Duration::from_millis(250));
                status = self.probe();
            }
        }
        let ok = status.health_ok && status.capabilities_ok;
        Ok(self.mutation(
            "start",
            status,
            ok,
            (!ok).then(|| "readiness_timeout".into()),
        ))
    }

    fn stop(&self) -> Result<HermesGatewayMutation, HermesGatewayRuntimeError> {
        let Some(pid) = self.read_pid() else {
            return Ok(self.mutation("stop", self.probe(), true, Some("already_inactive".into())));
        };
        let status = Command::new("/bin/kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .map_err(|error| HermesGatewayRuntimeError::Control(format!("stop_gateway:{error}")))?;
        thread::sleep(Duration::from_millis(300));
        let _ = fs::remove_file(&self.pid_path);
        Ok(self.mutation(
            "stop",
            self.probe(),
            status.success(),
            (!status.success()).then(|| "kill_failed".into()),
        ))
    }
}

fn api_key() -> Option<String> {
    read_hermes_api_key()
}

/// Resolve the Hermes API key using the canonical adapter precedence.
///
/// Resolution order is:
/// - `CYNIC_HERMES_API_KEY`
/// - `API_SERVER_KEY`
/// - `CYNIC_HERMES_API_KEY_FILE`
/// - `${CYNIC_PRINCIPAL_HOME|HOME}/.config/cynic/hermes-api-key`
/// - `/etc/cynic/hermes-api-key`
pub fn read_hermes_api_key() -> Option<String> {
    std::env::var("CYNIC_HERMES_API_KEY")
        .ok()
        .or_else(|| std::env::var("API_SERVER_KEY").ok())
        .or_else(|| {
            std::env::var_os("CYNIC_HERMES_API_KEY_FILE")
                .and_then(|path| fs::read_to_string(PathBuf::from(path)).ok())
                .map(|value| value.trim().to_string())
        })
        .or_else(|| {
            let home = std::env::var_os("CYNIC_PRINCIPAL_HOME")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)?;
            fs::read_to_string(home.join(".config/cynic/hermes-api-key")).ok()
        })
        .or_else(|| fs::read_to_string("/etc/cynic/hermes-api-key").ok())
        .map(|value| value.trim().to_string())
}
