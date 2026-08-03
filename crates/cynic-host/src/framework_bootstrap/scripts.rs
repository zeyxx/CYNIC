mod runtime_artifacts;

pub(crate) use runtime_artifacts::BootstrapRuntimeArtifact;
use runtime_artifacts::{install_activation_fragment, install_preflight_fragment, verify_fragment};

pub(crate) fn bootstrap_enroll_script() -> String {
    r#"#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

if [ "$(id -u)" != "0" ]; then
  echo "enroll.sh requires the registry-declared MachineEnrollment principal with uid 0" >&2
  exit 1
fi

"$ROOT/install.sh"
"$ROOT/verify.sh"
"#
    .to_string()
}

pub(crate) fn bootstrap_install_script(artifacts: &[BootstrapRuntimeArtifact]) -> String {
    r#"#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
BUNDLE="$ROOT/host-exec-bundle"
RUNTIME_USER="${CYNIC_RUNTIME_USER:-cynic}"
RUNTIME_GROUP="${CYNIC_RUNTIME_GROUP:-cynic}"
HELPER_TARGET="${CYNIC_HOST_EXEC_HELPER:-/usr/local/libexec/cynic-host-exec}"
SUDOERS_TARGET="${CYNIC_HOST_EXEC_SUDOERS:-/etc/sudoers.d/cynic-host-exec}"
CLI_TARGET="${CYNIC_STANDALONE_CLI:-/usr/local/bin/cynic}"
NODE_TARGET="${CYNIC_NODE_BINARY:-/usr/local/bin/cynic-node}"
NODE_CONTROL_CONFIG_TARGET="${CYNIC_NODE_CONTROL_CONFIG:-/etc/cynic/node-control.toml}"
NODE_SERVICE_TARGET="${CYNIC_NODE_SERVICE_UNIT:-/etc/systemd/system/cynic-node.service}"
REGISTRY_TARGET="${CYNIC_RUNTIME_REGISTRY:-/etc/cynic/registry.json}"
STORAGE_CONFIG_TARGET="${CYNIC_STORAGE_CONFIG:-/etc/cynic/storage.toml}"
LXC_BRIDGE_TARGET="${CYNIC_LXC_EXEC_BRIDGE_HELPER:-/usr/local/libexec/cynic-lxc-exec-bridge}"
LXC_BRIDGE_SUDOERS="${CYNIC_LXC_EXEC_BRIDGE_SUDOERS:-/etc/sudoers.d/cynic-lxc-exec-bridge}"
STATE_ROOT="${CYNIC_STATE_ROOT:-/var/lib/cynic}"
RUNTIME_INSTALL_ROOT="${CYNIC_RUNTIME_INSTALL_ROOT:-$STATE_ROOT/runtime}"
RUNTIME_ARTIFACT_INBOX="${CYNIC_RUNTIME_ARTIFACT_INBOX:-$STATE_ROOT/runtime-artifacts/inbox}"
BOOTSTRAP_RELEASE=""

if [ "$(id -u)" != "0" ]; then
  echo "install.sh must run as root" >&2
  exit 1
fi

verify_seed_integrity() {
  test -f "$ROOT/seed-artifacts.sha256" || {
    echo "seed checksum projection is missing" >&2
    exit 1
  }
  command -v sha256sum >/dev/null 2>&1 || {
    echo "sha256sum is required to verify the seed before host mutation" >&2
    exit 1
  }
  (
    cd "$ROOT"
    sha256sum -c seed-artifacts.sha256
  )
}

verify_seed_integrity
__CYNIC_RUNTIME_PREFLIGHT__

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

install -d -o root -g root -m 0755 "$(dirname "$HELPER_TARGET")" "$(dirname "$SUDOERS_TARGET")"
install -o root -g root -m 0755 "$BUNDLE/cynic-host-exec" "$HELPER_TARGET"
install -o root -g root -m 0440 "$BUNDLE/cynic-host-exec.sudoers" "$SUDOERS_TARGET"
if [ -f "$ROOT/etc/node-control.toml" ]; then
  install -d -o root -g "$RUNTIME_GROUP" -m 0755 "$(dirname "$NODE_CONTROL_CONFIG_TARGET")"
  install -o root -g "$RUNTIME_GROUP" -m 0640 "$ROOT/etc/node-control.toml" "$NODE_CONTROL_CONFIG_TARGET"
fi
if [ -f "$ROOT/etc/systemd/cynic-node.service" ]; then
  install -d -o root -g root -m 0755 "$(dirname "$NODE_SERVICE_TARGET")"
  install -o root -g root -m 0644 "$ROOT/etc/systemd/cynic-node.service" "$NODE_SERVICE_TARGET"
fi
if [ -f "$ROOT/etc/registry.json" ]; then
  install -d -o root -g "$RUNTIME_GROUP" -m 0755 "$(dirname "$REGISTRY_TARGET")"
  install -o root -g "$RUNTIME_GROUP" -m 0644 "$ROOT/etc/registry.json" "$REGISTRY_TARGET"
fi
if [ -f "$ROOT/etc/storage.toml" ]; then
  install -d -o root -g "$RUNTIME_GROUP" -m 0755 "$(dirname "$STORAGE_CONFIG_TARGET")"
  install -o root -g "$RUNTIME_GROUP" -m 0644 "$ROOT/etc/storage.toml" "$STORAGE_CONFIG_TARGET"
fi
if [ -f "$ROOT/lxc-exec-bridge-bundle/cynic-lxc-exec-bridge" ]; then
  install -d -o root -g root -m 0755 "$(dirname "$LXC_BRIDGE_TARGET")" "$(dirname "$LXC_BRIDGE_SUDOERS")"
  install -o root -g root -m 0755 "$ROOT/lxc-exec-bridge-bundle/cynic-lxc-exec-bridge" "$LXC_BRIDGE_TARGET"
  install -o root -g root -m 0440 "$ROOT/lxc-exec-bridge-bundle/cynic-lxc-exec-bridge.sudoers" "$LXC_BRIDGE_SUDOERS"
  visudo -cf "$LXC_BRIDGE_SUDOERS"
fi
visudo -cf "$SUDOERS_TARGET"

"$HELPER_TARGET" principal-prepare --apply > /var/lib/cynic/bootstrap/principal-prepare.json
chown "$RUNTIME_USER:$RUNTIME_GROUP" /var/lib/cynic/bootstrap/principal-prepare.json
install -d -o "$RUNTIME_USER" -g "$RUNTIME_GROUP" -m 0750 \
  "$RUNTIME_ARTIFACT_INBOX"
install -d -o "$RUNTIME_USER" -g "$RUNTIME_GROUP" -m 0755 \
  "$RUNTIME_INSTALL_ROOT" "$RUNTIME_INSTALL_ROOT/releases"
__CYNIC_RUNTIME_ACTIVATION__

echo "cynic framework bootstrap installed: $HELPER_TARGET"
"#
    .replace(
        "__CYNIC_RUNTIME_PREFLIGHT__",
        &install_preflight_fragment(artifacts),
    )
    .replace(
        "__CYNIC_RUNTIME_ACTIVATION__",
        &install_activation_fragment(artifacts),
    )
}

pub(crate) fn bootstrap_verify_script(artifacts: &[BootstrapRuntimeArtifact]) -> String {
    r#"#!/bin/sh
set -eu

RUNTIME_USER="${CYNIC_RUNTIME_USER:-cynic}"
RUNTIME_GROUP="${CYNIC_RUNTIME_GROUP:-cynic}"
HELPER_TARGET="${CYNIC_HOST_EXEC_HELPER:-/usr/local/libexec/cynic-host-exec}"
SUDOERS_TARGET="${CYNIC_HOST_EXEC_SUDOERS:-/etc/sudoers.d/cynic-host-exec}"
EVIDENCE_DIR="${CYNIC_BOOTSTRAP_EVIDENCE_DIR:-/var/lib/cynic/bootstrap}"
CLI_TARGET="${CYNIC_STANDALONE_CLI:-/usr/local/bin/cynic}"
NODE_TARGET="${CYNIC_NODE_BINARY:-/usr/local/bin/cynic-node}"
NODE_CONTROL_CONFIG_TARGET="${CYNIC_NODE_CONTROL_CONFIG:-/etc/cynic/node-control.toml}"
NODE_SERVICE_TARGET="${CYNIC_NODE_SERVICE_UNIT:-/etc/systemd/system/cynic-node.service}"
REGISTRY_TARGET="${CYNIC_RUNTIME_REGISTRY:-/etc/cynic/registry.json}"
STORAGE_CONFIG_TARGET="${CYNIC_STORAGE_CONFIG:-/etc/cynic/storage.toml}"
LXC_BRIDGE_TARGET="${CYNIC_LXC_EXEC_BRIDGE_HELPER:-/usr/local/libexec/cynic-lxc-exec-bridge}"
STATE_ROOT="${CYNIC_STATE_ROOT:-/var/lib/cynic}"
RUNTIME_INSTALL_ROOT="${CYNIC_RUNTIME_INSTALL_ROOT:-$STATE_ROOT/runtime}"
RUNTIME_ARTIFACT_INBOX="${CYNIC_RUNTIME_ARTIFACT_INBOX:-$STATE_ROOT/runtime-artifacts/inbox}"

install -d -o "$RUNTIME_USER" -g "$RUNTIME_GROUP" -m 0755 "$EVIDENCE_DIR"
test -d "$RUNTIME_INSTALL_ROOT"
test -d "$RUNTIME_ARTIFACT_INBOX"
test -x "$HELPER_TARGET"
test -f "$SUDOERS_TARGET"
visudo -cf "$SUDOERS_TARGET"
if [ -x "$CLI_TARGET" ]; then
  "$CLI_TARGET" runtime paths --json > "$EVIDENCE_DIR/standalone-cli-runtime-paths.json"
fi
if [ -x "$NODE_TARGET" ]; then
  test -f "$REGISTRY_TARGET"
  if [ -f "$STORAGE_CONFIG_TARGET" ]; then
    test -r "$STORAGE_CONFIG_TARGET"
  fi
  "$NODE_TARGET" registry-observation --json --registry "$REGISTRY_TARGET" > "$EVIDENCE_DIR/node-registry-observation.json"
  if [ -f "$NODE_CONTROL_CONFIG_TARGET" ]; then
    test -f "$NODE_SERVICE_TARGET"
    grep -F "ExecStart=/usr/local/bin/cynic-node control --config /etc/cynic/node-control.toml" "$NODE_SERVICE_TARGET" >/dev/null
    "$NODE_TARGET" control --config "$NODE_CONTROL_CONFIG_TARGET" --check > "$EVIDENCE_DIR/node-control-config-check.json"
  fi
fi
if [ -x "$LXC_BRIDGE_TARGET" ]; then
  sudo -u "$RUNTIME_USER" sudo -n "$LXC_BRIDGE_TARGET" --status > "$EVIDENCE_DIR/lxc-exec-bridge-status.json"
fi
"$HELPER_TARGET" bridge-status > "$EVIDENCE_DIR/bridge-status.json"
sudo -u "$RUNTIME_USER" sudo -n "$HELPER_TARGET" principal-prepare > "$EVIDENCE_DIR/principal-plan-through-sudo.json"
sudo -u "$RUNTIME_USER" sudo -n "$HELPER_TARGET" network-baseline observe > "$EVIDENCE_DIR/network-baseline-observe.json" || true
__CYNIC_RUNTIME_READ_BACK__

chown "$RUNTIME_USER:$RUNTIME_GROUP" "$EVIDENCE_DIR"/*.json
echo "cynic framework bootstrap verified: $EVIDENCE_DIR"
"#
    .replace("__CYNIC_RUNTIME_READ_BACK__", &verify_fragment(artifacts))
}

pub(crate) fn bootstrap_security_maintenance_script() -> String {
    r#"#!/bin/sh
set -eu

RUNTIME_USER="${CYNIC_RUNTIME_USER:-cynic}"
HELPER_TARGET="${CYNIC_HOST_EXEC_HELPER:-/usr/local/libexec/cynic-host-exec}"
SUDOERS_TARGET="${CYNIC_HOST_EXEC_SUDOERS:-/etc/sudoers.d/cynic-host-exec}"

test -x "$HELPER_TARGET"
test -f "$SUDOERS_TARGET"
visudo -cf "$SUDOERS_TARGET"

grep -Fx "$RUNTIME_USER ALL=(root) NOPASSWD: $HELPER_TARGET bridge-status" "$SUDOERS_TARGET" >/dev/null
grep -Fx "$RUNTIME_USER ALL=(root) NOPASSWD: $HELPER_TARGET principal-prepare" "$SUDOERS_TARGET" >/dev/null
grep -Fx "$RUNTIME_USER ALL=(root) NOPASSWD: $HELPER_TARGET service-reconcile --apply" "$SUDOERS_TARGET" >/dev/null
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
    use super::{BootstrapRuntimeArtifact, bootstrap_install_script, bootstrap_verify_script};

    fn artifacts() -> Vec<BootstrapRuntimeArtifact> {
        vec![BootstrapRuntimeArtifact {
            binary: "cynic-mcp".to_string(),
            install_path: "/usr/local/bin/cynic-mcp".to_string(),
        }]
    }

    #[test]
    fn bootstrap_activates_a_versioned_slot_behind_the_stable_mcp_entrypoint() {
        let script = bootstrap_install_script(&artifacts());

        assert!(script.contains("$RUNTIME_INSTALL_ROOT/releases/bootstrap-$seed_digest"));
        assert!(script.contains("ln -s \"releases/bootstrap-$seed_digest\" \"$current_next\""));
        assert!(
            script.contains("preflight_runtime_artifact 'cynic-mcp' '/usr/local/bin/cynic-mcp'")
        );
        assert!(script.contains("stage_runtime_artifact 'cynic-mcp' '/usr/local/bin/cynic-mcp'"));
        assert!(
            script.contains("activate_runtime_entrypoint 'cynic-mcp' '/usr/local/bin/cynic-mcp'")
        );
        assert!(script.contains(
            "test \"$(readlink \"$entrypoint\")\" = \"$RUNTIME_INSTALL_ROOT/current/$binary\""
        ));
        assert!(script.contains("cynic.runtime_seed.release_marker.v3"));
        assert!(script.contains("\"release_epoch\": 0"));
    }

    #[test]
    fn verification_reads_back_the_mcp_entrypoint_instead_of_trusting_install_exit() {
        let script = bootstrap_verify_script(&artifacts());

        assert!(
            script.contains("read_back_runtime_entrypoint 'cynic-mcp' '/usr/local/bin/cynic-mcp'")
        );
        assert!(script.contains("test -L \"$entrypoint\""));
        assert!(script.contains("test -x \"$entrypoint\""));
    }
}
