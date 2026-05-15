#!/usr/bin/env bash
# CYNIC — Install repo-managed systemd user units as symlinks.
set -euo pipefail

PROJECT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
SYSTEMD_USER_DIR="${SYSTEMD_USER_DIR_OVERRIDE:-$HOME/.config/systemd/user}"

managed_units=(
    "infra/systemd/cynic-kernel.service"
    "infra/systemd/surrealdb.service"
    "infra/systemd/cynic-healthcheck.service"
    "infra/systemd/cynic-healthcheck.timer"
    "infra/systemd/surrealdb-backup.service"
    "infra/systemd/surrealdb-backup.timer"
    "infra/systemd/llama-server.service"
    "infra/systemd/hermes-proxy.service"
    "infra/systemd/hermes-x-ingest.service"
    "infra/systemd/hermes-curation.timer"
    "infra/systemd/hermes-data-organism.timer"
    "infra/systemd/hermes-feedback-loop.timer"
    "infra/systemd/hermes-gemini-briefing.timer"
    "infra/systemd/hermes-k15-consumer.timer"
    "infra/systemd/hermes-navigator.timer"
    "infra/systemd/hermes-search-executor.timer"
    "infra/systemd/hermes-search-generator.timer"
)

mkdir -p "$SYSTEMD_USER_DIR"

for rel in "${managed_units[@]}"; do
    src="$PROJECT_DIR/$rel"
    dst="$SYSTEMD_USER_DIR/$(basename "$rel")"
    [ -f "$src" ] || {
        echo "ERROR: missing repo unit $src" >&2
        exit 1
    }
    ln -sfn "$src" "$dst"
    echo "Linked: $dst -> $src"
done

systemctl --user daemon-reload
bash "$PROJECT_DIR/scripts/verify-systemd-units.sh"
echo "Done. Repo-managed systemd units are active."
