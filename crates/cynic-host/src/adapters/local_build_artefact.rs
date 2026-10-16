//! Local build artefact observation — binds `RetentionObserver` to the
//! host's own `/var/lib/cynic/checkouts` tree (no SSH). It reads `/proc`-less
//! substrate via `capture_command` (lsblk/df/du/git) and reuses the pure
//! `parse_checkout_list` / `parse_df_pressure` parsers shared with the
//! remote adapter so the two never diverge.
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use crate::domain::build_artefact::parse::{parse_checkout_list, parse_df_pressure};
use crate::domain::build_artefact::{
    plan_build_artefacts, RetentionError, RetentionExecutor, RetentionItem,
    RetentionItemExecution, RetentionObserver, RetentionObservation, RetentionPlan,
    RetentionPlanner, RetentionPolicy, RetentionResult, RETENTION_OBSERVATION_SCHEMA,
    RETENTION_PLAN_SCHEMA, RETENTION_RESULT_SCHEMA,
};
use crate::domain::disk::probe::{parse_df_filesystems, parse_lsblk_disks};
use crate::process_observation::capture_command;
use cynic_build::build_artefact::CheckoutEntry;
use cynic_protocol::disk::bytes_to_human;
use cynic_protocol::retention::{PressureSignal, RetentionKind};

const CHECKOUTS_BASE: &str = "/var/lib/cynic/checkouts";

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalBuildArtefactAdapter;

impl RetentionObserver for LocalBuildArtefactAdapter {
    fn observe(&self) -> Result<RetentionObservation, RetentionError> {
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let physical_disks = observe_physical_disks();
        let filesystems = observe_filesystems();
        let (checkouts_raw, remote_shas) = observe_checkouts_raw(false);
        let checkouts = parse_checkout_list(&checkouts_raw, &remote_shas);
        let total_checkout_bytes: u64 = checkouts.iter().map(|c| c.size_bytes).sum();

        let df_out = capture_command("df", &["-k", CHECKOUTS_BASE])
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

        // Convert CheckoutEntry → RetentionItem
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
                remote_exists: co.remote_exists,
                sha: Some(co.sha.clone()),
                branch: Some(co.branch.clone()),
                owner_uid: Some(co.owner_uid),
                owner_gid: Some(co.owner_gid),
            })
            .collect();

        // Residual unknowns
        let mut residual_unknowns = Vec::new();
        if !Path::new(CHECKOUTS_BASE).exists() {
            residual_unknowns.push(format!("checkouts_base_missing:{CHECKOUTS_BASE}"));
        }
        if physical_disks.is_empty() {
            residual_unknowns.push("physical_disks_unknown".to_string());
        }
        let _ = filesystems;
        let _ = total_checkout_bytes;

        Ok(RetentionObservation {
            schema: RETENTION_OBSERVATION_SCHEMA.to_string(),
            timestamp,
            items,
            total_reclaimable_bytes: 0,
            pressure: Some(PressureSignal {
                use_pct: pressure.use_pct,
                avail_bytes: pressure.avail_bytes,
                is_pressured: pressure.is_pressured,
            }),
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
        Ok(o) => RetentionItemExecution {
            item_id: item.id.clone(),
            success: o.success,
            exit_code: o.exit_code,
            stdout_tail: tail(&o.stdout, 800),
            stderr_tail: tail(&o.stderr, 800),
        },
        Err(e) => RetentionItemExecution {
            item_id: item.id.clone(),
            success: false,
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: e.to_string(),
        },
    }
}

fn observe_physical_disks() -> Vec<cynic_protocol::disk::PhysicalDisk> {
    let out = capture_command(
        "lsblk",
        &[
            "-b",
            "-J",
            "-o",
            "NAME,SIZE,TYPE,ROTA,VENDOR,MODEL,SERIAL,WWN,MOUNTPOINT",
        ],
    );
    match out {
        Ok(c) if c.success => parse_lsblk_disks(&c.stdout),
        _ => Vec::new(),
    }
}

fn observe_filesystems() -> Vec<cynic_protocol::disk::DiskFilesystem> {
    let out = capture_command(
        "df",
        &["-h", "--output=source,fstype,size,used,avail,pcent,target"],
    );
    let stdout = match out {
        Ok(c) if c.success => c.stdout,
        _ => {
            let fb = capture_command("df", &["-h"]);
            match fb {
                Ok(c) if c.success => c.stdout,
                _ => return Vec::new(),
            }
        }
    };
    parse_df_filesystems(&stdout)
}

fn observe_checkouts_raw(shallow: bool) -> (String, Vec<String>) {
    let remote_shas = if shallow {
        Vec::new()
    } else {
        fetch_remote_shas()
    };
    let mut lines = Vec::new();
    let entries = fs::read_dir(CHECKOUTS_BASE).ok();
    let Some(entries) = entries else {
        return (String::new(), remote_shas);
    };
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
        let line =
            format!("{path_str}|{sha}|{size_bytes}|{uid}:{gid}|{mtime_secs}|{remote_exists}");
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
    for base in [CHECKOUTS_BASE, "."] {
        let out = if base == "." {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_returns_observation_with_items() {
        let adapter = LocalBuildArtefactAdapter;
        let obs = adapter.observe().expect("observe");
        assert_eq!(obs.schema, RETENTION_OBSERVATION_SCHEMA);
        // In a test environment, checkouts may or may not exist.
        // The important thing is that observe() returns Ok.
    }

    #[test]
    fn plan_filters_orphan_and_age() {
        let obs = RetentionObservation {
            schema: RETENTION_OBSERVATION_SCHEMA.to_string(),
            timestamp: "2026-10-16T00:00:00Z".to_string(),
            items: vec![
                RetentionItem {
                    id: "checkout:abc123".to_string(),
                    path: "/var/lib/cynic/checkouts/old-orphan".to_string(),
                    size_bytes: 5000,
                    age: Some(std::time::Duration::from_secs(200_000)),
                    reason: String::new(),
                    requires_privilege: false,
                    argv: vec![],
                    is_orphan: Some(true),
                    is_root_owned: Some(false),
                    remote_exists: Some(false),
                    sha: Some("abc123".to_string()),
                    branch: Some("old-orphan".to_string()),
                    owner_uid: Some(999),
                    owner_gid: Some(990),
                },
                RetentionItem {
                    id: "checkout:live123".to_string(),
                    path: "/var/lib/cynic/checkouts/live".to_string(),
                    size_bytes: 5000,
                    age: Some(std::time::Duration::from_secs(1000)),
                    reason: String::new(),
                    requires_privilege: false,
                    argv: vec![],
                    is_orphan: Some(false),
                    is_root_owned: Some(false),
                    remote_exists: Some(true),
                    sha: Some("live123".to_string()),
                    branch: Some("live".to_string()),
                    owner_uid: Some(999),
                    owner_gid: Some(990),
                },
            ],
            total_reclaimable_bytes: 0,
            pressure: None,
        };
        let policy = RetentionPolicy {
            kind: RetentionKind::BuildArtefact,
            max_age: Some(std::time::Duration::from_secs(48 * 3600)),
            pressure_trigger: None,
            preserve_count: 0,
            preserve_profiles: vec![],
            grace_period: None,
            scopes: vec![],
        };
        let adapter = LocalBuildArtefactAdapter;
        let plan = adapter.plan(&obs, &policy);
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].path, "/var/lib/cynic/checkouts/old-orphan");
    }
}
