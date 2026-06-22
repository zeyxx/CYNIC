#!/usr/bin/env bash
set -euo pipefail
PAC="http://127.0.0.1:8892/proxy-policy.pac"
gsettings set org.gnome.system.proxy mode 'auto'
gsettings set org.gnome.system.proxy autoconfig-url "$PAC"
printf 'proxy_pac=on\n'
printf 'pac=%s\n' "$PAC"
