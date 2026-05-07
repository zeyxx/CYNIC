#!/bin/bash
# Sync ~/.cynic-env (shell format) to ~/.config/cynic/env (systemd format)
# Removes 'export' prefixes and writes to systemd-compatible location

set -e

CYNIC_ENV="$HOME/.cynic-env"
SYSTEMD_ENV="$HOME/.config/cynic/env"
BACKUP="${SYSTEMD_ENV}.bak.$(date +%s)"

# Ensure target directory exists
mkdir -p "$(dirname "$SYSTEMD_ENV")"

# Backup existing file
if [[ -f "$SYSTEMD_ENV" ]]; then
  cp "$SYSTEMD_ENV" "$BACKUP"
  echo "Backed up $SYSTEMD_ENV → $BACKUP"
fi

# Convert shell format to systemd format
# Remove 'export ' prefix and write to systemd location
{
  while IFS= read -r line; do
    # Skip empty lines and comments
    [[ -z "$line" || "$line" =~ ^# ]] && continue

    # Remove 'export ' prefix if present
    line="${line#export }"

    # Write to systemd file (KEY=value, no quotes)
    echo "$line"
  done < "$CYNIC_ENV"
} > "$SYSTEMD_ENV"

echo "✓ Synced $CYNIC_ENV → $SYSTEMD_ENV ($(wc -l < "$SYSTEMD_ENV") lines)"
echo "⚠️  Verify: systemctl --user set-environment $(head -1 $SYSTEMD_ENV | cut -d= -f1)=test"
