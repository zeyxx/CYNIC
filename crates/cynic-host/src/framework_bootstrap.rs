use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkBootstrapBundleReport {
    schema: &'static str,
    pub status: &'static str,
    mutates: bool,
    output_dir: String,
    files: Vec<String>,
    entrypoint: String,
    operator_command: String,
    maintenance_command: String,
    target_runtime: &'static str,
    security_invariants: Vec<&'static str>,
}

pub fn write_framework_bootstrap_bundle(
    output_dir: &str,
) -> Result<FrameworkBootstrapBundleReport, String> {
    let output_dir = Path::new(output_dir);
    let host_exec_bundle_dir = output_dir.join("host-exec-bundle");
    fs::create_dir_all(&host_exec_bundle_dir)
        .map_err(|error| format!("failed to create host-exec-bundle dir: {error}"))?;
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to locate current executable: {error}"))?;
    let bridge_manifest =
        crate::bridge::write_bridge_bundle_from_binary(&host_exec_bundle_dir, &current_exe)
            .map_err(|error| format!("failed to write host-exec bridge bundle: {error}"))?;

    let mut files = vec![
        "host-exec-bundle/cynic-host-exec".to_string(),
        "host-exec-bundle/cynic-host-exec.sudoers".to_string(),
        "host-exec-bundle/manifest.json".to_string(),
    ];

    write_executable_file(
        output_dir.join("install.sh"),
        bootstrap_install_script(),
        output_dir,
        &mut files,
    )?;
    write_executable_file(
        output_dir.join("verify.sh"),
        bootstrap_verify_script(),
        output_dir,
        &mut files,
    )?;
    write_executable_file(
        output_dir.join("security-maintenance.sh"),
        bootstrap_security_maintenance_script(),
        output_dir,
        &mut files,
    )?;
    write_file(
        output_dir.join("security-policy.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "cynic.framework_bootstrap.security_policy.v1",
            "runtime_user": "cynic",
            "runtime_group": "cynic",
            "helper_target": "/usr/local/libexec/cynic-host-exec",
            "sudoers_target": "/etc/sudoers.d/cynic-host-exec",
            "allowed_invocations": [
                ["/usr/local/libexec/cynic-host-exec", "principal-prepare"],
                ["/usr/local/libexec/cynic-host-exec", "principal-prepare", "--apply"],
                ["/usr/local/libexec/cynic-host-exec", "network-baseline", "observe"]
            ],
            "denied_classes": ["shell", "raw_systemctl", "raw_package_manager", "unbounded_helper"],
            "bridge_manifest": bridge_manifest
        }))
        .map_err(|error| format!("failed to serialize security policy: {error}"))?,
        output_dir,
        &mut files,
    )?;

    Ok(FrameworkBootstrapBundleReport {
        schema: "cynic.framework_bootstrap.bundle.v1",
        status: "written",
        mutates: false,
        output_dir: output_dir.display().to_string(),
        files,
        entrypoint: "install.sh".to_string(),
        operator_command: "scp -r <bundle-dir> <node>:/tmp/cynic-framework-bootstrap && ssh <node> 'sudo /tmp/cynic-framework-bootstrap/install.sh && sudo /tmp/cynic-framework-bootstrap/verify.sh'".to_string(),
        maintenance_command: "sudo /tmp/cynic-framework-bootstrap/security-maintenance.sh".to_string(),
        target_runtime: "fresh_linux_node",
        security_invariants: vec![
            "runtime user is system principal cynic with nologin shell",
            "sudoers grants only exact cynic-host-exec subcommands",
            "bootstrap installs minimal OS packages through the detected package manager when missing",
            "sudoers is validated with visudo before verification",
            "direct shell/systemctl/package-manager sudo paths must remain denied",
            "verification emits evidence under /var/lib/cynic/bootstrap",
        ],
    })
}

fn write_file(
    path: PathBuf,
    content: String,
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    fs::write(&path, content)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    let relative = path
        .strip_prefix(output_dir)
        .map_err(|error| format!("failed to relativize {}: {error}", path.display()))?;
    files.push(relative.display().to_string());
    Ok(())
}

fn write_executable_file(
    path: PathBuf,
    content: String,
    output_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    write_file(path.clone(), content, output_dir, files)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("failed to chmod {}: {error}", path.display()))
}

fn bootstrap_install_script() -> String {
    r#"#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
BUNDLE="$ROOT/host-exec-bundle"
RUNTIME_USER="${CYNIC_RUNTIME_USER:-cynic}"
RUNTIME_GROUP="${CYNIC_RUNTIME_GROUP:-cynic}"
HELPER_TARGET="${CYNIC_HOST_EXEC_HELPER:-/usr/local/libexec/cynic-host-exec}"
SUDOERS_TARGET="${CYNIC_HOST_EXEC_SUDOERS:-/etc/sudoers.d/cynic-host-exec}"

if [ "$(id -u)" != "0" ]; then
  echo "install.sh must run as root" >&2
  exit 1
fi

install_bootstrap_packages() {
  if command -v sudo >/dev/null 2>&1 && command -v visudo >/dev/null 2>&1; then
    return 0
  fi
  if command -v apt-get >/dev/null 2>&1; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y sudo passwd
  elif command -v apk >/dev/null 2>&1; then
    apk add --no-cache sudo shadow
  elif command -v dnf >/dev/null 2>&1; then
    dnf install -y sudo shadow-utils
  elif command -v yum >/dev/null 2>&1; then
    yum install -y sudo shadow-utils
  else
    echo "no supported package manager found for sudo/shadow bootstrap" >&2
    exit 1
  fi
}

install_bootstrap_packages
command -v visudo >/dev/null 2>&1 || { echo "visudo is required after package bootstrap" >&2; exit 1; }
command -v sudo >/dev/null 2>&1 || { echo "sudo is required after package bootstrap" >&2; exit 1; }

NOLOGIN=/usr/sbin/nologin
[ -x "$NOLOGIN" ] || NOLOGIN=/sbin/nologin
[ -x "$NOLOGIN" ] || NOLOGIN=/bin/false

group_exists() {
  if command -v getent >/dev/null 2>&1; then
    getent group "$1" >/dev/null 2>&1
  else
    grep -q "^$1:" /etc/group
  fi
}

if command -v groupadd >/dev/null 2>&1; then
  groupadd --system --force "$RUNTIME_GROUP"
elif ! group_exists "$RUNTIME_GROUP"; then
  addgroup -S "$RUNTIME_GROUP"
fi

if ! id -u "$RUNTIME_USER" >/dev/null 2>&1; then
  if command -v useradd >/dev/null 2>&1; then
    useradd --system --home-dir "/home/$RUNTIME_USER" --create-home --gid "$RUNTIME_GROUP" --shell "$NOLOGIN" "$RUNTIME_USER"
  else
    adduser -S -D -h "/home/$RUNTIME_USER" -G "$RUNTIME_GROUP" -s "$NOLOGIN" "$RUNTIME_USER"
  fi
fi

install -d -o "$RUNTIME_USER" -g "$RUNTIME_GROUP" -m 0755 "/home/$RUNTIME_USER"
install -d -o "$RUNTIME_USER" -g "$RUNTIME_GROUP" -m 0755 /etc/cynic /var/lib/cynic /var/log/cynic /var/lib/cynic/bootstrap
install -d -o root -g root -m 0755 "$(dirname "$HELPER_TARGET")" "$(dirname "$SUDOERS_TARGET")"
install -o root -g root -m 0755 "$BUNDLE/cynic-host-exec" "$HELPER_TARGET"
install -o root -g root -m 0440 "$BUNDLE/cynic-host-exec.sudoers" "$SUDOERS_TARGET"
visudo -cf "$SUDOERS_TARGET"

"$HELPER_TARGET" principal-prepare --apply > /var/lib/cynic/bootstrap/principal-prepare.json
chown "$RUNTIME_USER:$RUNTIME_GROUP" /var/lib/cynic/bootstrap/principal-prepare.json

echo "cynic framework bootstrap installed: $HELPER_TARGET"
"#.to_string()
}

fn bootstrap_verify_script() -> String {
    r#"#!/bin/sh
set -eu

RUNTIME_USER="${CYNIC_RUNTIME_USER:-cynic}"
RUNTIME_GROUP="${CYNIC_RUNTIME_GROUP:-cynic}"
HELPER_TARGET="${CYNIC_HOST_EXEC_HELPER:-/usr/local/libexec/cynic-host-exec}"
SUDOERS_TARGET="${CYNIC_HOST_EXEC_SUDOERS:-/etc/sudoers.d/cynic-host-exec}"
EVIDENCE_DIR="${CYNIC_BOOTSTRAP_EVIDENCE_DIR:-/var/lib/cynic/bootstrap}"

install -d -o "$RUNTIME_USER" -g "$RUNTIME_GROUP" -m 0755 "$EVIDENCE_DIR"
test -x "$HELPER_TARGET"
test -f "$SUDOERS_TARGET"
visudo -cf "$SUDOERS_TARGET"

"$HELPER_TARGET" bridge-status > "$EVIDENCE_DIR/bridge-status.json"
sudo -u "$RUNTIME_USER" sudo -n "$HELPER_TARGET" principal-prepare > "$EVIDENCE_DIR/principal-plan-through-sudo.json"
sudo -u "$RUNTIME_USER" sudo -n "$HELPER_TARGET" network-baseline observe > "$EVIDENCE_DIR/network-baseline-observe.json" || true

chown "$RUNTIME_USER:$RUNTIME_GROUP" "$EVIDENCE_DIR"/*.json
echo "cynic framework bootstrap verified: $EVIDENCE_DIR"
"#.to_string()
}

fn bootstrap_security_maintenance_script() -> String {
    r#"#!/bin/sh
set -eu

RUNTIME_USER="${CYNIC_RUNTIME_USER:-cynic}"
HELPER_TARGET="${CYNIC_HOST_EXEC_HELPER:-/usr/local/libexec/cynic-host-exec}"
SUDOERS_TARGET="${CYNIC_HOST_EXEC_SUDOERS:-/etc/sudoers.d/cynic-host-exec}"

test -x "$HELPER_TARGET"
test -f "$SUDOERS_TARGET"
visudo -cf "$SUDOERS_TARGET"

grep -Fx "$RUNTIME_USER ALL=(root) NOPASSWD: $HELPER_TARGET principal-prepare" "$SUDOERS_TARGET" >/dev/null
grep -Fx "$RUNTIME_USER ALL=(root) NOPASSWD: $HELPER_TARGET principal-prepare --apply" "$SUDOERS_TARGET" >/dev/null
grep -Fx "$RUNTIME_USER ALL=(root) NOPASSWD: $HELPER_TARGET network-baseline observe" "$SUDOERS_TARGET" >/dev/null
if grep -Fx "$RUNTIME_USER ALL=(root) NOPASSWD: $HELPER_TARGET" "$SUDOERS_TARGET" >/dev/null; then
  echo "unbounded helper sudoers rule detected" >&2
  exit 1
fi
if grep -E '/bin/sh|/bin/bash| bash| systemctl| apt-get| dnf| apk' "$SUDOERS_TARGET" >/dev/null; then
  echo "forbidden raw sudo target detected" >&2
  exit 1
fi

sudo -u "$RUNTIME_USER" sudo -n "$HELPER_TARGET" principal-prepare >/dev/null
if sudo -u "$RUNTIME_USER" sudo -n /bin/sh -c true >/dev/null 2>&1; then
  echo "direct shell sudo unexpectedly allowed" >&2
  exit 1
fi
if sudo -u "$RUNTIME_USER" sudo -n systemctl status cynic-node.service >/dev/null 2>&1; then
  echo "direct systemctl sudo unexpectedly allowed" >&2
  exit 1
fi

echo "cynic framework bootstrap security maintenance passed"
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::write_framework_bootstrap_bundle;

    #[test]
    fn framework_bootstrap_bundle_writes_security_kit() {
        let out = std::env::temp_dir().join(format!(
            "cynic-framework-bootstrap-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));

        let report = write_framework_bootstrap_bundle(out.to_str().expect("path"))
            .expect("framework bootstrap bundle");

        assert_eq!(report.status, "written");
        assert!(out.join("host-exec-bundle/cynic-host-exec").is_file());
        assert!(out.join("install.sh").is_file());
        assert!(out.join("verify.sh").is_file());
        assert!(out.join("security-maintenance.sh").is_file());
        assert!(out.join("security-policy.json").is_file());
        std::fs::remove_dir_all(out).expect("cleanup");
    }
}
