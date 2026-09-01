//! Cleanup agent — observes and cleans orphaned builder resources.
//!
//! Built locally on cortex (rustc 1.95), deployed to builder.
//! Runs via `cynic-cleanup --dry-run` or `cynic-cleanup --yes`.

use std::process::Command;

fn parse_size(s: &str) -> u64 {
    let s = s.trim();
    if let Some(num) = s.strip_suffix("G") {
        num.parse::<f64>().map(|n| (n * 1_000_000_000.0) as u64).unwrap_or(0)
    } else if let Some(num) = s.strip_suffix("M") {
        num.parse::<f64>().map(|n| (n * 1_000_000.0) as u64).unwrap_or(0)
    } else if let Some(num) = s.strip_suffix("K") {
        num.parse::<f64>().map(|n| (n * 1_000.0) as u64).unwrap_or(0)
    } else {
        s.parse::<u64>().unwrap_or(0)
    }
}

fn ssh_cmd(cmd: &str) -> Result<String, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("ssh failed: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn observe_disk() -> Result<(u8, u64, u64), String> {
    let out = ssh_cmd("ssh cynic-builder-ssh 'df -B1 / | tail -1'")?;
    let parts: Vec<&str> = out.split_whitespace().collect();
    if parts.len() < 4 {
        return Err("unexpected df output".to_string());
    }
    let total: u64 = parts[1].parse().map_err(|_| "parse total")?;
    let used: u64 = parts[2].parse().map_err(|_| "parse used")?;
    let available: u64 = parts[3].parse().map_err(|_| "parse available")?;
    let percent = ((used as f64 / total as f64) * 100.0) as u8;
    Ok((percent, available, total))
}

fn observe_checkouts() -> Result<Vec<(String, u64)>, String> {
    let out = ssh_cmd("ssh cynic-builder-ssh 'du -sh /var/lib/cynic/checkouts/* 2>/dev/null'")?;
    let mut checkouts = Vec::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 {
            checkouts.push((parts[1].to_string(), parse_size(parts[0])));
        }
    }
    Ok(checkouts)
}

fn observe_tmp() -> Vec<(String, u64)> {
    let out = ssh_cmd("ssh cynic-builder-ssh 'du -sh /tmp/cynic-* 2>/dev/null | sort -rh | head -10'").unwrap_or_default();
    out.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                Some((parts[1].to_string(), parse_size(parts[0])))
            } else {
                None
            }
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dry_run = !args.contains(&"--yes".to_string());

    println!("=== CYNIC Cleanup Agent ===");
    println!("Mode: {}", if dry_run { "DRY-RUN (use --yes to execute)" } else { "EXECUTE" });

    let (percent, available, total) = match observe_disk() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to observe disk: {}", e);
            std::process::exit(1);
        }
    };

    println!("\nDisk: {}% used, {}B available / {}B total", percent, available, total);

    let checkouts = match observe_checkouts() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to observe checkouts: {}", e);
            Vec::new()
        }
    };

    println!("\nCheckouts:");
    for (path, size) in &checkouts {
        println!("  {}B  {}", size, path);
    }

    let tmp_items = observe_tmp();
    println!("\n/tmp/cynic-*:");
    for (path, size) in &tmp_items {
        println!("  {}B  {}", size, path);
    }

    println!("\nRecommendation:");
    if percent < 80 {
        println!("  Disk usage below 80% threshold. No action needed.");
    } else {
        println!("  Disk usage critical. Cleanup recommended.");
        println!("  Run with --yes to execute cleanup.");
    }
}
