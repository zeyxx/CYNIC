//! Observable uv/python authority surface for principal-local ecosystem tooling.

use super::probe::resolve_binary;
use super::types::{ObservedPythonTooling, PrincipalInstallLayout};

pub fn observe_python_tooling(layout: &PrincipalInstallLayout) -> ObservedPythonTooling {
    let uv_tools_dir = layout.home_dir.join(".local/share/uv/tools");
    let principal_local_bin_dir = layout.bin_dir.clone();
    let mut notes = Vec::new();

    notes.push(format!(
        "uv tool payloads should live under {}",
        uv_tools_dir.display()
    ));
    notes.push(format!(
        "principal CLI shims should resolve from {}",
        principal_local_bin_dir.display()
    ));

    let uv = resolve_binary("uv", layout);
    let python3 = resolve_binary("python3", layout);

    if uv.path.is_none() {
        notes.push(
            "uv missing: Python CLI authorities like hermes-agent cannot be normalized".into(),
        );
    }
    if python3.path.is_none() {
        notes.push("python3 missing: Python distribution provenance cannot be observed".into());
    }

    ObservedPythonTooling {
        uv,
        python3,
        uv_tools_dir,
        principal_local_bin_dir,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ecosystem_package::types::{InstallPrincipalKind, RunAs};
    use std::path::PathBuf;

    fn test_layout() -> PrincipalInstallLayout {
        PrincipalInstallLayout {
            principal: InstallPrincipalKind::RuntimeUser,
            runtime_user: "cynic".into(),
            home_dir: PathBuf::from("/home/cynic"),
            bin_dir: PathBuf::from("/home/cynic/.local/bin"),
            src_dir: PathBuf::from("/home/cynic/.local/share/cynic/src"),
            share_dir: PathBuf::from("/home/cynic/.local/share/cynic"),
            run_as: RunAs::Current,
            writable: true,
            notes: vec![],
        }
    }

    #[test]
    fn python_tooling_uses_principal_local_layout() {
        let tooling = observe_python_tooling(&test_layout());
        assert_eq!(
            tooling.uv_tools_dir,
            PathBuf::from("/home/cynic/.local/share/uv/tools")
        );
        assert_eq!(
            tooling.principal_local_bin_dir,
            PathBuf::from("/home/cynic/.local/bin")
        );
        assert!(
            tooling
                .notes
                .iter()
                .any(|note| note.contains("/home/cynic/.local/share/uv/tools"))
        );
    }
}
