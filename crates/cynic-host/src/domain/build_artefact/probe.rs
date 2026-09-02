//! Pure parsers for build artefact observation (no I/O).
//! 4-layer substrate mining: physical -> volume -> usage (checkouts) -> pressure.

use cynic_protocol::build_artefact::{
    BUILD_ARTEFACT_GC_PLAN_SCHEMA, BuildArtefactGcItem, BuildArtefactGcPlan, BuildArtefactReport,
    CheckoutEntry, DiskPressure,
};
use cynic_protocol::disk::bytes_to_human;

/// Parse `ls -1 /var/lib/cynic/checkouts` + `du -sb` + `stat` lines into CheckoutEntry.
/// Each line: "<path>|<sha>|<size_bytes>|<uid>:<gid>|<mtime_secs>|<remote_exists 0/1>"
/// Pure, no FS touch.
pub fn parse_checkout_list(raw: &str, remote_shas: &[String]) -> Vec<CheckoutEntry> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 6 {
            continue;
        }
        let path = parts[0].to_string();
        let sha = parts[1].to_string();
        let size_bytes: u64 = parts[2].parse().unwrap_or(0);
        let owner: Vec<&str> = parts[3].split(':').collect();
        let uid: u32 = owner.first().and_then(|v| v.parse().ok()).unwrap_or(999);
        let gid: u32 = owner.get(1).and_then(|v| v.parse().ok()).unwrap_or(990);
        let age_secs: Option<u64> = parts[4].parse().ok();
        let remote_flag: Option<bool> = match parts[5] {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        };
        let is_orphan = match remote_flag {
            Some(exists) => !exists,
            None => {
                // No remote fetch available (unknown) -> conservative, not orphan
                if remote_shas.is_empty() {
                    false
                } else {
                    !remote_shas.contains(&sha)
                }
            }
        };
        let branch = path.rsplit('/').next().unwrap_or(&path).to_string();
        out.push(CheckoutEntry {
            path: path.clone(),
            sha: sha.clone(),
            branch,
            age_secs,
            size_bytes,
            size_human: bytes_to_human(size_bytes),
            owner_uid: uid,
            owner_gid: gid,
            is_orphan,
            is_root_owned: uid == 0,
            remote_exists: remote_flag,
        });
    }
    out.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    out
}

/// Parse `df -k /var/lib/cynic` avail/use% into DiskPressure. Pure.
pub fn parse_df_pressure(df_output: &str, root_owned_count: usize) -> DiskPressure {
    let mut avail_bytes = 0u64;
    let mut use_pct = 0u8;
    for line in df_output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(kb) = parts[3].parse::<u64>() {
                avail_bytes = kb * 1024;
            }
            if let Some(pct_str) = parts.get(4) {
                if let Ok(pct) = pct_str.trim_end_matches('%').parse::<u8>() {
                    use_pct = pct;
                }
            }
            break;
        }
    }
    DiskPressure {
        avail_bytes,
        avail_human: bytes_to_human(avail_bytes),
        use_pct,
        is_pressured: use_pct >= 80 || avail_bytes < 5 * 1024 * 1024 * 1024,
        root_owned_target_count: root_owned_count,
    }
}

/// Pure GC plan from report. Orphan + old + root-owned are reclaimable.
pub fn build_gc_plan(report: &BuildArtefactReport) -> BuildArtefactGcPlan {
    let mut items = Vec::new();
    let mut total = 0u64;
    for co in &report.checkouts {
        let mut reasons = Vec::new();
        if co.is_orphan {
            reasons.push("orphan(no remote)".to_string());
        }
        if co.is_root_owned {
            reasons.push("root-owned".to_string());
        }
        if let Some(age) = co.age_secs {
            if age > 48 * 3600 {
                reasons.push(format!("age {}h", age / 3600));
            }
        }
        if reasons.is_empty() {
            continue;
        }
        let reason = reasons.join("+");
        let id = format!("checkout:{}", co.sha.chars().take(8).collect::<String>());
        let label = format!("{} ({})", co.path, reason);
        total += co.size_bytes;
        items.push(BuildArtefactGcItem {
            id,
            label,
            path: co.path.clone(),
            size_human: co.size_human.clone(),
            size_bytes: co.size_bytes,
            reason,
            requires_privilege: co.is_root_owned,
            argv: vec!["rm".to_string(), "-rf".to_string(), co.path.clone()],
        });
    }
    BuildArtefactGcPlan {
        schema: BUILD_ARTEFACT_GC_PLAN_SCHEMA.to_string(),
        items,
        total_reclaimable_bytes: total,
        total_reclaimable_human: bytes_to_human(total),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_checkout_orphan_detection() {
        let raw = "/var/lib/cynic/checkouts/audit-1a186a6|1a186a6c4564263d35e622e21770e9cb9be9a945|69632000|999:990|3600|0\n/var/lib/cynic/checkouts/main-1f74d98a|1f74d98a6a00be0abe87e06740a18d854f4132de|1024000|0:0|100|1";
        let remotes = vec!["1f74d98a6a00be0abe87e06740a18d854f4132de".to_string()];
        let entries = parse_checkout_list(raw, &remotes);
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .find(|e| e.sha.starts_with("1a186a6"))
                .unwrap()
                .is_orphan
        );
        assert!(
            !entries
                .iter()
                .find(|e| e.sha.starts_with("1f74d98"))
                .unwrap()
                .is_orphan
        );
        assert!(
            entries
                .iter()
                .find(|e| e.sha.starts_with("1f74d98"))
                .unwrap()
                .is_root_owned
        );
    }

    #[test]
    fn parse_df_pressure_threshold() {
        let df = "Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/mapper/pve-vm--105--disk--0 82542592 33800000 44000000 44% /";
        let p = parse_df_pressure(df, 0);
        assert_eq!(p.use_pct, 44);
        assert!(!p.is_pressured);
        let df2 = "Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/mapper/pve-vm--105--disk--0 82542592 70000000 1000000 88% /";
        let p2 = parse_df_pressure(df2, 2);
        assert!(p2.is_pressured);
        assert_eq!(p2.root_owned_target_count, 2);
    }

    #[test]
    fn build_gc_plan_filters_orphan_and_age() {
        let report = BuildArtefactReport {
            schema: "cynic.build_artefact_report.v1".to_string(),
            timestamp: "2026-09-02T17:00:00Z".to_string(),
            physical_disks: vec![],
            filesystems: vec![],
            checkouts: vec![
                CheckoutEntry {
                    path: "/var/lib/cynic/checkouts/old-orphan".to_string(),
                    sha: "abc123".to_string(),
                    branch: "old-orphan".to_string(),
                    age_secs: Some(200_000),
                    size_bytes: 5000,
                    size_human: "5.0K".to_string(),
                    owner_uid: 999,
                    owner_gid: 990,
                    is_orphan: true,
                    is_root_owned: false,
                    remote_exists: Some(false),
                },
                CheckoutEntry {
                    path: "/var/lib/cynic/checkouts/live".to_string(),
                    sha: "live123".to_string(),
                    branch: "live".to_string(),
                    age_secs: Some(1000),
                    size_bytes: 5000,
                    size_human: "5.0K".to_string(),
                    owner_uid: 999,
                    owner_gid: 990,
                    is_orphan: false,
                    is_root_owned: false,
                    remote_exists: Some(true),
                },
            ],
            total_checkout_bytes: 10000,
            total_checkout_human: "10K".to_string(),
            target_bytes: 0,
            target_human: "0B".to_string(),
            pressure: DiskPressure {
                avail_bytes: 40 * 1024 * 1024 * 1024,
                avail_human: "40G".to_string(),
                use_pct: 30,
                is_pressured: false,
                root_owned_target_count: 0,
            },
            residual_unknowns: vec![],
        };
        let plan = build_gc_plan(&report);
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].path, "/var/lib/cynic/checkouts/old-orphan");
    }
}
