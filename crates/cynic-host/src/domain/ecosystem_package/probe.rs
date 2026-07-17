//! Principal-aware binary resolution (prefer layout bin over ambient PATH).

use std::process::Command;

use super::types::{ObservedBinary, PrincipalInstallLayout};

/// Prefer principal bin_dir over ambient PATH when both exist.
pub fn resolve_binary(name: &str, layout: &PrincipalInstallLayout) -> ObservedBinary {
    let principal_path = layout.bin_dir.join(name);
    if principal_path.is_file() {
        return ObservedBinary {
            path: Some(principal_path.display().to_string()),
            version: None,
            under_principal: true,
        };
    }
    // Also check $HOME/bin used by some cynic operator wrappers
    let home_bin = layout.home_dir.join("bin").join(name);
    if home_bin.is_file() {
        return ObservedBinary {
            path: Some(home_bin.display().to_string()),
            version: None,
            under_principal: true,
        };
    }
    if let Some(p) = which(name) {
        let under = p.starts_with(&layout.home_dir.display().to_string());
        return ObservedBinary {
            path: Some(p),
            version: None,
            under_principal: under,
        };
    }
    ObservedBinary {
        path: None,
        version: None,
        under_principal: false,
    }
}

pub fn which(bin: &str) -> Option<String> {
    let out = Command::new("which").arg(bin).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() {
        None
    } else {
        Some(p)
    }
}
