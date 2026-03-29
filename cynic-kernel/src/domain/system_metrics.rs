//! Port trait for system-level metrics (CPU, RAM, disk).
//! Domain-pure: no knowledge of sysinfo or any specific implementation.

use async_trait::async_trait;
use std::fmt;

/// Point-in-time snapshot of system resource usage.
#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub cpu_usage_percent: f64,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub disk_available_gb: f64,
    pub disk_total_gb: f64,
    pub load_average_1m: f64,
    pub uptime_seconds: u64,
    pub created_at: String,
}

impl SystemSnapshot {
    /// Compact string for observation context (fits 200-char limit).
    pub fn to_compact(&self) -> String {
        let uptime_display = if self.uptime_seconds >= 86400 {
            format!("{}d", self.uptime_seconds / 86400)
        } else if self.uptime_seconds >= 3600 {
            format!("{}h", self.uptime_seconds / 3600)
        } else {
            format!("{}m", self.uptime_seconds / 60)
        };
        format!(
            "cpu:{:.1}% mem:{:.1}/{:.1}GB disk:{:.0}/{:.0}GB load:{:.2} up:{}",
            self.cpu_usage_percent,
            self.memory_used_gb,
            self.memory_total_gb,
            self.disk_available_gb,
            self.disk_total_gb,
            self.load_average_1m,
            uptime_display,
        )
    }
}

#[derive(Debug)]
pub struct SystemMetricsError(pub String);

impl fmt::Display for SystemMetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "system metrics error: {}", self.0)
    }
}

impl std::error::Error for SystemMetricsError {}

/// Driven port for system-level resource sensing.
/// Implementations: SysinfoMetrics (production), NullMetrics (tests).
#[async_trait]
pub trait SystemMetricsPort: Send + Sync {
    async fn snapshot(&self) -> Result<SystemSnapshot, SystemMetricsError>;
}

/// No-op implementation for tests and environments without system access.
/// Used by introspection tests and as fallback when system probing is unavailable.
#[allow(dead_code)]
#[derive(Debug)]
pub struct NullSystemMetrics;

#[async_trait]
impl SystemMetricsPort for NullSystemMetrics {
    async fn snapshot(&self) -> Result<SystemSnapshot, SystemMetricsError> {
        Err(SystemMetricsError(
            "NullSystemMetrics — no system access".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_format_fits_200_chars() {
        let snap = SystemSnapshot {
            cpu_usage_percent: 42.1,
            memory_used_gb: 11.2,
            memory_total_gb: 15.5,
            disk_available_gb: 120.0,
            disk_total_gb: 500.0,
            load_average_1m: 0.82,
            uptime_seconds: 3 * 86400 + 7200,
            created_at: "2026-03-28T12:00:00Z".into(),
        };
        let compact = snap.to_compact();
        assert!(
            compact.len() <= 200,
            "compact was {} chars: {}",
            compact.len(),
            compact
        );
        assert!(compact.contains("cpu:42.1%"));
        assert!(compact.contains("mem:11.2/15.5GB"));
        assert!(compact.contains("up:3d"));
    }

    #[test]
    fn error_display_and_error_trait() {
        let e = SystemMetricsError("test".into());
        assert!(e.to_string().contains("test"));
        let _: &dyn std::error::Error = &e;
    }

    #[tokio::test]
    async fn null_metrics_returns_err() {
        let null = NullSystemMetrics;
        assert!(null.snapshot().await.is_err());
    }
}
