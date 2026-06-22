#!/usr/bin/env bash
set -euo pipefail
cat >&2 <<'MSG'
Refusing blind global proxy activation.
Use proxy-preflight.sh first and prefer a PAC/browser-scoped attachment.
If you intentionally want global proxy mode, run the gsettings commands manually.
MSG
exit 2
gsettings set org.gnome.system.proxy mode 'manual'
gsettings set org.gnome.system.proxy.http host '127.0.0.1'
gsettings set org.gnome.system.proxy.http port 8890
gsettings set org.gnome.system.proxy.https host '127.0.0.1'
gsettings set org.gnome.system.proxy.https port 8890
gsettings set org.gnome.system.proxy ignore-hosts "['localhost', '127.0.0.0/8', '::1']"
