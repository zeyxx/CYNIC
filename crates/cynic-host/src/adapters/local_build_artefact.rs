//! Local build artefact observation — binds `BuildArtefactObservationPort` to the
//! host's own `/var/lib/cynic/checkouts` tree (no SSH). It reads `/proc`-less
//! substrate via `capture_command` (lsblk/df/du/git) and reuses the pure
//! `parse_checkout_list` / `parse_df_pressure` / `build_gc_plan` parsers shared
//! with the remote adapter so the two never diverge.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use crate::domain::build_artefact::port::{
    BuildArtefactGcExecutionPort, BuildArtefactObservationPort,
};
use crate::domain::build_artefact::probe::{build_gc_plan, parse_checkout_list, parse_df_pressure};
use crate::domain::disk::probe::{parse_df_filesystems, parse_lsblk_disks};
use crate::process_observation::capture_command;
use cynic_protocol::build_artefact::{
    BUILD_ARTEFACT_GC_RESULT_SCHEMA, BUILD_ARTEFACT_REPORT_SCHEMA, BuildArtefactGcPlan,
    BuildArtefactGcResult, BuildArtefactGcStepExecution, BuildArtefactReport,
};
use cynic_protocol::disk::bytes_to_human;

const CHECKOUTS_BASE: &str = "/var/lib/cynic/checkouts";

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalBuildArtefactAdapter;

impl BuildArtefactObservationPort for LocalBuildArtefactAdapter {
    fn observe_build_artefacts(&self) -> Result<BuildArtefactReport, String> {
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let physical_disks = observe_physical_disks();
        let filesystems = observe_filesystems();
        let (checkouts_raw, remote_shas) = observe_checkouts_raw();
        let checkouts = parse_checkout_list(&checkouts_raw, &remote_shas);
        let total_checkout_bytes: u64 = checkouts.iter().map(|c| c.size_bytes).sum();
        let total_checkout_human = bytes_to_human(total_checkout_bytes);
        // target/ size is rolled into checkouts for now - separate once builder exposes it
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

        // residual unknowns: missing base or unparsable lsblk
        let mut residual_unknowns = Vec::new();
        if !Path::new(CHECKOUTS_BASE).exists() {
            residual_unknowns.push(format!("checkouts_base_missing:{CHECKOUTS_BASE}"));
        }
        if physical_disks.is_empty() {
            // not fatal - may be container without lsblk
            residual_unknowns.push("physical_disks_unknown".to_string());
        }

        Ok(BuildArtefactReport {
            schema: BUILD_ARTEFACT_REPORT_SCHEMA.to_string(),
            timestamp,
            physical_disks,
            filesystems,
            checkouts,
            total_checkout_bytes,
            total_checkout_human,
            target_bytes: 0,
            target_human: "0B".to_string(),
            pressure,
            residual_unknowns,
        })
    }

    fn plan_gc(&self, report: &BuildArtefactReport) -> BuildArtefactGcPlan {
        build_gc_plan(report)
    }
}

impl BuildArtefactGcExecutionPort for LocalBuildArtefactAdapter {
    fn execute_gc(&self, plan: &BuildArtefactGcPlan) -> BuildArtefactGcResult {
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

        BuildArtefactGcResult {
            schema: BUILD_ARTEFACT_GC_RESULT_SCHEMA.to_string(),
            timestamp,
            success,
            items_executed,
            total_reclaimed_bytes,
            total_reclaimed_human: bytes_to_human(total_reclaimed_bytes),
        }
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

fn observe_checkouts_raw() -> (String, Vec<String>) {
    // Collect a pipe of "<path>|<sha>|<size>|<uid:gid>|<mtime_secs>|<remote 0/1>"
    // then feed to pure parse_checkout_list. Remote shas come from one batch ls-remote.
    let remote_shas = fetch_remote_shas();
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
        let (size_bytes, uid, gid, mtime_secs) = file_meta(&path);
        let remote_exists = if remote_shas.contains(&sha) { "1" } else { "0" };
        // if remote_shas empty (no origin), leave flag but pure parser will fallback to list check
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

fn file_meta(path: &Path) -> (u64, u32, u32, u64) {
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
                .as_secs()
                .saturating_sub(d.as_secs()))
        })
        .unwrap_or(0);
    // size via du -sb if available, else walk
    let size_bytes = capture_command("du", &["-sb", &path.display().to_string()])
        .ok()
        .and_then(|c| {
            if c.success {
                c.stdout.split_whitespace().next()?.parse::<u64>().ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| walk_size(path));
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
    // Try to get remote shas from origin in the checkouts base or current repo
    let out = capture_command("git", &["ls-remote", "origin"]).ok();
    let stdout = match out {
        Some(c) if c.success => c.stdout,
        _ => {
            // fallback: try git in CHECKOUTS_BASE sibling (the host repo)
            return Vec::new();
        }
    };
    stdout
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .collect()
}

fn execute_gc_item(
    item: &cynic_protocol::build_artefact::BuildArtefactGcItem,
) -> BuildArtefactGcStepExecution {
    if item.argv.is_empty() {
        return BuildArtefactGcStepExecution {
            item_id: item.id.clone(),
            label: item.label.clone(),
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
        Ok(o) => BuildArtefactGcStepExecution {
            item_id: item.id.clone(),
            label: item.label.clone(),
            success: o.success,
            exit_code: o.exit_code,
            stdout_tail: tail(&o.stdout, 800),
            stderr_tail: tail(&o.stderr, 800),
        },
        Err(e) => BuildArtefactGcStepExecution {
            item_id: item.id.clone(),
            label: item.label.clone(),
            success: false,
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: e.to_string(),
        },
    }
}

fn tail(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        s[s.len() - n..].to_string()
    }
}
