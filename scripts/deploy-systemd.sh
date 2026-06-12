#!/usr/bin/env bash
# deploy-systemd.sh — Idempotent deploy of repo-managed systemd units
#
# Symlinks infra/systemd/*.{service,timer} → ~/.config/systemd/user/
# Shows diff before replacing. Runs daemon-reload after changes.
#
# Usage:
#   scripts/deploy-systemd.sh             # deploy (interactive diff)
#   scripts/deploy-systemd.sh --dry-run   # show what would change
#   scripts/deploy-systemd.sh --force     # skip diff confirmation

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE_DIR="$PROJECT_DIR/infra/systemd"
TARGET_DIR="${HOME}/.config/systemd/user"

DRY_RUN=false
FORCE=false
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=true ;;
        --force)   FORCE=true ;;
        *)         echo "Unknown flag: $arg"; exit 1 ;;
    esac
done

mkdir -p "$TARGET_DIR"

CHANGED=0
SKIPPED=0
CREATED=0

for src in "$SOURCE_DIR"/*.service "$SOURCE_DIR"/*.timer "$PROJECT_DIR"/organs/*/systemd/*.service "$PROJECT_DIR"/organs/*/systemd/*.timer; do
    [ -f "$src" ] || continue
    name="$(basename "$src")"

    # Skip template units (foo@.service) — they're instantiated, not deployed directly
    case "$name" in *@.*) continue ;; esac
    # Skip shell scripts (not systemd units)
    case "$name" in *.sh) continue ;; esac

    target="$TARGET_DIR/$name"
    resolved_src="$(cd "$(dirname "$src")" && pwd)/$(basename "$src")"

    # Case 1: symlink already points to the right place
    if [ -L "$target" ]; then
        current="$(readlink -f "$target" 2>/dev/null || true)"
        if [ "$current" = "$resolved_src" ]; then
            SKIPPED=$((SKIPPED + 1))
            continue
        fi
        # Symlink points elsewhere — show diff
        echo "DRIFT $name"
        echo "  current → $(readlink "$target")"
        echo "  wanted  → $resolved_src"
        if [ "$DRY_RUN" = true ]; then
            CHANGED=$((CHANGED + 1))
            continue
        fi
        if ! diff --color=auto "$target" "$src" 2>/dev/null; then
            true  # diff exits 1 on differences
        fi
        if [ "$FORCE" != true ]; then
            echo -n "  Replace? [Y/n] "
            read -r ans < /dev/tty || ans="y"
            case "$ans" in [nN]*) echo "  Skipped."; continue ;; esac
        fi
        ln -sf "$resolved_src" "$target"
        CHANGED=$((CHANGED + 1))
    elif [ -f "$target" ]; then
        # Regular file (not symlink) — show diff, offer to replace with symlink
        echo "FILE $name (not a symlink — will replace with symlink)"
        if [ "$DRY_RUN" = true ]; then
            CHANGED=$((CHANGED + 1))
            continue
        fi
        if ! diff --color=auto "$target" "$src" 2>/dev/null; then
            true
        fi
        if [ "$FORCE" != true ]; then
            echo -n "  Replace file with symlink? [Y/n] "
            read -r ans < /dev/tty || ans="y"
            case "$ans" in [nN]*) echo "  Skipped."; continue ;; esac
        fi
        rm "$target"
        ln -s "$resolved_src" "$target"
        CHANGED=$((CHANGED + 1))
    else
        # New unit — create symlink
        echo "NEW  $name"
        if [ "$DRY_RUN" = true ]; then
            CREATED=$((CREATED + 1))
            continue
        fi
        ln -s "$resolved_src" "$target"
        CREATED=$((CREATED + 1))
    fi
done

echo ""
echo "Summary: $CREATED new, $CHANGED changed, $SKIPPED up-to-date"

if [ "$DRY_RUN" = true ]; then
    echo "(dry-run — no changes applied)"
    exit 0
fi

if [ $((CREATED + CHANGED)) -gt 0 ]; then
    echo "Running daemon-reload..."
    systemctl --user daemon-reload
    echo "Done. Restart affected services manually if needed."
else
    echo "Nothing to do."
fi
