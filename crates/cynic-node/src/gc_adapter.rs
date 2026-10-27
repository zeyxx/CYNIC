//! Daily garbage collection adapter for build targets.
//!
//! I/O adapter: reads disk pressure, lists build-target profiles, calls
//! the pure decision function in cynic-build, and deletes selected targets.
//! No decision logic here — that lives in cynic_build::gc.

use std::time::Duration;
use tracing::{info, warn};

const BUILD_TARGETS_DIR: &str = "/var/lib/cynic/build-targets";
const PRESSURE_THRESHOLD: f64 = 0.8;
const GC_INTERVAL: Duration = Duration::from_secs(86400); // daily

/// Run one GC cycle. Returns the number of profiles removed.
pub fn run_gc_once() -> usize {
    // 1. Check disk pressure.
    let pressure = disk_pressure().unwrap_or(0.0);
    if pressure < PRESSURE_THRESHOLD {
        info!("gc: disk pressure {:.1}% — below threshold, skipping", pressure * 100.0);
        return 0;
    }

    // 2. List build-target profiles.
    let profiles = list_profiles().unwrap_or_default();
    if profiles.is_empty() {
        return 0;
    }

    // 3. Pure decision.
    let to_remove = cynic_build::gc::decide_gc(pressure, &profiles);

    // 4. Delete.
    let mut removed = 0;
    for name in &to_remove {
        let path = std::path::Path::new(BUILD_TARGETS_DIR).join(name);
        if std::fs::remove_dir_all(&path).is_ok() {
            removed += 1;
            info!("gc: removed build target '{}'", name);
        } else {
            warn!("gc: failed to remove build target '{}'", name);
        }
    }
    removed
}

/// Spawn a daily GC background task.
pub fn spawn_daily_gc(shutdown: tokio_util::sync::CancellationToken) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(GC_INTERVAL);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let removed = run_gc_once();
                    if removed > 0 {
                        info!("gc: daily cycle removed {} profiles", removed);
                    }
                }
                _ = shutdown.cancelled() => {
                    info!("gc: shutting down");
                    break;
                }
            }
        }
    });
}

fn disk_pressure() -> Option<f64> {
    let output = std::process::Command::new("df")
        .args(["--output=pcent", BUILD_TARGETS_DIR])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let pct = line.trim().trim_end_matches('%');
        if let Ok(val) = pct.parse::<f64>() {
            return Some(val / 100.0);
        }
    }
    None
}

fn list_profiles() -> Option<Vec<cynic_build::gc::TargetProfile>> {
    let entries = std::fs::read_dir(BUILD_TARGETS_DIR).ok()?;
    let now = std::time::SystemTime::now();
    let mut profiles = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let size = dir_size(&path);
        let age_days = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|mtime| now.duration_since(mtime).unwrap_or_default().as_secs() / 86400)
            .unwrap_or(0);
        profiles.push(cynic_build::gc::TargetProfile {
            name,
            size_bytes: size,
            age_days,
        });
    }
    Some(profiles)
}

fn dir_size(path: &std::path::Path) -> u64 {
    walkdir_size(path)
}

fn walkdir_size(path: &std::path::Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += walkdir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    total
}
