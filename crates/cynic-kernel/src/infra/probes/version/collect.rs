//! Collection logic for version observations.

use std::time::Duration;

use crate::domain::probe::DependencyVersion;

use super::config::{DependencySpec, GITHUB_API};

pub(super) fn normalize_version(raw: &str) -> String {
    let raw = raw.trim();

    for prefix in &["v", "b", "Version ", "version "] {
        if let Some(rest) = raw.strip_prefix(prefix) {
            return rest.split_whitespace().next().unwrap_or(rest).to_string();
        }
    }

    for token in raw.split_whitespace().rev() {
        if token.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return token.to_string();
        }
    }

    raw.to_string()
}

pub(super) async fn fetch_latest(client: &reqwest::Client, repo: &str) -> Option<String> {
    let url = format!("{GITHUB_API}/{repo}/releases/latest");
    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        return None;
    }

    let body: serde_json::Value = response.json().await.ok()?;
    body.get("tag_name")?.as_str().map(normalize_version)
}

pub(super) async fn detect_running(cmd: &[&str]) -> Option<String> {
    if cmd.is_empty() {
        return None;
    }

    let binary = cmd[0];
    let args = &cmd[1..];

    let output = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::process::Command::new(binary)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/usr/local/bin:/bin")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    let first_line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();

    if first_line.is_empty() {
        None
    } else {
        Some(first_line)
    }
}

pub(super) async fn collect_dependencies(
    client: &reqwest::Client,
    tracked: &'static [DependencySpec],
) -> (Vec<DependencyVersion>, u32, u32) {
    let mut dependencies = Vec::with_capacity(tracked.len());
    let mut behind_count = 0u32;
    let mut unknown_count = 0u32;

    for dep in tracked {
        let running = match dep.version_cmd {
            Some(cmd) => detect_running(cmd)
                .await
                .map(|value| normalize_version(&value)),
            None => None,
        };

        let latest = fetch_latest(client, dep.repo)
            .await
            .map(|tag| normalize_version(&tag));

        let behind = match (&running, &latest) {
            (Some(running), Some(latest)) => {
                let is_behind = running != latest;
                if is_behind {
                    behind_count += 1;
                }
                is_behind
            }
            _ => false,
        };

        if running.is_none() && dep.version_cmd.is_some() {
            unknown_count += 1;
        }

        dependencies.push(DependencyVersion {
            name: dep.name.to_string(),
            running,
            latest,
            behind,
            versions_behind: if behind { 1 } else { 0 },
            source: dep.repo.to_string(),
        });
    }

    (dependencies, behind_count, unknown_count)
}
