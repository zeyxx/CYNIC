//! Local build artefact observation — binds `RetentionObserver` to the
//! host's own `/var/lib/cynic/checkouts` tree (no SSH).
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use cynic_protocol::build::{parse_checkout_list, parse_df_pressure};
use crate::build_artefact::{
    plan_build_artefacts, RETENTION_OBSERVATION_SCHEMA, RETENTION_RESULT_SCHEMA, RetentionError,
    RetentionExecutor, RetentionItem, RetentionItemExecution, RetentionObservation,
    RetentionObserver, RetentionPlan, RetentionPlanner, RetentionPolicy, RetentionResult,
};
use cynic_protocol::retention::PressureSignal;
use cynic_types::paths::CynicPaths;
use crate::adapters::live_service_paths::{is_live_service_path, observe_unit_executables};

fn checkouts_base() -> PathBuf {
    CynicPaths::state_dir().join("checkouts")
}

pub fn bytes_to_human(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1}T", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}

pub struct CaptureOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

fn capture_command(bin: &str, args: &[&str]) -> Result<CaptureOutput, String> {
    let mut cmd = std::process::Command::new(bin);
    cmd.args(args);
    let output = cmd.output().map_err(|e| format!("exec error: {e}"))?;
    Ok(CaptureOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
    })
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalBuildArtefactAdapter;

impl RetentionObserver for LocalBuildArtefactAdapter {
    fn observe(&self) -> Result<RetentionObservation, RetentionError> {
        Self::observe_inner(false)
    }
}

impl LocalBuildArtefactAdapter {
    /// Classification-only observation: skips recursive `du` and network
    /// `ls-remote` (sizes read 0, `remote_exists` unset). For consumers
    /// that need orphan/age classification but not sizes.
    pub fn observe_shallow() -> Result<RetentionObservation, RetentionError> {
        Self::observe_inner(true)
    }

    fn observe_inner(shallow: bool) -> Result<RetentionObservation, RetentionError> {
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let (checkouts_raw, remote_shas) = observe_checkouts_raw(shallow);
        let checkouts = parse_checkout_list(&checkouts_raw, &remote_shas);
        let unit_executables = observe_unit_executables();
        let checkouts_base = checkouts_base();
        let df_out = capture_command("df", &["-k", &checkouts_base.to_string_lossy()])
            .ok()
            .filter(|c| c.success)
            .map(|c| c.stdout)
            .or_else(|| {
                capture_command("df", &["-k", "/"])
                    .ok()
                    .filter(|c| c.success)
                    .map(|c| c.stdout)
            })
            .unwrap_or_default();
        let root_owned = checkouts.iter().filter(|c| c.is_root_owned).count();
        let pressure = parse_df_pressure(&df_out, root_owned);

        let items: Vec<RetentionItem> = checkouts
            .iter()
            .map(|co| RetentionItem {
                id: format!("checkout:{}", co.sha.chars().take(8).collect::<String>()),
                path: co.path.clone(),
                size_bytes: co.size_bytes,
                age: co.age_secs.map(std::time::Duration::from_secs),
                reason: String::new(),
                requires_privilege: co.is_root_owned,
                argv: vec![],
                is_orphan: Some(co.is_orphan),
                is_root_owned: Some(co.is_root_owned),
                is_live_service_path: Some(
                    is_live_service_path(
                        std::path::Path::new(&co.path),
                        &unit_executables,
                    ),
                ),
                remote_exists: co.remote_exists,
                sha: Some(co.sha.clone()),
                branch: Some(co.branch.clone()),
                owner_uid: Some(co.owner_uid),
                owner_gid: Some(co.owner_gid),
            })
            .collect();

        Ok(RetentionObservation {
            schema: RETENTION_OBSERVATION_SCHEMA.to_string(),
            timestamp,
            items,
            total_reclaimable_bytes: 0,
            pressure: Some(pressure),
        })
    }
}

impl RetentionPlanner for LocalBuildArtefactAdapter {
    fn plan(&self, obs: &RetentionObservation, policy: &RetentionPolicy) -> RetentionPlan {
        plan_build_artefacts(obs, policy)
    }
}

impl RetentionExecutor for LocalBuildArtefactAdapter {
    fn execute(&self, plan: &RetentionPlan) -> Result<RetentionResult, RetentionError> {
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let mut items_executed = Vec::new();
        let mut total_reclaimed_bytes = 0u64;
        let mut success = true;

        for item in &plan.items {
            let exec = execute_gc_item(item);
            if exec.success {
                total_reclaimed_bytes += item.size_bytes;
            } else {
                success = false;
            }
            items_executed.push(exec);
        }

        Ok(RetentionResult {
            schema: RETENTION_RESULT_SCHEMA.to_string(),
            timestamp,
            success,
            items_executed,
            total_reclaimed_bytes,
        })
    }
}

fn execute_gc_item(item: &RetentionItem) -> RetentionItemExecution {
    if item.argv.is_empty() {
        return RetentionItemExecution {
            item_id: item.id.clone(),
            success: false,
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: "empty argv".to_string(),
        };
    }
    let (bin, args): (&str, Vec<&str>) = if item.requires_privilege {
        ("sudo", item.argv.iter().map(|s| s.as_str()).collect())
    } else {
        (
            item.argv[0].as_str(),
            item.argv[1..].iter().map(|s| s.as_str()).collect(),
        )
    };
    match capture_command(bin, &args) {
        Ok(c) => RetentionItemExecution {
            item_id: item.id.clone(),
            success: c.success,
            exit_code: c.exit_code,
            stdout_tail: tail(&c.stdout, 512),
            stderr_tail: tail(&c.stderr, 512),
        },
        Err(e) => RetentionItemExecution {
            item_id: item.id.clone(),
            success: false,
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: e,
        },
    }
}

fn observe_checkouts_raw(shallow: bool) -> (String, Vec<String>) {
    let checkouts = checkouts_base();
    let entries = match fs::read_dir(&checkouts) {
        Ok(e) => e,
        Err(_) => return (String::new(), Vec::new()),
    };
    let remote_shas = if shallow {
        Vec::new()
    } else {
        fetch_remote_shas()
    };
    let mut lines = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let path_str = path.display().to_string();
        let sha = git_sha(&path).unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        });
        let (size_bytes, uid, gid, mtime_secs) = file_meta(&path, shallow);
        let remote_exists = if remote_shas.contains(&sha) { "1" } else { "0" };
        let foreign = if contains_foreign_owned_entries(&path, uid) {
            "1"
        } else {
            "0"
        };
        let line = format!(
            "{path_str}|{sha}|{size_bytes}|{uid}:{gid}|{mtime_secs}|{remote_exists}|{foreign}"
        );
        lines.push(line);
    }
    (lines.join("\n"), remote_shas)
}

fn git_sha(path: &Path) -> Option<String> {
    let out = capture_command(
        "git",
        &["-C", &path.display().to_string(), "rev-parse", "HEAD"],
    )
    .ok()?;
    if out.success && !out.stdout.is_empty() {
        Some(out.stdout.trim().to_string())
    } else {
        None
    }
}

fn file_meta(path: &Path, shallow: bool) -> (u64, u32, u32, u64) {
    let meta = fs::metadata(path).ok();
    let (uid, gid) = meta
        .as_ref()
        .map(|m| (m.uid(), m.gid()))
        .unwrap_or((999, 990));
    let mtime_secs = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| {
            (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs())
            .saturating_sub(d.as_secs())
        })
        .unwrap_or(0);
    let size_bytes = if shallow {
        0
    } else {
        capture_command("du", &["-sb", &path.display().to_string()])
            .ok()
            .and_then(|c| {
                if c.success {
                    c.stdout.split_whitespace().next()?.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| walk_size(path))
    };
    (size_bytes, uid, gid, mtime_secs)
}

fn contains_foreign_owned_entries(path: &Path, owner_uid: u32) -> bool {
    let mut stack = vec![path.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let Ok(entries) = fs::read_dir(&cur) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.file_name().and_then(|n| n.to_str()) == Some(".git") {
                continue;
            }
            if let Ok(m) = e.metadata() {
                if m.uid() != owner_uid {
                    return true;
                }
            }
            if p.is_dir() && !p.is_symlink() {
                stack.push(p);
            }
        }
    }
    false
}

fn walk_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(cur) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&cur) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_symlink() {
                    continue;
                }
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(m) = e.metadata() {
                    total += m.len();
                }
            }
        }
    }
    total
}

fn fetch_remote_shas() -> Vec<String> {
    let checkouts = checkouts_base().to_string_lossy().into_owned();
    for base in [checkouts.as_str(), "."].iter() {
        let out = if *base == "." {
            capture_command("git", &["ls-remote", "origin"]).ok()
        } else {
            let mut found = None;
            if let Ok(entries) = fs::read_dir(base) {
                for e in entries.flatten() {
                    let p = e.path().join(".git");
                    if p.exists() {
                        found = Some(e.path().display().to_string());
                        break;
                    }
                }
            }
            if let Some(path) = found {
                capture_command("git", &["-C", &path, "ls-remote", "origin"]).ok()
            } else {
                capture_command("git", &["ls-remote", "origin"]).ok()
            }
        };
        if let Some(c) = out {
            if c.success && !c.stdout.trim().is_empty() {
                return c
                    .stdout
                    .lines()
                    .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
                    .collect();
            }
        }
    }
    Vec::new()
}

fn tail(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        s[s.len() - n..].to_string()
    }
}
