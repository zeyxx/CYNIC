pub(crate) fn bootstrap_install_script() -> String {
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
REGISTRY_TARGET="${CYNIC_RUNTIME_REGISTRY:-/etc/cynic/registry.json}"
STORAGE_CONFIG_TARGET="${CYNIC_STORAGE_CONFIG:-/etc/cynic/storage.toml}"
LXC_BRIDGE_TARGET="${CYNIC_LXC_EXEC_BRIDGE_HELPER:-/usr/local/libexec/cynic-lxc-exec-bridge}"
LXC_BRIDGE_SUDOERS="${CYNIC_LXC_EXEC_BRIDGE_SUDOERS:-/etc/sudoers.d/cynic-lxc-exec-bridge}"

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
if [ -x "$ROOT/bin/cynic" ]; then
  install -d -o root -g root -m 0755 "$(dirname "$CLI_TARGET")"
  install -o root -g root -m 0755 "$ROOT/bin/cynic" "$CLI_TARGET"
fi
if [ -x "$ROOT/bin/cynic-node" ]; then
  install -d -o root -g root -m 0755 "$(dirname "$NODE_TARGET")"
  install -o root -g root -m 0755 "$ROOT/bin/cynic-node" "$NODE_TARGET"
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

echo "cynic framework bootstrap installed: $HELPER_TARGET"
"#.to_string()
}

pub(crate) fn bootstrap_verify_script() -> String {
    r#"#!/bin/sh
set -eu

RUNTIME_USER="${CYNIC_RUNTIME_USER:-cynic}"
RUNTIME_GROUP="${CYNIC_RUNTIME_GROUP:-cynic}"
HELPER_TARGET="${CYNIC_HOST_EXEC_HELPER:-/usr/local/libexec/cynic-host-exec}"
SUDOERS_TARGET="${CYNIC_HOST_EXEC_SUDOERS:-/etc/sudoers.d/cynic-host-exec}"
EVIDENCE_DIR="${CYNIC_BOOTSTRAP_EVIDENCE_DIR:-/var/lib/cynic/bootstrap}"
CLI_TARGET="${CYNIC_STANDALONE_CLI:-/usr/local/bin/cynic}"
NODE_TARGET="${CYNIC_NODE_BINARY:-/usr/local/bin/cynic-node}"
REGISTRY_TARGET="${CYNIC_RUNTIME_REGISTRY:-/etc/cynic/registry.json}"
STORAGE_CONFIG_TARGET="${CYNIC_STORAGE_CONFIG:-/etc/cynic/storage.toml}"
LXC_BRIDGE_TARGET="${CYNIC_LXC_EXEC_BRIDGE_HELPER:-/usr/local/libexec/cynic-lxc-exec-bridge}"

install -d -o "$RUNTIME_USER" -g "$RUNTIME_GROUP" -m 0755 "$EVIDENCE_DIR"
test -x "$HELPER_TARGET"
test -f "$SUDOERS_TARGET"
visudo -cf "$SUDOERS_TARGET"
if [ -x "$CLI_TARGET" ]; then
  "$CLI_TARGET" runtime paths --json > "$EVIDENCE_DIR/standalone-cli-runtime-paths.json"
fi
if [ -x "$NODE_TARGET" ]; then
  test -f "$REGISTRY_TARGET"
  test -f "$STORAGE_CONFIG_TARGET"
  "$NODE_TARGET" registry-observation --json --registry "$REGISTRY_TARGET" > "$EVIDENCE_DIR/node-registry-observation.json"
fi
if [ -x "$LXC_BRIDGE_TARGET" ]; then
  sudo -u "$RUNTIME_USER" sudo -n "$LXC_BRIDGE_TARGET" --status > "$EVIDENCE_DIR/lxc-exec-bridge-status.json"
fi
"$HELPER_TARGET" bridge-status > "$EVIDENCE_DIR/bridge-status.json"
sudo -u "$RUNTIME_USER" sudo -n "$HELPER_TARGET" principal-prepare > "$EVIDENCE_DIR/principal-plan-through-sudo.json"
sudo -u "$RUNTIME_USER" sudo -n "$HELPER_TARGET" network-baseline observe > "$EVIDENCE_DIR/network-baseline-observe.json" || true

chown "$RUNTIME_USER:$RUNTIME_GROUP" "$EVIDENCE_DIR"/*.json
echo "cynic framework bootstrap verified: $EVIDENCE_DIR"
"#.to_string()
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
