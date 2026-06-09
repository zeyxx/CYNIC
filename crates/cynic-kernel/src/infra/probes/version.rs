//! VersionProbe — supply chain self-awareness.
//!
//! The kernel learns what versions of its dependencies are running
//! and whether newer versions exist upstream. This is proprioception
//! for the supply chain: the organism knowing its own body's composition.
//!
//! Tracked dependencies:
//! - llama.cpp (inference engine, sovereign GPU + CPU)
//! - tailscale (mesh network)
//! - surrealdb (persistence)
//! - cynic-kernel itself (from compile-time version)
//!
//! Data sources:
//! - Running versions: subprocess `--version` or HTTP API
//! - Latest versions: GitHub Releases API (rate-limited, cached)
//!
//! Interval: 1 hour (upstream releases don't change every 30s).

use async_trait::async_trait;
use std::time::Duration;

use crate::domain::probe::{
    DependencyVersion, ProbeDetails, ProbeError, ProbeResult, ProbeStatus, VersionDetails,
};

/// GitHub Releases API endpoint pattern.
const GITHUB_API: &str = "https://api.github.com/repos";

/// Dependencies to track: (name, github_repo, version_command).
/// version_command is None for deps checked via HTTP API instead of subprocess.
const TRACKED: &[(&str, &str, Option<&[&str]>)] = &[
    (
        "llama.cpp",
        "ggml-org/llama.cpp",
        None, // checked via fleet probe HTTP, not subprocess
    ),
    (
        "tailscale",
        "tailscale/tailscale",
        Some(&["tailscale", "version"]),
    ),
    (
        "surrealdb",
        "surrealdb/surrealdb",
        Some(&["surreal", "version"]),
    ),
];

#[derive(Debug)]
pub struct VersionProbe {
    client: reqwest::Client,
}

impl Default for VersionProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionProbe {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent("cynic-kernel/version-probe")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }

    /// Poll GitHub Releases API for the latest tag.
    async fn fetch_latest(&self, repo: &str) -> Option<String> {
        let url = format!("{GITHUB_API}/{repo}/releases/latest");
        let resp = self.client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: serde_json::Value = resp.json().await.ok()?;
        let tag = body.get("tag_name")?.as_str()?;
        Some(tag.to_string())
    }

    /// Detect the running version of a local binary via subprocess.
    async fn detect_running(args: &[&str]) -> Option<String> {
        if args.is_empty() {
            return None;
        }
        let binary = args[0];
        let cmd_args = &args[1..];

        // R23: explicit env, not inherited
        let output = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::process::Command::new(binary)
                .args(cmd_args)
                .env_clear()
                .env("PATH", "/usr/bin:/usr/local/bin:/bin")
                .kill_on_drop(true) // K18
                .output(),
        )
        .await
        .ok()?
        .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next()?.trim().to_string();
        if first_line.is_empty() {
            return None;
        }
        Some(first_line)
    }

    /// Extract a semver-like version from a string.
    /// Handles formats like "v1.2.3", "b1234", "surrealdb 3.0.3", "1.82.1".
    fn extract_version(raw: &str) -> String {
        // Try to find a version pattern in the string
        let raw = raw.trim();
        // Remove common prefixes
        for prefix in &["v", "b", "Version ", "version "] {
            if let Some(rest) = raw.strip_prefix(prefix) {
                return rest.split_whitespace().next().unwrap_or(rest).to_string();
            }
        }
        // Take the last whitespace-separated token that looks like a version
        for token in raw.split_whitespace().rev() {
            if token.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return token.to_string();
            }
        }
        raw.to_string()
    }
}

#[async_trait]
impl crate::domain::probe::Probe for VersionProbe {
    fn name(&self) -> &str {
        "version"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(3600) // 1 hour — upstream releases are slow
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = std::time::Instant::now();
        let mut dependencies = Vec::new();
        let mut behind_count = 0u32;
        let mut unknown_count = 0u32;

        // Kernel version (compile-time)
        let kernel_version = env!("CARGO_PKG_VERSION").to_string();
        dependencies.push(DependencyVersion {
            name: "cynic-kernel".to_string(),
            running: Some(kernel_version),
            latest: None, // no upstream to compare — we ARE the upstream
            behind: false,
            versions_behind: 0,
            source: "zeyxx/CYNIC".to_string(),
        });

        // Track each dependency
        for &(dep_name, repo, version_cmd) in TRACKED {
            let running = match version_cmd {
                Some(args) => Self::detect_running(args)
                    .await
                    .map(|v| Self::extract_version(&v)),
                None => None, // HTTP-based deps (llama.cpp) — skip local detection
            };

            let latest = self
                .fetch_latest(repo)
                .await
                .map(|v| Self::extract_version(&v));

            let behind = match (&running, &latest) {
                (Some(r), Some(l)) => r != l,
                _ => false,
            };

            if running.is_none() && version_cmd.is_some() {
                unknown_count += 1;
            }
            if behind {
                behind_count += 1;
            }

            dependencies.push(DependencyVersion {
                name: dep_name.to_string(),
                running,
                latest,
                behind,
                versions_behind: if behind { 1 } else { 0 }, // coarse — semver diff is future work
                source: repo.to_string(),
            });
        }

        let status = if unknown_count > 0 {
            ProbeStatus::Degraded
        } else {
            ProbeStatus::Ok
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ProbeResult {
            name: self.name().to_string(),
            status,
            details: ProbeDetails::Version(VersionDetails {
                dependencies,
                behind_count,
                unknown_count,
            }),
            duration_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_handles_common_formats() {
        assert_eq!(VersionProbe::extract_version("v1.82.1"), "1.82.1");
        assert_eq!(VersionProbe::extract_version("b9253"), "9253");
        assert_eq!(
            VersionProbe::extract_version("surrealdb 3.0.3 for linux on x86_64"),
            "3.0.3"
        );
        assert_eq!(VersionProbe::extract_version("1.82.1"), "1.82.1");
        assert_eq!(VersionProbe::extract_version("Version 0.7.7"), "0.7.7");
    }

    #[test]
    fn kernel_version_is_available() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }
}
