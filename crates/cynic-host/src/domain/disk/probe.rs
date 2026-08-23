//! Pure logic for planning cleanup items based on observed caches.
//!
//! Also hosts the portable `df`-output parser so that both the local and the
//! SSH remote disk adapters share one parsing path (no hardwired substrate).

use super::types::{bytes_to_human, DiskCleanupItem, DiskCleanupPlan, DiskFilesystem, DiskReport};

/// Pure parser for `df -h --output=source,fstype,size,used,avail,pcent,target`
/// stdout. Shared by `LocalDiskAdapter` (local `df`) and `RemoteDiskAdapter`
/// (SSH `df`) so the two never diverge on column layout. Returns only real
/// backing-store filesystems (`/dev/*`, tmpfs, or the root mount) and keeps
/// virtual entries. A missing `fstype` is tolerated as `"unknown"`.
pub fn parse_df_filesystems(output: &str) -> Vec<DiskFilesystem> {
    let mut list = Vec::new();
    let mut lines = output.lines();
    // Skip header line
    let _header = lines.next();

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        let (source, fstype, size, used, avail, pct, mount) = if parts.len() >= 7 {
            (
                parts[0], parts[1], parts[2], parts[3], parts[4], parts[5], parts[6],
            )
        } else {
            // If fstype is missing or we used simple df -h
            (
                parts[0],
                "unknown",
                parts[1],
                parts[2],
                parts[3],
                parts[4],
                parts[parts.len() - 1],
            )
        };
        // Filter virtual filesystems to keep report clean
        if source.starts_with("/dev/") || source == "tmpfs" || mount == "/" {
            list.push(DiskFilesystem {
                source: source.to_string(),
                fstype: fstype.to_string(),
                size_human: size.to_string(),
                used_human: used.to_string(),
                avail_human: avail.to_string(),
                use_pct: pct.to_string(),
                mount: mount.to_string(),
            });
        }
    }
    list
}

/// Build a cleanup plan from the observed disk report.
pub fn build_cleanup_plan(report: &DiskReport) -> DiskCleanupPlan {
    let mut items = Vec::new();
    let mut total_bytes = 0u64;

    for cache in &report.known_caches {
        if cache.size_bytes == 0 {
            continue;
        }

        let item = match cache.label.as_str() {
            "tmp-cynic-targets" => Some(DiskCleanupItem {
                id: "cynic_tmp_targets".to_string(),
                label: "CYNIC temporary cargo targets (/tmp/cynic-*)".to_string(),
                size_human: cache.size_human.clone(),
                size_bytes: cache.size_bytes,
                argv: vec![
                    "rm".to_string(),
                    "-rf".to_string(),
                    "/tmp/cynic-*".to_string(),
                ],
                requires_privilege: true, // Needs sudo because some targets might be root-owned from test suites run under sudo
            }),
            "journal-logs" => Some(DiskCleanupItem {
                id: "systemd_journal_vacuum".to_string(),
                label: "Systemd journal vacuum (vacuum to 200M)".to_string(),
                size_human: cache.size_human.clone(),
                size_bytes: cache.size_bytes.saturating_sub(200 * 1024 * 1024), // Estimate saving everything except 200MB
                argv: vec!["journalctl".to_string(), "--vacuum-size=200M".to_string()],
                requires_privilege: true,
            }),
            "rotated-syslogs" => Some(DiskCleanupItem {
                id: "rotated_syslogs_purge".to_string(),
                label: "Purge rotated log files in /var/log/".to_string(),
                size_human: cache.size_human.clone(),
                size_bytes: cache.size_bytes,
                argv: vec![
                    "rm".to_string(),
                    "-f".to_string(),
                    "/var/log/syslog.1".to_string(),
                    "/var/log/syslog.2.gz".to_string(),
                    "/var/log/syslog.3.gz".to_string(),
                    "/var/log/syslog.4.gz".to_string(),
                    "/var/log/kern.log.1".to_string(),
                    "/var/log/kern.log.2.gz".to_string(),
                    "/var/log/kern.log.3.gz".to_string(),
                    "/var/log/ufw.log.1".to_string(),
                    "/var/log/ufw.log.2.gz".to_string(),
                    "/var/log/ufw.log.3.gz".to_string(),
                ],
                requires_privilege: true,
            }),
            "npm-cache" => Some(DiskCleanupItem {
                id: "npm_cache_clean".to_string(),
                label: "User npm cache".to_string(),
                size_human: cache.size_human.clone(),
                size_bytes: cache.size_bytes,
                argv: vec![
                    "npm".to_string(),
                    "cache".to_string(),
                    "clean".to_string(),
                    "--force".to_string(),
                ],
                requires_privilege: false,
            }),
            "pip-cache" => Some(DiskCleanupItem {
                id: "pip_cache_purge".to_string(),
                label: "User pip cache".to_string(),
                size_human: cache.size_human.clone(),
                size_bytes: cache.size_bytes,
                argv: vec!["pip".to_string(), "cache".to_string(), "purge".to_string()],
                requires_privilege: false,
            }),
            "uv-cache" => Some(DiskCleanupItem {
                id: "uv_cache_clean".to_string(),
                label: "User uv cache".to_string(),
                size_human: cache.size_human.clone(),
                size_bytes: cache.size_bytes,
                argv: vec!["uv".to_string(), "cache".to_string(), "clean".to_string()],
                requires_privilege: false,
            }),
            _ => None,
        };

        if let Some(item) = item {
            total_bytes += item.size_bytes;
            items.push(item);
        }
    }

    DiskCleanupPlan {
        schema: "cynic.host_disk_cleanup_plan.v1".to_string(),
        items,
        total_reclaimable_bytes: total_bytes,
        total_reclaimable_human: bytes_to_human(total_bytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::disk::types::{DiskCacheEntry, DiskReport};

    const SAMPLE_DF: &str = "\
Filesystem     Type  Size  Used Avail Use% Mounted on
/dev/sda2      ext4   50G   30G   20G  60% /
tmpfs          tmpfs  3.9G     0  3.9G   0% /dev
/dev/sdb1      xfs    20G   18G  2.0G  90% /mnt/data
overlay        overlay  50G   30G   20G  60% /
";

    #[test]
    fn test_parse_df_filesystems_filters_virtual() {
        let parsed = parse_df_filesystems(SAMPLE_DF);
        // overlay (non /dev, non tmpfs, mount != "/") is filtered out.
        // Kept: /dev/sda2 (mount /), tmpfs /dev, /dev/sdb1, + root overlay (mount "/")
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0].source, "/dev/sda2");
        assert_eq!(parsed[0].fstype, "ext4");
        assert_eq!(parsed[0].mount, "/");
        assert_eq!(parsed[2].source, "/dev/sdb1");
        assert_eq!(parsed[2].fstype, "xfs");
        assert_eq!(parsed[2].mount, "/mnt/data");
    }

    #[test]
    fn test_parse_df_filesystems_empty_and_header_only() {
        assert!(parse_df_filesystems("").is_empty());
        assert!(parse_df_filesystems("Filesystem Type Size Used Avail Use% Mounted on\n").is_empty());
    }

    #[test]
    fn test_build_cleanup_plan() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let report = DiskReport {
            schema: "cynic.host_disk_report.v1".to_string(),
            timestamp: "2026-07-19T00:00:00Z".to_string(),
            filesystems: vec![],
            known_caches: vec![
                DiskCacheEntry {
                    label: "tmp-cynic-targets".to_string(),
                    path: "/tmp/cynic-*".to_string(),
                    size_human: "10G".to_string(),
                    size_bytes: 10 * 1024 * 1024 * 1024,
                },
                DiskCacheEntry {
                    label: "npm-cache".to_string(),
                    path: format!("{home}/.npm"),
                    size_human: "500M".to_string(),
                    size_bytes: 500 * 1024 * 1024,
                },
            ],
            large_files: vec![],
        };

        let plan = build_cleanup_plan(&report);
        assert_eq!(plan.items.len(), 2);
        assert_eq!(
            plan.total_reclaimable_bytes,
            10 * 1024 * 1024 * 1024 + 500 * 1024 * 1024
        );
        assert_eq!(plan.items[0].id, "cynic_tmp_targets");
        assert_eq!(plan.items[1].id, "npm_cache_clean");
    }
}
