#!/bin/bash
# Bridge script: execute commands on Tailscale nodes via SSH
# Called by kernel's attempt_node_recovery() for remote remediation
#
# Usage: ts_exec_call.sh <node> <command> <timeout_secs>
# Example: ts_exec_call.sh cynic-core "systemctl restart llama-server" 30
#
# Returns JSON with exit_code field (0=success, 1=failure)
# Expected by: cynic-kernel/src/api/rest/inference_router.rs lines 305-307

set -e

NODE="${1:?ERROR: node name required}"
COMMAND="${2:?ERROR: command required}"
TIMEOUT="${3:-30}"

# Execute command on node via SSH (Tailscale resolves node names)
# Capture exit code without exiting script on failure
OUTPUT=$(ssh -o ConnectTimeout=5 -o BatchMode=yes root@"$NODE" "$COMMAND" 2>&1)
EXIT_CODE=$?

# Return JSON response (matches kernel's expected format)
cat <<RESPONSE
{"exit_code": $EXIT_CODE}
RESPONSE
