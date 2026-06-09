#!/bin/bash
set -euo pipefail
export LC_ALL=C

PROJECT_DIR="${CYNIC_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
STATE_DIR="$PROJECT_DIR/infra/organ-vercel"
STATE_FILE="$STATE_DIR/state.json"
AUDIT_FILE="$STATE_DIR/audit.jsonl"
REPO='zeyxx/CYNIC'
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

mkdir -p "$STATE_DIR"

deployments_json='[]'
deployment_error=''
if command -v gh >/dev/null 2>&1; then
  if deployments_json_raw="$(gh api "repos/$REPO/deployments" --paginate 2>/dev/null)"; then
    deployments_json="${deployments_json_raw:-[]}"
  else
    deployment_error='gh api repos/zeyxx/CYNIC/deployments failed'
  fi
else
  deployment_error='gh cli unavailable'
fi

export DEPLOYMENTS_JSON="$deployments_json"
python3 - "$TIMESTAMP" "$deployment_error" <<'PY' > "$STATE_FILE"
import json
import os
import sys
from datetime import datetime, timezone

timestamp = sys.argv[1]
deployment_error = sys.argv[2]
deployments = json.loads(os.environ.get('DEPLOYMENTS_JSON', '[]'))

def parse(ts):
    if not ts:
        return None
    return datetime.fromisoformat(ts.replace('Z', '+00:00'))

latest = max((parse(d.get('created_at')) for d in deployments if parse(d.get('created_at'))), default=None)
production = [d for d in deployments if (d.get('environment') or '').lower() == 'production']
preview = [d for d in deployments if (d.get('environment') or '').lower() == 'preview']
latest_prod = max((parse(d.get('created_at')) for d in production if parse(d.get('created_at'))), default=None)
latest_prev = max((parse(d.get('created_at')) for d in preview if parse(d.get('created_at'))), default=None)
now = datetime.now(timezone.utc)
hours_since_latest = round((now - latest).total_seconds() / 3600, 2) if latest else None

state = {
    'version': '1.0.0',
    'updated': timestamp,
    'source': 'organ-vercel-state',
    'deployment_count': len(deployments),
    'production_count': len(production),
    'preview_count': len(preview),
    'latest_deploy_at': latest.isoformat().replace('+00:00', 'Z') if latest else None,
    'latest_production_at': latest_prod.isoformat().replace('+00:00', 'Z') if latest_prod else None,
    'latest_preview_at': latest_prev.isoformat().replace('+00:00', 'Z') if latest_prev else None,
    'hours_since_latest': hours_since_latest,
    'deployment_gap_note': 'latest deployment age measured from GitHub deployments API',
    'alerts': ([deployment_error] if deployment_error else []) + ([f'no deployment in {hours_since_latest}h'] if hours_since_latest is not None and hours_since_latest > 4 else []),
}
print(json.dumps(state, indent=2))
PY

jq -c -n --arg timestamp "$TIMESTAMP" --argjson state "$(cat "$STATE_FILE")" '{timestamp:$timestamp,action:"observation",outcome:"success",details:$state}' >> "$AUDIT_FILE"
cat "$STATE_FILE"
