//! Meter ports — traits for measuring resource consumption.
//!
//! These traits are implemented by adapters in `cynic-host` (L1),
//! `cynic-host-probes` (L1), `cynic-network` (L1), `cynic-storage` (L2),
//! or `cynic-node` (L3). The domain crate stays pure.

use thiserror::Error;

/// Errors that can occur when measuring resources.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum MeterError {
    #[error("meter unavailable: {0}")]
    Unavailable(String),
    #[error("measurement unknown: {0}")]
    Unknown(String),
    #[error("meter error: {0}")]
    Error(String),
}

/// Measures CPU compute consumption.
pub trait ComputeMeter {
    /// Measures CPU time in milliseconds for a given agent.
    fn measure_cpu_ms(&self, agent_id: &str) -> Result<u64, MeterError>;
}

/// Measures GPU compute consumption (Vulkan, ROCm, CUDA).
pub trait GpuMeter {
    /// Measures GPU time in milliseconds for a given agent.
    fn measure_gpu_ms(&self, agent_id: &str) -> Result<u64, MeterError>;
    /// Measures VRAM usage in bytes for a given agent.
    fn measure_vram_bytes(&self, agent_id: &str) -> Result<u64, MeterError>;
    /// Measures GPU utilization as a fraction (0.0 - 1.0).
    fn measure_gpu_utilization(&self, agent_id: &str) -> Result<f64, MeterError>;
}

/// Measures storage consumption.
pub trait StorageMeter {
    /// Measures storage usage in bytes for a given agent.
    fn measure_bytes(&self, agent_id: &str) -> Result<u64, MeterError>;
    /// Measures storage IOPS for a given agent.
    fn measure_iops(&self, agent_id: &str) -> Result<u64, MeterError>;
}

/// Measures network consumption.
pub trait NetworkMeter {
    /// Measures network bytes transferred for a given agent.
    fn measure_bytes(&self, agent_id: &str) -> Result<u64, MeterError>;
    /// Measures network packets transferred for a given agent.
    fn measure_packets(&self, agent_id: &str) -> Result<u64, MeterError>;
}

/// Measures energy consumption.
pub trait EnergyMeter {
    /// Measures power draw in watts for a given agent.
    fn measure_watts(&self, agent_id: &str) -> Result<f64, MeterError>;
    /// Measures energy consumption in kWh for a given agent.
    fn measure_kwh(&self, agent_id: &str) -> Result<f64, MeterError>;
}

/// Measures token (CYD) balances.
pub trait TokenMeter {
    /// Measures CYD balance for a given agent.
    fn measure_cyd(&self, agent_id: &str) -> Result<u64, MeterError>;
}

/// Measures LLM context token consumption.
pub trait ContextMeter {
    /// Measures context tokens consumed for a given agent.
    fn measure_tokens(&self, agent_id: &str) -> Result<u64, MeterError>;
}

/// Measures time consumption.
pub trait TimeMeter {
    /// Measures wall-clock time in seconds for a given agent.
    fn measure_seconds(&self, agent_id: &str) -> Result<u64, MeterError>;
}

/// Measures action count.
pub trait ActionMeter {
    /// Measures number of actions performed by a given agent.
    fn measure_actions(&self, agent_id: &str) -> Result<u64, MeterError>;
}
