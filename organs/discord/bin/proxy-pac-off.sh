#!/usr/bin/env bash
set -euo pipefail
gsettings set org.gnome.system.proxy mode 'none'
printf 'proxy_pac=off\n'
