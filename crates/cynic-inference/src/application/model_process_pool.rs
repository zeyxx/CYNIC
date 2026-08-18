//! Multi-Process Model Pool for Sovereign Concurrent Inference.
//!
//! Manages multiple active GGUF `llama-server` processes on distinct ports
//! (e.g. 8080, 8081, 8082) with strict hardware memory budget enforcement.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use crate::adapters::local::ProcessLocalRuntime;
use crate::adapters::local::vault_inventory::FsLocalModelInventory;
use crate::domain::hardware_profile::HardwareProfile;
use crate::ports::local_runtime_port::{
    LocalInferenceRuntimePort, LocalModelInventoryPort, LocalRuntimeError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePoolSlot {
    pub alias: String,
    pub port: u16,
    pub endpoint: String,
    pub pid: u32,
    pub weight_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPoolReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub active_slots: Vec<ActivePoolSlot>,
    pub total_active_weight_bytes: u64,
    pub memory_budget_bytes: u64,
    pub available_ram_bytes: u64,
}

impl ModelPoolReport {
    pub const SCHEMA: &'static str = "cynic.inference.model_process_pool.v1";
}

#[derive(Debug)]
pub struct ModelProcessPool {
    runtime: ProcessLocalRuntime,
    inventory: FsLocalModelInventory,
    active_ports: Arc<Mutex<HashMap<String, u16>>>,
    pub base_port: u16,
}

impl ModelProcessPool {
    pub fn new() -> Self {
        Self {
            runtime: ProcessLocalRuntime::from_env(),
            inventory: FsLocalModelInventory::from_env(),
            active_ports: Arc::new(Mutex::new(HashMap::new())),
            base_port: 8080,
        }
    }

    /// Find an available TCP port starting from base_port.
    pub fn find_available_port(&self, start_port: u16) -> Result<u16, LocalRuntimeError> {
        for port in start_port..(start_port + 50) {
            if TcpListener::bind(("127.0.0.1", port)).is_ok() {
                return Ok(port);
            }
        }
        Err(LocalRuntimeError::Control(
            "no free ports available in range 8080-8130".to_string(),
        ))
    }

    /// Ensure model profile is running in the pool. If not running, checks hardware memory budget and launches on a free port.
    pub fn ensure_model_active(&self, alias: &str) -> Result<ActivePoolSlot, LocalRuntimeError> {
        let inv = self.inventory.list_profiles()?;
        let profile = inv
            .models
            .iter()
            .find(|p| p.alias == alias)
            .ok_or_else(|| LocalRuntimeError::UnknownProfile(alias.to_string()))?;

        let hw = HardwareProfile::probe();
        let current_report = self.pool_status()?;

        // Memory preflight check
        let requested_bytes = profile.size_bytes.unwrap_or(0);
        if current_report.total_active_weight_bytes + requested_bytes > hw.absolute_max_weight_bytes
        {
            return Err(LocalRuntimeError::Control(format!(
                "memory budget exceeded: active weight {} B + requested {} B exceeds max budget {} B",
                current_report.total_active_weight_bytes,
                requested_bytes,
                hw.absolute_max_weight_bytes
            )));
        }

        // Check if active
        let active_status = self.runtime.status()?;
        if active_status.state == crate::domain::local_runtime::LocalRuntimeState::Active
            && active_status.alias.as_deref() == Some(alias)
        {
            if let Some(pid) = active_status.main_pid {
                let port = active_status.listen_port.unwrap_or(8080);
                return Ok(ActivePoolSlot {
                    alias: alias.to_string(),
                    port,
                    endpoint: format!("http://127.0.0.1:{port}/v1"),
                    pid,
                    weight_bytes: requested_bytes,
                });
            }
        }

        let mutation = self.runtime.start(alias)?;
        let status = mutation.status;

        let pid = status.main_pid.unwrap_or(0);
        let port = status.listen_port.unwrap_or(8080);
        let endpoint = format!("http://127.0.0.1:{port}/v1");

        let mut ports = self.active_ports.lock().unwrap();
        ports.insert(alias.to_string(), port);

        Ok(ActivePoolSlot {
            alias: alias.to_string(),
            port,
            endpoint,
            pid,
            weight_bytes: requested_bytes,
        })
    }

    /// Report overall pool status and active model slots.
    pub fn pool_status(&self) -> Result<ModelPoolReport, LocalRuntimeError> {
        let inv = self.inventory.list_profiles()?;
        let hw = HardwareProfile::probe();

        let mut active_slots = Vec::new();
        let mut total_active_weight_bytes = 0u64;

        if let Ok(st) = self.runtime.status() {
            if st.state == crate::domain::local_runtime::LocalRuntimeState::Active {
                if let Some(alias) = st.alias.as_deref() {
                    if let Some(prof) = inv.models.iter().find(|p| p.alias == alias) {
                        let port = st.listen_port.unwrap_or(8080);
                        let pid = st.main_pid.unwrap_or(0);
                        let weight = prof.size_bytes.unwrap_or(0);
                        active_slots.push(ActivePoolSlot {
                            alias: prof.alias.clone(),
                            port,
                            endpoint: format!("http://127.0.0.1:{port}/v1"),
                            pid,
                            weight_bytes: weight,
                        });
                        total_active_weight_bytes += weight;
                    }
                }
            }
        }

        Ok(ModelPoolReport {
            schema: ModelPoolReport::SCHEMA,
            status: "ok",
            active_slots,
            total_active_weight_bytes,
            memory_budget_bytes: hw.absolute_max_weight_bytes,
            available_ram_bytes: hw.available_ram_bytes,
        })
    }

    /// Terminate a specific model process from the pool.
    pub fn stop_model(&self, alias: &str) -> Result<(), LocalRuntimeError> {
        self.runtime.stop(Some(alias))?;
        let mut ports = self.active_ports.lock().unwrap();
        ports.remove(alias);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_process_pool_new_and_status() {
        let pool = ModelProcessPool::new();
        assert_eq!(pool.base_port, 8080);
        let port = pool
            .find_available_port(8080)
            .expect("should find free port");
        assert!(port >= 8080);

        let status = pool.pool_status().expect("pool status should succeed");
        assert_eq!(status.schema, "cynic.inference.model_process_pool.v1");
        assert_eq!(status.status, "ok");
    }
}
