//! Principal-aware binary resolution (prefer layout bin over ambient PATH).

use std::path::{Path, PathBuf};

use super::types::{ObservedBinary, PrincipalInstallLayout, PythonInstallProvenance};

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
    if bin.is_empty() || bin.contains('/') {
        return None;
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(bin))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.display().to_string())
}

pub fn observe_python_install_provenance(
    distribution: &str,
    binary_path: &str,
) -> Option<PythonInstallProvenance> {
    let installer = read_installer_marker(distribution, binary_path);
    Some(classify_python_install_provenance(
        installer.as_deref(),
        binary_path,
    ))
}

/// Read Python package provenance without importing or executing Python.
/// Installers expose a small `INSTALLER` marker in a `*.dist-info` directory;
/// the layout is observed from the executable's installation root.
fn read_installer_marker(distribution: &str, binary_path: &str) -> Option<String> {
    let executable = Path::new(binary_path);
    let bin_dir = executable.parent()?;
    let mut roots = Vec::new();
    if let Some(root) = bin_dir.parent() {
        roots.push(root.to_path_buf());
        if let Some(parent) = root.parent() {
            roots.push(parent.to_path_buf());
        }
    }
    roots.sort();
    roots.dedup();
    let distribution_key = normalize_distribution(distribution);
    for root in roots {
        for lib_name in ["lib", "lib64"] {
            let lib_root = root.join(lib_name);
            let Ok(entries) = std::fs::read_dir(lib_root) else {
                continue;
            };
            for entry in entries.flatten() {
                let python_root = entry.path();
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if !kind.is_dir() || !entry.file_name().to_string_lossy().starts_with("python") {
                    continue;
                }
                for site_name in ["site-packages", "dist-packages"] {
                    let site = python_root.join(site_name);
                    let Ok(packages) = std::fs::read_dir(site) else {
                        continue;
                    };
                    for package in packages.flatten() {
                        let name = package.file_name().to_string_lossy().to_string();
                        if !name.ends_with(".dist-info") {
                            continue;
                        }
                        let prefix = name.trim_end_matches(".dist-info");
                        let package_key = prefix.rsplit_once('-').map_or(prefix, |(name, _)| name);
                        if normalize_distribution(package_key) != distribution_key {
                            continue;
                        }
                        let marker = package.path().join("INSTALLER");
                        let value = std::fs::read_to_string(marker).ok()?;
                        let value = value.trim();
                        if !value.is_empty() {
                            return Some(value.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn normalize_distribution(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '-' | '_' | '.' => '-',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

pub fn classify_python_install_provenance(
    installer: Option<&str>,
    binary_path: &str,
) -> PythonInstallProvenance {
    if binary_path.contains("/.local/share/uv/tools/") {
        return PythonInstallProvenance::UvTool;
    }
    if binary_path.contains("/pipx/venvs/") || installer == Some("pipx") {
        return PythonInstallProvenance::Pipx;
    }
    if binary_path.contains("/.venv/") || binary_path.contains("/venv/") {
        return if installer == Some("uv") {
            PythonInstallProvenance::UvVenv
        } else {
            PythonInstallProvenance::VirtualEnv
        };
    }
    if installer == Some("uv") {
        return PythonInstallProvenance::UvTool;
    }
    if installer == Some("pip") {
        if binary_path.starts_with("/home/") || binary_path.contains("/.local/bin/") {
            return PythonInstallProvenance::PipUserSite;
        }
        if binary_path.starts_with("/usr/bin/") || binary_path.starts_with("/usr/local/bin/") {
            return PythonInstallProvenance::SystemPython;
        }
    }
    PythonInstallProvenance::Unknown
}

#[cfg(test)]
mod tests {
    use super::{
        PythonInstallProvenance, classify_python_install_provenance, read_installer_marker,
    };
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn classify_pip_user_site_install() {
        assert_eq!(
            classify_python_install_provenance(Some("pip"), "/home/user/.local/bin/hermes"),
            PythonInstallProvenance::PipUserSite
        );
    }

    #[test]
    fn classify_uv_tool_install() {
        assert_eq!(
            classify_python_install_provenance(
                Some("uv"),
                "/home/user/.local/share/uv/tools/hermes-agent/bin/hermes"
            ),
            PythonInstallProvenance::UvTool
        );
    }

    #[test]
    fn classify_uv_venv_install() {
        assert_eq!(
            classify_python_install_provenance(
                Some("uv"),
                "/home/user/Bureau/CYNIC/.venv/bin/hermes"
            ),
            PythonInstallProvenance::UvVenv
        );
    }

    #[test]
    fn reads_dist_info_installer_marker_without_python() {
        let root = std::env::temp_dir().join(format!("cynic-python-probe-{}", std::process::id()));
        let site = root.join(".local/lib/python3.12/site-packages/demo_tool-1.0.dist-info");
        fs::create_dir_all(&site).expect("create package metadata");
        fs::write(site.join("INSTALLER"), "uv\n").expect("write installer marker");
        let binary = root.join(".local/bin/demo-tool");
        fs::create_dir_all(binary.parent().expect("binary parent")).expect("create bin");
        fs::write(&binary, b"shim").expect("write binary marker");

        assert_eq!(
            read_installer_marker("demo-tool", &PathBuf::from(&binary).display().to_string()),
            Some("uv".to_string())
        );
        let _ = fs::remove_dir_all(root);
    }
}
