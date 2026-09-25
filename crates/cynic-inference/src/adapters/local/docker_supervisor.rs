//! Docker-based local inference supervisor integrated with Osmantic ODS Compose infrastructure.
//!
//! Controls the ODS compose-managed `dream-llama-server` container by updating
<<<<<<< HEAD
//! `<CYNIC_ODS_DIR>/.env` and using `<CYNIC_ODS_DIR>/dream-cli` to restart the service.
=======
//! `/home/user/dream-server/.env` and using `/home/user/dream-server/dream-cli` to restart the service.
>>>>>>> github/main
//!
//! Select with `CYNIC_INFERENCE_SUPERVISOR=docker`.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::adapters::local::vault_inventory::FsLocalModelInventory;
use crate::domain::local_runtime::{
    LocalRuntimeMutationReport, LocalRuntimeState, LocalRuntimeStatus,
};
use crate::ports::local_runtime_port::{
    LocalInferenceRuntimePort, LocalModelInventoryPort, LocalRuntimeError,
};

const SUPERVISOR_SURFACE: &str = "docker";
<<<<<<< HEAD
const ODS_DIR_ENV: &str = "CYNIC_ODS_DIR";
const ODS_DIR_DEFAULT: &str = "/var/lib/cynic/ods";

/// Root of the ODS compose deployment this supervisor drives.
///
/// Previously a `const` holding one operator's home directory, which made the
/// adapter unusable on any other host and embedded a machine-local path in
/// compiled code. Injected via `CYNIC_ODS_DIR`, defaulting to a principal-owned
/// path rather than an operator home.
fn ods_dir() -> std::path::PathBuf {
    std::env::var(ODS_DIR_ENV)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(ODS_DIR_DEFAULT))
}
=======
const ODS_DIR: &str = "/home/user/dream-server";
>>>>>>> github/main

#[derive(Debug)]
pub struct DockerLocalRuntime {
    inventory: FsLocalModelInventory,
}

impl DockerLocalRuntime {
    pub fn from_env() -> Self {
        Self {
            inventory: FsLocalModelInventory::from_env(),
        }
    }

    pub fn with_inventory(inventory: FsLocalModelInventory) -> Self {
        Self { inventory }
    }

    fn read_active_env(&self) -> Result<(String, String), std::io::Error> {
<<<<<<< HEAD
        let env_path = ods_dir().join(".env");
=======
        let env_path = Path::new(ODS_DIR).join(".env");
>>>>>>> github/main
        let content = fs::read_to_string(env_path)?;
        let mut model = String::new();
        let mut port = "11434".to_string();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "LLM_MODEL" => model = v.trim().to_string(),
                    "OLLAMA_PORT" => port = v.trim().to_string(),
                    _ => {}
                }
            }
        }
        Ok((model, port))
    }

    fn is_container_running(&self) -> bool {
        let output = Command::new("docker")
            .args([
                "ps",
                "--filter",
                "name=dream-llama-server",
                "--format",
                "{{.Names}}",
            ])
            .output();
        let Ok(out) = output else {
            return false;
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        stdout.lines().any(|l| l.trim() == "dream-llama-server")
    }

    fn update_env_file(
        &self,
        alias: &str,
        gguf: &str,
        ctx: &str,
        port: &str,
    ) -> Result<(), std::io::Error> {
<<<<<<< HEAD
        let env_path = ods_dir().join(".env");
=======
        let env_path = Path::new(ODS_DIR).join(".env");
>>>>>>> github/main
        let content = fs::read_to_string(&env_path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        let keys = [
            ("LLM_MODEL", alias),
            ("GGUF_FILE", gguf),
            ("CTX_SIZE", ctx),
            ("MAX_CONTEXT", ctx),
            ("OLLAMA_PORT", port),
        ];

        for (k, v) in keys {
            let prefix = format!("{k}=");
            let mut found = false;
            for line in &mut lines {
                if line.trim().starts_with(&prefix) {
                    *line = format!("{k}={v}");
                    found = true;
                    break;
                }
            }
            if !found {
                lines.push(format!("{k}={v}"));
            }
        }

        fs::write(env_path, lines.join("\n") + "\n")?;
        Ok(())
    }
}

impl LocalInferenceRuntimePort for DockerLocalRuntime {
    fn status(&self) -> Result<LocalRuntimeStatus, LocalRuntimeError> {
        let running = self.is_container_running();
        let (alias, port) = self
            .read_active_env()
            .unwrap_or_else(|_| ("unknown".to_string(), "11434".to_string()));

        if !running {
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
                message: Some("dream-llama-server container is not running".to_string()),
            });
        }

        Ok(LocalRuntimeStatus {
            schema: LocalRuntimeStatus::SCHEMA,
            surface: SUPERVISOR_SURFACE,
            alias: Some(alias),
            state: LocalRuntimeState::Active,
            unit: Some("dream-llama-server".to_string()),
            main_pid: None,
            memory_current_bytes: None,
            endpoint: Some(format!("http://127.0.0.1:{port}")),
            listen_host: Some("127.0.0.1".to_string()),
            listen_port: Some(port.parse().unwrap_or(11434)),
            message: Some("dream-llama-server container is running".to_string()),
        })
    }

    fn start(&self, alias: &str) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        let profile = self.inventory.get_profile(alias)?;

        // Environment overrides
        let mut ctx = profile.ctx_size.to_string();
        if let Ok(override_ctx) = std::env::var("CYNIC_OVERRIDE_CTX_SIZE") {
            ctx = override_ctx;
        }
        let mut port = profile.port.to_string();
        if let Ok(override_port) = std::env::var("CYNIC_OVERRIDE_PORT") {
            port = override_port;
        }

        // Symlink weights to ODS models folder
        let src_path = Path::new(&profile.model_path);
        let gguf_filename = src_path
            .file_name()
            .ok_or_else(|| LocalRuntimeError::Control("invalid model_path filename".into()))?
            .to_str()
            .ok_or_else(|| LocalRuntimeError::Control("invalid GGUF filename encoding".into()))?;

<<<<<<< HEAD
        let dest_path = ods_dir().join("data/models").join(gguf_filename);
=======
        let dest_path = Path::new(ODS_DIR).join("data/models").join(gguf_filename);
>>>>>>> github/main
        if dest_path.exists() {
            let _ = fs::remove_file(&dest_path);
        }

        std::os::unix::fs::symlink(src_path, &dest_path)
            .map_err(|e| LocalRuntimeError::Control(format!("symlink GGUF weight to ODS: {e}")))?;

        // Update ODS compose env config
        self.update_env_file(alias, gguf_filename, &ctx, &port)
            .map_err(|e| LocalRuntimeError::Control(format!("update ODS .env config: {e}")))?;

        crate::progress::progress(
            "start",
            format!("alias={alias} supervisor=docker compose=ODS"),
        );

        // Restart ODS llama-server using ODS CLI
<<<<<<< HEAD
        let status = Command::new(ods_dir().join("dream-cli"))
=======
        let status = Command::new(Path::new(ODS_DIR).join("dream-cli"))
>>>>>>> github/main
            .args(["restart", "llama-server"])
            .status()
            .map_err(|e| LocalRuntimeError::Control(format!("execute dream-cli restart: {e}")))?;

        if !status.success() {
            return Err(LocalRuntimeError::Control(format!(
                "dream-cli restart failed with exit status {:?}",
                status.code()
            )));
        }

        let ok = self.is_container_running();
        let endpoint = format!("http://127.0.0.1:{port}");

        let st = LocalRuntimeStatus {
            schema: LocalRuntimeStatus::SCHEMA,
            surface: SUPERVISOR_SURFACE,
            alias: Some(alias.to_string()),
            state: if ok {
                LocalRuntimeState::Active
            } else {
                LocalRuntimeState::Inactive
            },
            unit: Some("dream-llama-server".to_string()),
            main_pid: None,
            memory_current_bytes: None,
            endpoint: Some(endpoint),
            listen_host: Some("127.0.0.1".to_string()),
            listen_port: Some(port.parse().unwrap_or(11434)),
            message: if ok {
                None
            } else {
                Some("container died immediately after restart".to_string())
            },
        };

        Ok(LocalRuntimeMutationReport {
            schema: LocalRuntimeMutationReport::SCHEMA,
            surface: SUPERVISOR_SURFACE,
            operation: "start".into(),
            alias: alias.to_string(),
            unit: "dream-llama-server".to_string(),
            ok,
            status: st,
            message: Some(format!("resolved_alias={alias}")),
        })
    }

    fn stop(&self, _alias: Option<&str>) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        let (alias, _port) = self
            .read_active_env()
            .unwrap_or_else(|_| ("unknown".to_string(), "11434".to_string()));

        // Stop ODS llama-server using ODS CLI
<<<<<<< HEAD
        let status = Command::new(ods_dir().join("dream-cli"))
=======
        let status = Command::new(Path::new(ODS_DIR).join("dream-cli"))
>>>>>>> github/main
            .args(["stop", "llama-server"])
            .status()
            .map_err(|e| LocalRuntimeError::Control(format!("execute dream-cli stop: {e}")))?;

        if !status.success() {
            return Err(LocalRuntimeError::Control(format!(
                "dream-cli stop failed with exit status {:?}",
                status.code()
            )));
        }

        let ok = !self.is_container_running();

        let st = LocalRuntimeStatus {
            schema: LocalRuntimeStatus::SCHEMA,
            surface: SUPERVISOR_SURFACE,
            alias: None,
            state: LocalRuntimeState::Inactive,
            unit: Some("dream-llama-server".to_string()),
            main_pid: None,
            memory_current_bytes: None,
            endpoint: None,
            listen_host: None,
            listen_port: None,
            message: Some("stopped".to_string()),
        };

        Ok(LocalRuntimeMutationReport {
            schema: LocalRuntimeMutationReport::SCHEMA,
            surface: SUPERVISOR_SURFACE,
            operation: "stop".into(),
            alias: alias.clone(),
            unit: "dream-llama-server".to_string(),
            ok,
            status: st,
            message: Some(format!("resolved_alias={alias} stopped container")),
        })
    }
}
