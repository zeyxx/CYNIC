//! Pure parsers for build artefact observation (no I/O).
//!
//! These are internal to the build_artefact domain — they parse raw
//! command output into intermediate types that adapters convert to the
//! retention trio's `RetentionObservation`.

use cynic_build::build_artefact::{CheckoutEntry, DiskPressure};
use cynic_protocol::disk::bytes_to_human;

/// Parse `ls -1 <checkouts>` + `du -sb` + `stat` lines into CheckoutEntry.
/// Each line: "<path>|<sha>|<size_bytes>|<uid>:<gid>|<mtime_secs>|<remote_exists 0/1>|<foreign 0/1>"
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
        // Optional 7th field: the tree contains entries this account does
        // not own. Absent means false, so a line from an older producer
        // still parses.
        let contains_foreign_owned = parts.get(6).map(|v| *v == "1").unwrap_or(false);
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
            // Privilege is required when the ROOT is root-owned, or when
            // the CONTENT holds entries this account cannot remove.
            is_root_owned: uid == 0 || contains_foreign_owned,
            contains_foreign_owned,
            remote_exists: remote_flag,
        });
    }
    out.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    out
}

/// Parse `df -k <state root>` avail/use% into DiskPressure. Pure.
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
