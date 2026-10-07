use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::types::{LxcBridgeBundleReport, LxcBridgeInstallPlan, LxcBridgeStatus};

const HELPER_NAME: &str = "cynic-lxc-exec-bridge";
const HELPER_TARGET: &str = "/usr/local/libexec/cynic-lxc-exec-bridge";
const SUDOERS_TARGET: &str = "/etc/sudoers.d/cynic-lxc-exec-bridge";
const INCIDENT_AUTHORITY_CONFIG: &str = "/etc/cynic/lxc-breakglass-authority.toml";

pub fn install_plan(helper_source: impl AsRef<Path>) -> LxcBridgeInstallPlan {
    let helper_source = helper_source.as_ref().display().to_string();
    LxcBridgeInstallPlan {
        schema: "cynic.lxc_exec_bridge.install_plan.v1",
        status: "planned",
        mutates: true,
        helper_source: helper_source.clone(),
        helper_target: HELPER_TARGET.to_string(),
        sudoers_target: SUDOERS_TARGET.to_string(),
        install_argv: vec![
            vec![
                "install".into(),
                "-o".into(),
                "root".into(),
                "-g".into(),
                "root".into(),
                "-m".into(),
                "0755".into(),
                helper_source,
                HELPER_TARGET.into(),
            ],
            vec![
                "install".into(),
                "-o".into(),
                "root".into(),
                "-g".into(),
                "root".into(),
                "-m".into(),
                "0440".into(),
                format!("<bundle>/{HELPER_NAME}.sudoers"),
                SUDOERS_TARGET.into(),
            ],
            vec!["visudo".into(), "-cf".into(), SUDOERS_TARGET.into()],
        ],
        proofs: vec![
            vec![HELPER_TARGET.into(), "--status".into()],
            vec!["visudo".into(), "-cf".into(), SUDOERS_TARGET.into()],
        ],
    }
}

pub fn bundle(
    output_dir: impl AsRef<Path>,
    helper_source: impl AsRef<Path>,
) -> LxcBridgeBundleReport {
    match write_bundle(output_dir.as_ref(), helper_source.as_ref()) {
        Ok(hash) => LxcBridgeBundleReport {
            schema: "cynic.lxc_exec_bridge.bundle.v1",
            status: "completed".to_string(),
            output_dir: output_dir.as_ref().display().to_string(),
            files: vec![
                HELPER_NAME.to_string(),
                format!("{HELPER_NAME}.sudoers"),
                "lxc-breakglass-authority.toml.tpl".to_string(),
                "manifest.json".to_string(),
            ],
            helper_sha256: Some(format!("sha256:{hash}")),
            secret_exposed: false,
            unknowns: Vec::new(),
        },
        Err(error) => LxcBridgeBundleReport {
            schema: "cynic.lxc_exec_bridge.bundle.v1",
            status: "blocked".to_string(),
            output_dir: output_dir.as_ref().display().to_string(),
            files: Vec::new(),
            helper_sha256: None,
            secret_exposed: false,
            unknowns: vec![error],
        },
    }
}

fn write_bundle(output_dir: &Path, helper_source: &Path) -> Result<String, String> {
    fs::create_dir_all(output_dir).map_err(|error| error.to_string())?;
    let helper = output_dir.join(HELPER_NAME);
    fs::copy(helper_source, &helper).map_err(|error| format!("helper copy failed: {error}"))?;
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o755))
        .map_err(|error| error.to_string())?;
    let bytes = fs::read(&helper).map_err(|error| error.to_string())?;
    let hash = hex::encode(Sha256::digest(&bytes));
    // BOUNDED BY THE BINARY, NOT BY SUDOERS — and that is a measured property,
    // not a choice.
    //
    // MEASURED on the builder: a sudoers rule naming a binary with NO argument
    // (`cynic ALL=(root) NOPASSWD: /usr/bin/echo`) grants that binary with ANY
    // argument — `sudo -l /usr/bin/echo extra-arg` returns the grant. So this
    // membrane cannot bound the bridge's argv at all, and a second
    // `... --status` rule would be redundant: the bare rule already covers it.
    //
    // Why sudoers cannot bound it: the bridge is driven by a JSON request on
    // stdin, and there is no argv to constrain. The real admission is
    // `admit()` -> `admitted_operation()` — an exact-argv allowlist that
    // refuses everything else with `operation_not_in_lxc_bridge_catalog` — and
    // the binary refuses outright when `geteuid() != 0`.
    //
    // This is the ONE membrane in CYNIC whose bound is entirely inside the
    // binary. Recorded here rather than implied, because a reader who assumes
    // sudoers is the enforcement point would conclude the bridge is unbounded.
    let sudoers = format!(
        "# CYNIC Rust-native bounded LXC bridge\n\
         # ONE rule, deliberately unbounded in argv: a bare sudoers grant allows\n\
         # any argument (measured), and the bridge takes its request on stdin.\n\
         # The bound is `cynic_lxc::admit` -> `admitted_operation`: an exact-argv\n\
         # allowlist, plus a root check in the binary. Do not add a `--status`\n\
         # rule expecting it to tighten anything: it would be redundant.\n\
         cynic ALL=(root) NOPASSWD: {HELPER_TARGET}\n"
    );
    let sudoers_path = output_dir.join(format!("{HELPER_NAME}.sudoers"));
    fs::write(&sudoers_path, sudoers).map_err(|error| error.to_string())?;
    fs::set_permissions(&sudoers_path, fs::Permissions::from_mode(0o440))
        .map_err(|error| error.to_string())?;
    let authority_template = format!(
        "# Installed root-owned configuration; paths are injected by host bootstrap.\n# This file contains no signing secret.\npublic_key_path = \"<incident-authority-public-key>\"\nreceipt_root = \"<durable-breakglass-receipt-root>\"\n# Target: {INCIDENT_AUTHORITY_CONFIG}\n"
    );
    fs::write(
        output_dir.join("lxc-breakglass-authority.toml.tpl"),
        authority_template,
    )
    .map_err(|error| error.to_string())?;
    let manifest = serde_json::json!({
        "schema": "cynic.lxc_exec_bridge.bundle_manifest.v1",
        "helper": HELPER_NAME,
        "helper_sha256": format!("sha256:{hash}"),
        "helper_target": HELPER_TARGET,
        "sudoers_target": SUDOERS_TARGET,
        "incident_authority_config": INCIDENT_AUTHORITY_CONFIG,
        "runtime": "rust_native",
        "shell_execution": false
    });
    fs::write(
        output_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap_or_default(),
    )
    .map_err(|error| error.to_string())?;
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_sudoers_has_one_rule_and_says_where_the_bound_lives() {
        // Regression lock. MEASURED on the builder: a bare sudoers grant
        // (`cynic ALL=(root) NOPASSWD: /usr/bin/echo`) authorizes the binary
        // with ANY argument — `sudo -n /usr/bin/echo REAL_ARGS_WORK` executed
        // as root, exit 0. So a `... --status` rule would be redundant, and the
        // bundle must not imply sudoers is the enforcement point.
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cynic-lxc-bundle-{unique}"));
        fs::create_dir_all(&dir).expect("tempdir");
        let helper = dir.join("helper-src");
        fs::write(&helper, b"fake helper").expect("write helper");

        let report = bundle(&dir, &helper);
        assert_eq!(report.status, "completed", "{:?}", report.unknowns);

        let sudoers =
            fs::read_to_string(dir.join(format!("{HELPER_NAME}.sudoers"))).expect("sudoers");
        let rules: Vec<&str> = sudoers
            .lines()
            .filter(|line| line.contains("NOPASSWD"))
            .collect();
        assert_eq!(
            rules.len(),
            1,
            "exactly one rule; a second would be redundant: {rules:?}"
        );
        assert!(
            rules[0].ends_with(HELPER_TARGET),
            "the rule names the binary with no argument: {}",
            rules[0]
        );
        assert!(
            sudoers.contains("admitted_operation"),
            "the bundle must name where the bound actually lives: {sudoers}"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}

pub fn status(helper_path: Option<&str>, sudoers_path: Option<&str>) -> LxcBridgeStatus {
    let helper = PathBuf::from(helper_path.unwrap_or(HELPER_TARGET));
    let sudoers = PathBuf::from(sudoers_path.unwrap_or(SUDOERS_TARGET));
    let helper_meta = fs::metadata(&helper).ok();
    let sudoers_meta = fs::metadata(&sudoers).ok();
    let sudoers_content = fs::read_to_string(&sudoers).ok();
    let bounded = sudoers_content.as_ref().map(|content| {
        content
            .lines()
            .filter(|line| line.contains("NOPASSWD"))
            .all(|line| {
                line.ends_with(HELPER_TARGET)
                    || line.ends_with(&format!("{HELPER_TARGET} --status"))
            })
            && !content.contains("/usr/sbin/pct")
            && !content.contains(" *")
    });
    let mut blockers = Vec::new();
    if helper_meta.is_none() {
        blockers.push("helper_missing".to_string());
    }
    if helper_meta
        .as_ref()
        .is_some_and(|meta| meta.permissions().mode() & 0o111 == 0)
    {
        blockers.push("helper_not_executable".to_string());
    }
    if helper_meta.as_ref().is_some_and(|meta| meta.uid() != 0) {
        blockers.push("helper_not_root_owned".to_string());
    }
    if sudoers_meta.is_none() {
        blockers.push("sudoers_missing".to_string());
    }
    if sudoers_meta
        .as_ref()
        .is_some_and(|meta| meta.permissions().mode() & 0o777 != 0o440)
    {
        blockers.push("sudoers_mode_invalid".to_string());
    }
    if bounded == Some(false) {
        blockers.push("sudoers_unbounded".to_string());
    }
    LxcBridgeStatus {
        schema: "cynic.lxc_exec_bridge.status.v1",
        status: if blockers.is_empty() {
            "active"
        } else {
            "blocked"
        },
        helper_path: helper.display().to_string(),
        helper_present: helper_meta.is_some(),
        helper_executable: helper_meta
            .as_ref()
            .is_some_and(|meta| meta.permissions().mode() & 0o111 != 0),
        helper_root_owned: helper_meta.as_ref().map(|meta| meta.uid() == 0),
        sudoers_path: sudoers.display().to_string(),
        sudoers_present: sudoers_meta.is_some(),
        sudoers_mode_0440: sudoers_meta
            .as_ref()
            .map(|meta| meta.permissions().mode() & 0o777 == 0o440),
        sudoers_bounded: bounded,
        blockers,
    }
}
