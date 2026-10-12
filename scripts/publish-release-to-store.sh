#!/usr/bin/env bash
# CYNIC — Publication canonique des binaires vers le Runtime Artifact Store
# Source: 10.10.10.119 (cynic-builder) -> Target: 10.10.10.103 (/data/runtime-releases)

set -euo pipefail

STORE_HOST="10.10.10.103"
STORE_ROOT="/data/runtime-releases"
TARGET_DIR="${1:-target/release}"
VERSION="${2:-$(cargo pkgid | cut -d# -f2 | cut -d: -f2 2>/dev/null || echo "v0.8.0")}"

if [[ ! "$VERSION" =~ ^v ]]; then
    VERSION="v${VERSION}"
fi

echo "==> [CYNIC PUBLISH] Publication vers le store ${STORE_HOST} (version ${VERSION})"

if [ ! -f "${TARGET_DIR}/cynic" ]; then
    echo "ERROR: Binaire cynic introuvable dans ${TARGET_DIR}/cynic" >&2
    exit 1
fi

STAGING_DIR="$(mktemp -d /tmp/cynic-publish-XXXXXX)"
trap 'rm -rf "${STAGING_DIR}"' EXIT

cp "${TARGET_DIR}/cynic" "${STAGING_DIR}/"
(cd "${STAGING_DIR}" && sha256sum cynic > SHA256SUMS)

echo "==> [CYNIC PUBLISH] Envoi SSH/SCP vers ${STORE_HOST}:${STORE_ROOT}/${VERSION}/"
ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@"${STORE_HOST}" "mkdir -p '${STORE_ROOT}/${VERSION}'"
scp -o BatchMode=yes -o StrictHostKeyChecking=no "${STAGING_DIR}/cynic" "${STAGING_DIR}/SHA256SUMS" root@"${STORE_HOST}":"${STORE_ROOT}/${VERSION}/"

echo "==> [CYNIC PUBLISH] Mise à jour du symlink 'latest' et permissions"
ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@"${STORE_HOST}" "cd '${STORE_ROOT}' && rm -rf latest && ln -s '${VERSION}' latest && chown -R cynic:cynic '${STORE_ROOT}'"

echo "==> [CYNIC PUBLISH] Vérification du point de téléchargement HTTP :"
curl -fsS "http://${STORE_HOST}:8443/latest/SHA256SUMS"
echo "==> [CYNIC PUBLISH] Publication réussie."
