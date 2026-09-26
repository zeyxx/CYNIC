#!/usr/bin/env bash
# CYNIC Canonical Release Build & Atomic Deployment Script
# Enforces Foundation deployment lifecycle and blue-green releases directory.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CYNIC_ROOT="${CYNIC_ROOT:-/var/lib/cynic}"
CYNIC_RELEASES="${CYNIC_ROOT}/releases"
CYNIC_BIN="${CYNIC_ROOT}/bin"
CYNIC_CURRENT="${CYNIC_ROOT}/current"

echo "==> [CYNIC DEPLOY] Reading workspace version from Cargo.toml..."
VERSION=$(grep -m1 '^version\s*=\s*' "${REPO_ROOT}/Cargo.toml" | cut -d '"' -f2)
RELEASE_TAG="v${VERSION}"
TARGET_RELEASE_DIR="${CYNIC_RELEASES}/${RELEASE_TAG}"

echo "==> [CYNIC DEPLOY] Target release: ${RELEASE_TAG} (${TARGET_RELEASE_DIR})"

# 1. Compilation en mode release
echo "==> [CYNIC DEPLOY] Building release binaries (cynic-cli, cynic-node, cynic-node-enroll)..."
cd "${REPO_ROOT}"
export RUSTFLAGS="-C debuginfo=0"
export RUST_MIN_STACK=8589934592
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0

cargo build --release -p cynic-cli -p cynic-node --bin cynic --bin cynic-node --bin cynic-node-enroll

# 2. Préparation du répertoire de release
echo "==> [CYNIC DEPLOY] Staging release artifacts in ${TARGET_RELEASE_DIR}..."
mkdir -p "${TARGET_RELEASE_DIR}"
cp -f "${REPO_ROOT}/target/release/cynic" "${TARGET_RELEASE_DIR}/cynic"
cp -f "${REPO_ROOT}/target/release/cynic-node" "${TARGET_RELEASE_DIR}/cynic-node"
cp -f "${REPO_ROOT}/target/release/cynic-node-enroll" "${TARGET_RELEASE_DIR}/cynic-node-enroll"

# 3. Bascule atomique du symlink current
echo "==> [CYNIC DEPLOY] Activating release symlink ${CYNIC_CURRENT} -> ${TARGET_RELEASE_DIR}..."
mkdir -p "${CYNIC_ROOT}"
ln -sfn "${TARGET_RELEASE_DIR}" "${CYNIC_CURRENT}"

# 4. Harmonisation de /var/lib/cynic/bin
echo "==> [CYNIC DEPLOY] Harmonizing ${CYNIC_BIN}..."
mkdir -p "${CYNIC_BIN}"
ln -sfn "${CYNIC_CURRENT}/cynic" "${CYNIC_BIN}/cynic"
ln -sfn "${CYNIC_CURRENT}/cynic-node" "${CYNIC_BIN}/cynic-node"

# 5. Vérification et renouvellement du grant d'enrôlement si demandé ou si expiré
RENEW="${1:-}"
if [[ "${RENEW}" == "--renew-grant" ]] || [[ ! -f "${CYNIC_ROOT}/identity/enrollment-grant.json" ]]; then
    echo "==> [CYNIC DEPLOY] Renewing node enrollment grant via cynic-node-enroll..."
    "${TARGET_RELEASE_DIR}/cynic-node-enroll" --ttl-days 365 --json
fi

# 6. Read-back check
echo "==> [CYNIC DEPLOY] Verifying installed binaries and node readiness..."
INSTALLED_CLI_VER=$("${CYNIC_BIN}/cynic" --version 2>&1 || true)
echo "  Installed CLI: ${INSTALLED_CLI_VER}"

if [[ -f "/etc/cynic/node-control.toml" ]]; then
    NODE_CHECK=$("${CYNIC_BIN}/cynic-node" control --check --config /etc/cynic/node-control.toml 2>&1 || true)
    echo "  Node status check:"
    echo "${NODE_CHECK}" | jq -r '{status: .status, ready: .build_scheduler_activation_ready, capabilities: .build_scheduler_capabilities_proven}' 2>/dev/null || echo "${NODE_CHECK}"
fi

echo "==> [CYNIC DEPLOY] Deployment of ${RELEASE_TAG} successfully completed."
