#!/usr/bin/env bash
# RECONSTRUCTED 2026-05-29 — Original lost in home directory reset. Content approximate.
#
# anteriorite-snapshot.sh — Preuve d'antériorité horodatée
#
# Crée une archive signée du codebase CYNIC avec :
# - Hash SHA-256 de chaque fichier source
# - Timestamp UTC
# - Archive tar.gz
# - Fichier de preuve (.proof) avec les hashes
#
# Usage: ./scripts/anteriorite-snapshot.sh [output_dir]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="${1:-$PROJECT_ROOT/snapshots}"
TIMESTAMP="$(date -u +%Y%m%d-%H%M%S)"
SNAPSHOT_NAME="cynic-anteriorite-${TIMESTAMP}"
SNAPSHOT_DIR="${OUTPUT_DIR}/${SNAPSHOT_NAME}"

echo "=== CYNIC Antériorité Snapshot ==="
echo "Timestamp: ${TIMESTAMP} UTC"
echo "Project:   ${PROJECT_ROOT}"
echo "Output:    ${SNAPSHOT_DIR}"
echo ""

# Create output directory
mkdir -p "${SNAPSHOT_DIR}"

# Directories to include in the snapshot
DIRS=(
    "cynic-kernel/src"
    "cynic-node"
    "cynic-askesis"
    "cynic-python"
    "cynic-mcp"
    "cynic-ui"
    "scripts"
    "docs"
    "data"
)

# Generate file hashes
echo "Generating SHA-256 hashes..."
HASH_FILE="${SNAPSHOT_DIR}/hashes.sha256"
: > "${HASH_FILE}"

for dir in "${DIRS[@]}"; do
    if [ -d "${PROJECT_ROOT}/${dir}" ]; then
        find "${PROJECT_ROOT}/${dir}" \
            -type f \
            \( -name "*.rs" -o -name "*.py" -o -name "*.ts" -o -name "*.tsx" \
               -o -name "*.js" -o -name "*.toml" -o -name "*.md" \
               -o -name "*.sh" -o -name "*.json" -o -name "*.yaml" \) \
            -not -path "*/target/*" \
            -not -path "*/node_modules/*" \
            -not -path "*/.venv/*" \
            -not -path "*/__pycache__/*" \
            -exec sha256sum {} + >> "${HASH_FILE}"
    fi
done

FILE_COUNT=$(wc -l < "${HASH_FILE}")
echo "Hashed ${FILE_COUNT} files"

# Git info
cd "${PROJECT_ROOT}"
GIT_HASH=$(git rev-parse HEAD 2>/dev/null || echo "no-git")
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")

# Create proof file
PROOF_FILE="${SNAPSHOT_DIR}/anteriorite.proof"
cat > "${PROOF_FILE}" <<EOF
=== CYNIC ANTÉRIORITÉ — PREUVE D'ANTÉRIORITÉ ===

Timestamp UTC:    ${TIMESTAMP}
Git Commit:       ${GIT_HASH}
Git Branch:       ${GIT_BRANCH}
Files Hashed:     ${FILE_COUNT}
Project:          CYNIC — Sovereign Judgment Infrastructure

--- MASTER HASH ---
$(sha256sum "${HASH_FILE}" | awk '{print $1}')

--- SYSTEM INFO ---
Hostname:         $(hostname)
Kernel:           $(uname -r)
Date (locale):    $(date)

--- VERIFICATION ---
To verify: sha256sum -c hashes.sha256
=================================================
EOF

echo ""
echo "Proof file: ${PROOF_FILE}"

# Create tar.gz archive
ARCHIVE="${OUTPUT_DIR}/${SNAPSHOT_NAME}.tar.gz"
cd "${OUTPUT_DIR}"
tar czf "${ARCHIVE}" "${SNAPSHOT_NAME}/"

# Hash the archive itself
ARCHIVE_HASH=$(sha256sum "${ARCHIVE}" | awk '{print $1}')

echo ""
echo "=== SNAPSHOT COMPLETE ==="
echo "Archive:      ${ARCHIVE}"
echo "Archive hash: ${ARCHIVE_HASH}"
echo "Files:        ${FILE_COUNT}"
echo "Timestamp:    ${TIMESTAMP} UTC"
echo ""
echo "Pour preuve légale, envoyer archive + hash à un tiers de confiance"
echo "(email horodaté, blockchain timestamp, huissier numérique)."
