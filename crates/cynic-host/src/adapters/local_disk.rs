//! Local disk space observation and cleanup adapter.

use crate::fleet_disk_observer::HOST_DISK_REPORT_SCHEMA;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use crate::domain::disk::port::{DiskCleanupExecutionPort, DiskObservationPort};
use crate::domain::disk::probe::{
    ROTATED_SYSLOG_FILES, build_cleanup_plan, parse_df_filesystems, parse_lsblk_disks,
};
use crate::domain::disk::{
    DiskCacheEntry, DiskCleanupItem, DiskCleanupPlan, DiskCleanupResult, DiskCleanupStepExecution,
    DiskFileEntry, DiskFilesystem, DiskReport, PhysicalDisk, bytes_to_human,
};
use crate::process_observation::capture_command;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalDiskAdapter;

impl DiskObservationPort for LocalDiskAdapter {
    fn observe_disk(&self) -> Result<DiskReport, String> {
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let filesystems = observe_filesystems()?;
        let known_caches = observe_known_caches();
        let large_files = observe_large_files();
        let physical_disks = observe_physical_disks();

        Ok(DiskReport {
            schema: HOST_DISK_REPORT_SCHEMA.to_string(),
            timestamp,
            filesystems,
            known_caches,
            large_files,
            physical_disks,
        })
    }

    fn plan_cleanup(&self, report: &DiskReport) -> DiskCleanupPlan {
        build_cleanup_plan(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_observation::capture_command;

    /// Integration check: real `lsblk` spawn + parser must agree on at least one
    /// device when run on a host that actually has block devices. This isolates
    /// `capture_command("lsblk")` (path/timeout) from `parse_lsblk_disks` (json).
    #[test]
    fn real_lsblk_capture_feeds_parser() {
        let out = capture_command(
            "lsblk",
            &[
                "-b",
                "-J",
                "-o",
                "NAME,SIZE,TYPE,ROTA,VENDOR,MODEL,SERIAL,WWN,MOUNTPOINT",
            ],
        );
        let captured = match out {
            Ok(res) if res.success => res.stdout,
            other => panic!("lsblk capture failed (spawn or timeout): {other:?}"),
        };
        let disks = parse_lsblk_disks(&captured);
        // This host has at least one block device (loop or disk).
        assert!(
            !disks.is_empty(),
            "lsblk produced stdout of {} bytes but parser found 0 devices",
            captured.len()
        );
    }
}

impl DiskCleanupExecutionPort for LocalDiskAdapter {
    fn execute_cleanup(&self, plan: &DiskCleanupPlan) -> DiskCleanupResult {
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let mut items_executed = Vec::new();
        let mut total_reclaimed_bytes = 0u64;
        let mut success = true;

        for item in &plan.items {
            let start_bytes = get_disk_free_bytes().unwrap_or(0);
            let execution = execute_cleanup_item(item);
            let end_bytes = get_disk_free_bytes().unwrap_or(0);

            let reclaimed = end_bytes.saturating_sub(start_bytes);
            if execution.success {
                // If we reclaimed space, use the measured value, otherwise fall back to estimated size
                total_reclaimed_bytes += if reclaimed > 0 {
                    reclaimed
                } else {
                    item.size_bytes
                };
            } else {
                success = false;
            }
            items_executed.push(execution);
        }

        DiskCleanupResult {
            schema: "cynic.host_disk_cleanup_result.v1".to_string(),
            timestamp,
            success,
            items_executed,
            total_reclaimed_bytes,
            total_reclaimed_human: bytes_to_human(total_reclaimed_bytes),
        }
    }
}

fn observe_filesystems() -> Result<Vec<DiskFilesystem>, String> {
    // Attempt standard df command with clean output columns
    let out = capture_command(
        "df",
        &["-h", "--output=source,fstype,size,used,avail,pcent,target"],
    );

    let output_str = match out {
        Ok(res) if res.success => res.stdout,
        _ => {
            // Fallback to normal df -h
            let fallback = capture_command("df", &["-h"]);
            match fallback {
                Ok(res) if res.success => res.stdout,
                Ok(res) => return Err(format!("df failed: {}", res.stderr)),
                Err(e) => return Err(e.to_string()),
            }
        }
    };

    Ok(parse_df_filesystems(&output_str))
}

fn get_disk_free_bytes() -> Option<u64> {
    let out = capture_command("df", &["-k", "/"]).ok()?;
    if !out.success {
        return None;
    }
    let line = out.stdout.lines().nth(1)?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 4 {
        // Avail column in KB
        let kb: u64 = parts[3].parse().ok()?;
        Some(kb * 1024)
    } else {
        None
    }
}

fn get_dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let mut total_size = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_symlink() {
                    continue;
                }
                if path.is_dir() {
                    // Skip noisy caches or recursively scanned sub-trees to keep speed high
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name == ".git" || name == "node_modules" {
                            continue;
                        }
                    }
                    stack.push(path);
                } else if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }
    }
    total_size
}

fn observe_known_caches() -> Vec<DiskCacheEntry> {
    let mut entries = Vec::new();
    let home = std::env::var("HOME").map(PathBuf::from).ok();

    // 1. /tmp/cynic-* targets
    let mut tmp_cynic_size = 0u64;
    if let Ok(read_dir) = fs::read_dir("/tmp") {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("cynic-") {
                    tmp_cynic_size += get_dir_size(&path);
                }
            }
        }
    }
    entries.push(DiskCacheEntry {
        label: "tmp-cynic-targets".to_string(),
        path: "/tmp/cynic-*".to_string(),
        size_human: bytes_to_human(tmp_cynic_size),
        size_bytes: tmp_cynic_size,
    });

    // 2. systemd journal
    let journal_path = Path::new("/var/log/journal");
    let journal_size = get_dir_size(journal_path);
    entries.push(DiskCacheEntry {
        label: "journal-logs".to_string(),
        path: journal_path.display().to_string(),
        size_human: bytes_to_human(journal_size),
        size_bytes: journal_size,
    });

    // 3. rotated syslog logs (canonical set from domain/disk/probe.rs)
    let mut rotated_syslog_size = 0u64;
    for file in ROTATED_SYSLOG_FILES {
        if let Ok(meta) = fs::metadata(file) {
            rotated_syslog_size += meta.len();
        }
    }
    entries.push(DiskCacheEntry {
        label: "rotated-syslogs".to_string(),
        path: "/var/log/syslog.*".to_string(),
        size_human: bytes_to_human(rotated_syslog_size),
        size_bytes: rotated_syslog_size,
    });

    if let Some(home_path) = home {
        // 4. npm cache
        let npm = home_path.join(".npm");
        let npm_sz = get_dir_size(&npm);
        entries.push(DiskCacheEntry {
            label: "npm-cache".to_string(),
            path: npm.display().to_string(),
            size_human: bytes_to_human(npm_sz),
            size_bytes: npm_sz,
        });

        // 5. pip cache
        let pip = home_path.join(".cache/pip");
        let pip_sz = get_dir_size(&pip);
        entries.push(DiskCacheEntry {
            label: "pip-cache".to_string(),
            path: pip.display().to_string(),
            size_human: bytes_to_human(pip_sz),
            size_bytes: pip_sz,
        });

        // 6. uv cache
        let uv = home_path.join(".cache/uv");
        let uv_sz = get_dir_size(&uv);
        entries.push(DiskCacheEntry {
            label: "uv-cache".to_string(),
            path: uv.display().to_string(),
            size_human: bytes_to_human(uv_sz),
            size_bytes: uv_sz,
        });

        // 7. HuggingFace cache
        let hf = home_path.join(".cache/huggingface");
        let hf_sz = get_dir_size(&hf);
        entries.push(DiskCacheEntry {
            label: "hf-cache".to_string(),
            path: hf.display().to_string(),
            size_human: bytes_to_human(hf_sz),
            size_bytes: hf_sz,
        });
    }

    entries
}

/// Observe physical disks via `lsblk -b -J` (localhost only). Returns an empty
/// Vec if `lsblk` is unavailable or yields non-JSON; never panics. This is the
/// fourth layer of the disk vertical (physical device → volume → filesystem →
/// usage), sourced from the same `parse_lsblk_disks` parser shared nowhere yet
/// (remote physical disks are left to each node's own `cynic runtime disk`).
fn observe_physical_disks() -> Vec<PhysicalDisk> {
    let out = capture_command(
        "lsblk",
        &[
            "-b",
            "-J",
            "-o",
            "NAME,SIZE,TYPE,ROTA,VENDOR,MODEL,SERIAL,WWN,MOUNTPOINT",
        ],
    );
    let stdout = match out {
        Ok(res) if res.success => res.stdout,
        _ => return Vec::new(),
    };
    parse_lsblk_disks(&stdout)
}

fn observe_large_files() -> Vec<DiskFileEntry> {
    let mut large_files = Vec::new();
    let mut search_paths = vec![Path::new("/tmp"), Path::new("/var/lib/cynic")]
        .into_iter()
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();
    if let Ok(principal_home) = std::env::var("CYNIC_PRINCIPAL_HOME") {
        search_paths.push(PathBuf::from(principal_home));
    }

    for base_path in search_paths {
        if !base_path.exists() {
            continue;
        }

        // Limit walking to avoid scanning too many files
        let mut stack = vec![base_path.to_path_buf()];
        let mut visited_dirs = 0;
        while let Some(current) = stack.pop() {
            visited_dirs += 1;
            if visited_dirs > 500 {
                // Hard limit on directory traversal depth/count for safety and speed
                break;
            }

            if let Ok(entries) = fs::read_dir(current) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_symlink() {
                        continue;
                    }

                    if path.is_dir() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name == ".git"
                                || name == "node_modules"
                                || name == "target"
                                || name == ".cache"
                            {
                                continue;
                            }
                        }
                        stack.push(path);
                    } else if let Ok(metadata) = entry.metadata() {
                        let len = metadata.len();
                        // Keep files > 100 MB
                        if len > 100 * 1024 * 1024 {
                            large_files.push(DiskFileEntry {
                                path: path.display().to_string(),
                                size_human: bytes_to_human(len),
                                size_bytes: len,
                            });
                        }
                    }
                }
            }
        }
    }

    large_files.sort_by_key(|entry| std::cmp::Reverse(entry.size_bytes));
    large_files.truncate(40);
    large_files
}

fn execute_cleanup_item(item: &DiskCleanupItem) -> DiskCleanupStepExecution {
    let mut stdout_tail = String::new();
    let mut stderr_tail = String::new();
    let mut success = true;
    let mut exit_code = Some(0);

    // Hardcoded handlers to avoid wildcards or risky command injection
    match item.id.as_str() {
        "cynic_tmp_targets" => {
            if let Ok(read_dir) = fs::read_dir("/tmp") {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("cynic-") {
                            if path.is_dir() {
                                if let Err(e) = fs::remove_dir_all(&path) {
                                    success = false;
                                    stderr_tail.push_str(&format!(
                                        "Failed to remove dir {}: {}\n",
                                        path.display(),
                                        e
                                    ));
                                }
                            } else if let Err(e) = fs::remove_file(&path) {
                                success = false;
                                stderr_tail.push_str(&format!(
                                    "Failed to remove file {}: {}\n",
                                    path.display(),
                                    e
                                ));
                            }
                        }
                    }
                }
            }
            if success {
                stdout_tail = "Cleaned all /tmp/cynic-* targets successfully".to_string();
            }
        }
        "rotated_syslogs_purge" => {
            for file in ROTATED_SYSLOG_FILES {
                let path = Path::new(file);
                if path.exists() {
                    // Try to remove it. Sudo might be needed since they are in /var/log/
                    let rm_res = if item.requires_privilege {
                        capture_command("sudo", &["rm", "-f", file])
                    } else {
                        capture_command("rm", &["-f", file])
                    };

                    match rm_res {
                        Ok(res) if res.success => {}
                        Ok(res) => {
                            success = false;
                            stderr_tail.push_str(&format!(
                                "Failed to remove syslog file {}: {}\n",
                                file, res.stderr
                            ));
                        }
                        Err(e) => {
                            success = false;
                            stderr_tail.push_str(&format!(
                                "Failed to execute remove for {}: {}\n",
                                file, e
                            ));
                        }
                    }
                }
            }
            if success {
                stdout_tail = "Purged all rotated syslog files successfully".to_string();
            }
        }
        _ => {
            // General external commands: npm cache clean, pip cache purge, uv cache clean, journalctl vacuum
            if item.argv.is_empty() {
                return DiskCleanupStepExecution {
                    item_id: item.id.clone(),
                    label: item.label.clone(),
                    success: false,
                    exit_code: None,
                    stdout_tail: String::new(),
                    stderr_tail: "No command provided for execution".to_string(),
                };
            }

            let (bin, args) = if item.requires_privilege {
                ("sudo", vec![item.argv[0].as_str()])
            } else {
                (item.argv[0].as_str(), vec![])
            };

            let mut all_args = args;
            if item.requires_privilege {
                all_args.extend(item.argv[1..].iter().map(|s| s.as_str()));
            } else {
                all_args.extend(item.argv[1..].iter().map(|s| s.as_str()));
            }

            match capture_command(bin, &all_args) {
                Ok(output) => {
                    success = output.success;
                    exit_code = output.exit_code;
                    stdout_tail = output.stdout;
                    stderr_tail = output.stderr;
                }
                Err(e) => {
                    success = false;
                    exit_code = None;
                    stderr_tail = e.to_string();
                }
            }
        }
    }

    DiskCleanupStepExecution {
        item_id: item.id.clone(),
        label: item.label.clone(),
        success,
        exit_code,
        stdout_tail,
        stderr_tail,
    }
}
