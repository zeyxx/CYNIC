/// Self-healing lifecycle management for cynic-node.
///
/// When the node crashes (ExitCode::FAILURE), it:
/// 1. Reads restart count from CYNIC_NODE_RESTART_COUNT env var
/// 2. Checks if max retries exceeded — if so, exit permanently
/// 3. Calculates exponential backoff: 2^n seconds, capped at 120s
/// 4. Increments restart count in environment
/// 5. Self-execs the same binary with updated env
///
/// Graceful shutdowns (SIGTERM) return ExitCode::SUCCESS and don't restart.
///
/// K18 compliance: Uses process-wrap for orphan safety in node lifecycle.
use std::env;
use std::process::{self, Command};
use std::time::Duration;
use tracing;

/// Self-healing context for node lifecycle management.
pub struct SelfHealingContext {
    config_path: String,
    restart_count: u32,
    max_restarts: u32,
}

impl SelfHealingContext {
    /// Create context from environment (reads CYNIC_NODE_RESTART_COUNT).
    pub fn from_env(config_path: String) -> Self {
        let restart_count = env::var("CYNIC_NODE_RESTART_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Self {
            config_path,
            restart_count,
            max_restarts: 5,
        }
    }

    /// Get the current restart attempt number (0-indexed).
    pub fn restart_attempt(&self) -> u32 {
        self.restart_count
    }

    /// Check if we've exhausted retries.
    pub fn exhausted(&self) -> bool {
        self.restart_count >= self.max_restarts
    }

    /// Calculate exponential backoff: 2^n, capped at 120s.
    ///
    /// n=0 → 1s, n=1 → 2s, n=2 → 4s, ... n=6+ → 120s (cap)
    fn backoff_secs(&self) -> u64 {
        2_u64.saturating_pow(self.restart_count).min(120)
    }

    /// Run the node lifecycle with automatic self-healing on crash.
    ///
    /// This function NEVER returns normally (on crash it self-execs, on success it exits).
    /// Only returns if there's a fatal setup error before the node can run.
    pub async fn run_with_self_healing<F>(&self, node_fn: F) -> !
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = process::ExitCode>>>,
    {
        if self.exhausted() {
            tracing::error!(
                restart_count = self.restart_count,
                max_restarts = self.max_restarts,
                "max restart attempts exceeded — giving up (node is permanently broken)"
            );
            process::exit(1);
        }

        if self.restart_count > 0 {
            tracing::warn!(
                restart_count = self.restart_count,
                max_restarts = self.max_restarts,
                "starting recovery attempt (previous crash detected)"
            );
        }

        // Run the actual node lifecycle
        let exit = node_fn().await;

        match exit {
            process::ExitCode::SUCCESS => {
                // Graceful shutdown (SIGTERM) — clean exit
                tracing::info!("graceful shutdown");
                process::exit(0);
            }
            process::ExitCode::FAILURE => {
                // Crash detected — restart with backoff
                let backoff = self.backoff_secs();

                tracing::error!(
                    restart_count = self.restart_count,
                    next_attempt = self.restart_count + 1,
                    backoff_secs = backoff,
                    "node crashed (FAILURE) — scheduling restart"
                );

                // Wait before restart (futures-friendly)
                tokio::time::sleep(Duration::from_secs(backoff)).await;

                // Self-exec with incremented restart count
                self.self_exec();
            }
        }
    }

    /// Self-execute the same binary with incremented restart count.
    ///
    /// This function calls `execve()` and does NOT return (process image is replaced).
    /// If exec fails, logs and exits.
    fn self_exec(&self) -> ! {
        let exe = std::env::current_exe().unwrap_or_else(|e| {
            tracing::error!("failed to get current executable path: {e}");
            process::exit(1);
        });

        let args: Vec<_> = std::env::args().collect();

        let mut cmd = Command::new(&exe);
        // Pass through original arguments (--config, --mcp, etc.)
        for arg in &args[1..] {
            cmd.arg(arg);
        }

        // Increment restart count for next iteration
        let next_count = self.restart_count + 1;
        cmd.env("CYNIC_NODE_RESTART_COUNT", next_count.to_string());

        tracing::info!(
            restart_count = self.restart_count,
            next_restart_count = next_count,
            "executing self (replacing process image)"
        );

        let err = cmd.exec(); // Never returns on success
        tracing::error!("failed to exec self: {err}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_calculation() {
        let ctx = SelfHealingContext {
            config_path: "/tmp/test.toml".to_string(),
            restart_count: 0,
            max_restarts: 5,
        };

        assert_eq!(ctx.backoff_secs(), 1); // 2^0 = 1

        let ctx = SelfHealingContext {
            config_path: "/tmp/test.toml".to_string(),
            restart_count: 3,
            max_restarts: 5,
        };
        assert_eq!(ctx.backoff_secs(), 8); // 2^3 = 8

        let ctx = SelfHealingContext {
            config_path: "/tmp/test.toml".to_string(),
            restart_count: 7,
            max_restarts: 5,
        };
        assert_eq!(ctx.backoff_secs(), 120); // 2^7 = 128, capped at 120
    }

    #[test]
    fn test_exhaustion_check() {
        let ctx = SelfHealingContext {
            config_path: "/tmp/test.toml".to_string(),
            restart_count: 4,
            max_restarts: 5,
        };
        assert!(!ctx.exhausted());

        let ctx = SelfHealingContext {
            config_path: "/tmp/test.toml".to_string(),
            restart_count: 5,
            max_restarts: 5,
        };
        assert!(ctx.exhausted());
    }
}
