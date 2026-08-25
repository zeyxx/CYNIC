#!/bin/sh
# install-bounded-pct.sh — HumanAdmin bootstrap for the Proxmox host (pve).
#
# One-shot privileged gesture: installs the root-owned bounded executor, the
# one-line sudoers membrane, and the pct PATH shim. Idempotent.
#
# Run as:  ssh -t proxmox-forge 'sudo sh -s' < infra/host-bootstrap/pve/install-bounded-pct.sh
# (or pipe it any way you like — it needs a real root shell once).

set -eu

echo "== 1. executor"
install -m 0755 -o root -g root /dev/stdin /usr/local/sbin/cynic-pct-exec <<'EXECUTOR'
@@EXECUTOR@@
EXECUTOR

echo "== 2. sudoers membrane"
install -m 0440 -o root -g root /dev/stdin /etc/sudoers.d/cynic-bounded-pct <<'SUDOERS'
@@SUDOERS@@
SUDOERS
visudo -cf /etc/sudoers.d/cynic-bounded-pct

echo "== 3. pct PATH shim"
[ -e /usr/local/bin/pct ] || ln -s /usr/sbin/pct /usr/local/bin/pct
ls -la /usr/local/bin/pct

echo "== 4. read-back"
/usr/local/sbin/cynic-pct-exec badaction || true
sudo -l -U cynic | grep -A1 cynic-pct-exec || true
echo "BOOTSTRAP_DONE"
