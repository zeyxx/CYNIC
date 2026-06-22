#!/usr/bin/env bash
set -euo pipefail

if command -v gsettings >/dev/null 2>&1; then
  gsettings set org.gnome.system.proxy mode 'none' || true
fi

printf 'proxy_rollback=done\n'
printf 'note=if_chrome_was_started_with_proxy_flags_close_that_window_or_profile\n'
