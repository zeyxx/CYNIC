//! Generic tooling authority observation for ecosystem package installs.

use super::probe::observe_python_install_provenance;
use super::types::{
    PythonInstallAuthority, PythonInstallProvenance, ToolingAuthority, ToolingAuthorityFamily,
    ToolingInstallProvenance,
};

pub fn observe_tooling_authority(
    family: ToolingAuthorityFamily,
    distribution: Option<&str>,
    binary_path: Option<&str>,
    expected_provenance: Option<ToolingInstallProvenance>,
) -> ToolingAuthority {
    let observed_provenance = match family {
        ToolingAuthorityFamily::Python => match (distribution, binary_path) {
            (Some(distribution), Some(binary_path)) => {
                observe_python_install_provenance(distribution, binary_path)
                    .map(tooling_provenance_from_python)
            }
            _ => None,
        },
        ToolingAuthorityFamily::Node => binary_path.and_then(classify_node_install_provenance),
        ToolingAuthorityFamily::Brew => binary_path.and_then(classify_brew_install_provenance),
        ToolingAuthorityFamily::Rustup => binary_path.and_then(classify_rustup_install_provenance),
    };

    ToolingAuthority {
        family,
        observed_provenance,
        expected_provenance,
        status: tooling_authority_status(observed_provenance, expected_provenance).into(),
    }
}

pub fn observe_python_install_authority(
    distribution: Option<&str>,
    binary_path: Option<&str>,
    expected_provenance: Option<PythonInstallProvenance>,
) -> PythonInstallAuthority {
    let expected_tooling = expected_provenance.map(tooling_provenance_from_python);
    let authority = observe_tooling_authority(
        ToolingAuthorityFamily::Python,
        distribution,
        binary_path,
        expected_tooling,
    );
    PythonInstallAuthority {
        observed_provenance: authority
            .observed_provenance
            .and_then(python_provenance_from_tooling),
        expected_provenance: authority
            .expected_provenance
            .and_then(python_provenance_from_tooling),
        status: authority.status,
    }
}

pub fn tooling_authority_status(
    observed: Option<ToolingInstallProvenance>,
    expected: Option<ToolingInstallProvenance>,
) -> &'static str {
    match (observed, expected) {
        (_, None) => "n/a",
        (Some(observed), Some(expected)) if observed == expected => "canonical",
        (Some(_), Some(_)) => "noncanonical",
        (None, Some(_)) => "unknown",
    }
}

pub fn tooling_provenance_from_python(
    provenance: PythonInstallProvenance,
) -> ToolingInstallProvenance {
    match provenance {
        PythonInstallProvenance::UvTool => ToolingInstallProvenance::UvTool,
        PythonInstallProvenance::UvVenv => ToolingInstallProvenance::UvVenv,
        PythonInstallProvenance::VirtualEnv => ToolingInstallProvenance::VirtualEnv,
        PythonInstallProvenance::Pipx => ToolingInstallProvenance::Pipx,
        PythonInstallProvenance::PipUserSite => ToolingInstallProvenance::PipUserSite,
        PythonInstallProvenance::SystemPython => ToolingInstallProvenance::SystemPython,
        PythonInstallProvenance::Unknown => ToolingInstallProvenance::Unknown,
    }
}

fn python_provenance_from_tooling(
    provenance: ToolingInstallProvenance,
) -> Option<PythonInstallProvenance> {
    Some(match provenance {
        ToolingInstallProvenance::UvTool => PythonInstallProvenance::UvTool,
        ToolingInstallProvenance::UvVenv => PythonInstallProvenance::UvVenv,
        ToolingInstallProvenance::VirtualEnv => PythonInstallProvenance::VirtualEnv,
        ToolingInstallProvenance::Pipx => PythonInstallProvenance::Pipx,
        ToolingInstallProvenance::PipUserSite => PythonInstallProvenance::PipUserSite,
        ToolingInstallProvenance::SystemPython => PythonInstallProvenance::SystemPython,
        ToolingInstallProvenance::Unknown => PythonInstallProvenance::Unknown,
        _ => return None,
    })
}

fn classify_node_install_provenance(binary_path: &str) -> Option<ToolingInstallProvenance> {
    if binary_path.contains("/node_modules/") || binary_path.contains("/npm/") {
        return Some(ToolingInstallProvenance::NpmGlobal);
    }
    if binary_path.contains("/.nvm/") || binary_path.contains("/.local/bin/") {
        return Some(ToolingInstallProvenance::NpmGlobal);
    }
    None
}

fn classify_brew_install_provenance(binary_path: &str) -> Option<ToolingInstallProvenance> {
    if binary_path.contains("/Cellar/") || binary_path.contains("/Homebrew/Cellar/") {
        return Some(ToolingInstallProvenance::HomebrewCellar);
    }
    if binary_path.contains("/Caskroom/") {
        return Some(ToolingInstallProvenance::HomebrewCask);
    }
    None
}

fn classify_rustup_install_provenance(binary_path: &str) -> Option<ToolingInstallProvenance> {
    if binary_path.contains("/.rustup/") || binary_path.contains("/.cargo/bin/") {
        return Some(ToolingInstallProvenance::RustupToolchain);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooling_status_is_canonical_when_expected_matches_observed() {
        assert_eq!(
            tooling_authority_status(
                Some(ToolingInstallProvenance::UvTool),
                Some(ToolingInstallProvenance::UvTool)
            ),
            "canonical"
        );
    }

    #[test]
    fn node_global_install_is_classified() {
        assert_eq!(
            classify_node_install_provenance("/home/user/.nvm/versions/node/v24.0.0/bin/codex"),
            Some(ToolingInstallProvenance::NpmGlobal)
        );
    }

    #[test]
    fn brew_cellar_install_is_classified() {
        assert_eq!(
            classify_brew_install_provenance("/opt/homebrew/Cellar/node/24.0.0/bin/node"),
            Some(ToolingInstallProvenance::HomebrewCellar)
        );
    }

    #[test]
    fn rustup_install_is_classified() {
        assert_eq!(
            classify_rustup_install_provenance("/home/user/.cargo/bin/rustc"),
            Some(ToolingInstallProvenance::RustupToolchain)
        );
    }
}
