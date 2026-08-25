#!/bin/sh
# pct-bounded-exec — root-owned bounded executor for the Proxmox host.
#
# Policy enforcement point projected from
# infra/privilege/pve-pct-helper.policy.json (schema
# cynic.host.pve_helper_policy.v1). The sudoers membrane grants ONLY exact
# argv of the form:
#   /usr/local/sbin/pct-bounded-exec <action> [fixed args]
# This executor never accepts arbitrary argv: the dispatch table below is the
# executable counterpart of the declared capabilities. Every invocation is
# journaled via syslog (tag: pct-bounded-exec).
#
# Installed by HumanAdmin bootstrap; invoked by RuntimeMachineIdentity
# through `sudo -n`.

set -eu

PCT=/usr/sbin/pct
LOG_TAG=pct-bounded-exec

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
	# capability id: pct-list
	log "list"
	exec "$PCT" list
	;;
probe)
	# capability id: pct-probe (read-only observation of the git CT)
	CID="$(find_git_ct)"
	[ -n "$CID" ] || die "no git/forgejo CT found"
	log "probe ct=$CID"
	echo "TARGET_CT=$CID"
	"$PCT" exec "$CID" -- hostname
	"$PCT" exec "$CID" -- ls -la /etc/forgejo/app.ini
	"$PCT" exec "$CID" -- sh -c 'id forgejo; ls /usr/local/bin/forgejo /usr/bin/forgejo 2>/dev/null; which sudo || true'
	;;
generate-token)
	# capability id: forgejo-machine-token. Scopes are POLICY and frozen here;
	# callers may only pass: <username> <token-name> [--scopes write:repository]
	if [ $# -eq 4 ] && [ "$3" = "--scopes" ] && [ "$4" = "write:repository" ]; then
		SCOPE_ARGS="--scopes write:repository"
	elif [ $# -eq 2 ]; then
		SCOPE_ARGS=""
	else
		die "usage: generate-token <username> <token-name> [[--scopes write:repository]]"
	fi
	USER_NAME="$1"
	TOKEN_NAME="$2"
	printf '%s' "$USER_NAME" | grep -qE '^[A-Za-z0-9._-]{1,64}$' ||
		die "username fails ^[A-Za-z0-9._-]{1,64}\$"
	printf '%s' "$TOKEN_NAME" | grep -qE '^[A-Za-z0-9._-]{1,64}$' ||
		die "token-name fails ^[A-Za-z0-9._-]{1,64}\$"
	CID="$(find_git_ct)"
	[ -n "$CID" ] || die "no git/forgejo CT found"
	log "generate-token ct=$CID user=$USER_NAME token_name=$TOKEN_NAME scopes=write:repository"
	"$PCT" exec "$CID" -- su -s /bin/sh forgejo -c \
		"forgejo admin user generate-access-token --config /etc/forgejo/app.ini --username '$USER_NAME' --token-name '$TOKEN_NAME'$SCOPE_ARGS" ||
		die "token generation failed inside ct=$CID"
	;;
*)
	die "unknown action '${cmd:-<empty>}' (allowlist: list | probe | generate-token)"
	;;
esac
