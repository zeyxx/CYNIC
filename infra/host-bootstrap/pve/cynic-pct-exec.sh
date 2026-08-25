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
