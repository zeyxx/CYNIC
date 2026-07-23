//! Principal-aware binary resolution (prefer layout bin over ambient PATH).

use std::process::Command;

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
    let out = Command::new("which").arg(bin).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() { None } else { Some(p) }
}

pub fn observe_python_install_provenance(
    distribution: &str,
    binary_path: &str,
) -> Option<PythonInstallProvenance> {
    let output = Command::new("python3")
        .args([
            "-c",
            "import importlib.metadata as m, sys; d=m.distribution(sys.argv[1]); print((d.read_text('INSTALLER') or '').strip())",
            distribution,
        ])
        .output()
        .ok()?;
    let installer = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        String::new()
    };
    Some(classify_python_install_provenance(
        if installer.is_empty() {
            None
        } else {
            Some(installer.as_str())
        },
        binary_path,
    ))
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
    use super::{PythonInstallProvenance, classify_python_install_provenance};

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
}
