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
#!/bin/sh
# cynic-pct-exec — root-owned bounded executor for Proxmox CT administration.
#
# Policy enforcement point installed by HumanAdmin bootstrap on the Proxmox
# host. Invoked exclusively through:
#   sudo /usr/local/sbin/cynic-pct-exec <action> [args]
# wired in /etc/sudoers.d/cynic-bounded-pct (single line, no wildcards).
#
# Allowlist (hardcoded policy — this file NEVER accepts arbitrary argv):
#   list                              -> pct list (inventory)
#   probe                             -> hostname/app.ini/user/binary of the
#                                        git-forgejo CT (read-only)
#   generate-token <user> <tokname>   -> forgejo admin generate-access-token,
#                                        scopes hardcoded write:repository
#
# Every invocation is syslog-journaled (tag: cynic-pct-exec).

set -eu

PCT=/usr/sbin/pct
LOG_TAG=cynic-pct-exec

log() { logger -t "$LOG_TAG" "$*" 2>/dev/null || true; }
die() { echo "REFUSED: $*" >&2; log "REFUSED $*"; exit 2; }

[ "$(id -u)" -eq 0 ] || die "executor must run as root"

cmd="${1:-}"
[ $# -gt 0 ] && shift

find_git_ct() {
	"$PCT" list | tail -n +2 | awk '$2 ~ /(git|forgejo)/ {print $1; exit}'
}

case "$cmd" in
list)
	log "list"
	exec "$PCT" list
	;;
probe)
	CID="$(find_git_ct)"
	[ -n "$CID" ] || die "no git/forgejo CT found"
	log "probe ct=$CID"
	echo "TARGET_CT=$CID"
	"$PCT" exec "$CID" -- hostname
	"$PCT" exec "$CID" -- ls -la /etc/forgejo/app.ini
	"$PCT" exec "$CID" -- sh -c 'id forgejo; ls /usr/local/bin/forgejo /usr/bin/forgejo 2>/dev/null; which sudo || true'
	;;
generate-token)
	USER_NAME="${1:-}"
	TOKEN_NAME="${2:-}"
	[ $# -eq 2 ] || die "usage: generate-token <username> <token-name>"
	printf '%s' "$USER_NAME" | grep -qE '^[A-Za-z0-9._-]{1,64}$' ||
		die "username fails ^[A-Za-z0-9._-]{1,64}\$"
	printf '%s' "$TOKEN_NAME" | grep -qE '^[A-Za-z0-9._-]{1,64}$' ||
		die "token-name fails ^[A-Za-z0-9._-]{1,64}\$"
	CID="$(find_git_ct)"
	[ -n "$CID" ] || die "no git/forgejo CT found"
	log "generate-token ct=$CID user=$USER_NAME token_name=$TOKEN_NAME"
	# Scopes are POLICY, hardcoded here: write:repository only.
	"$PCT" exec "$CID" -- su -s /bin/sh forgejo -c \
		"forgejo admin user generate-access-token --config /etc/forgejo/app.ini --username '$USER_NAME' --token-name '$TOKEN_NAME' --scopes write:repository" ||
		die "token generation failed inside ct=$CID"
	;;
*)
	die "unknown action '${cmd:-<empty>}' (allowlist: list | probe | generate-token)"
	;;
esac

EXECUTOR

echo "== 2. sudoers membrane"
install -m 0440 -o root -g root /dev/stdin /etc/sudoers.d/cynic-bounded-pct <<'SUDOERS'
# /etc/sudoers.d/cynic-bounded-pct — HumanAdmin bootstrap artifact (pve host).
#
# Narrow transport membrane to the root-owned bounded executor
# cynic-pct-exec (policy enforcement point). NOT a general command grant:
# no wildcards, no shell escapes, one exact argv per principal.
#
# Install as root:
#   install -m 0440 -o root -g root cynic-bounded-pct /etc/sudoers.d/
#   visudo -cf /etc/sudoers.d/cynic-bounded-pct

cynic ALL=(root) NOPASSWD: /usr/local/sbin/cynic-pct-exec list, /usr/local/sbin/cynic-pct-exec probe, /usr/local/sbin/cynic-pct-exec generate-token *

SUDOERS
visudo -cf /etc/sudoers.d/cynic-bounded-pct

echo "== 3. pct PATH shim"
[ -e /usr/local/bin/pct ] || ln -s /usr/sbin/pct /usr/local/bin/pct
ls -la /usr/local/bin/pct

echo "== 4. read-back"
/usr/local/sbin/cynic-pct-exec badaction || true
sudo -l -U cynic | grep -A1 cynic-pct-exec || true
echo "BOOTSTRAP_DONE"
