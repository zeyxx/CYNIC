#!/bin/bash
set -euo pipefail
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HERMES_HOME="${PROJECT_DIR}/.hermes_ouroboros"
CONFIG_FILE="${HOME}/.config/cynic/backends.toml"
LOG_FILE="${PROJECT_DIR}/cynic-ouroboros.log"
RUN_DATE="${OUROBOROS_DATE_OVERRIDE:-$(date -I)}"
RUN_STARTED_AT="$(date --iso-8601=seconds -u)"
RUN_START_EPOCH="$(date +%s)"

# Load local runtime env so nightly works the same way under systemd and shell.
if [ -f "${HOME}/.cynic-env" ]; then
    # shellcheck disable=SC1090
    source "${HOME}/.cynic-env"
fi
if [ -f "${HOME}/.config/cynic/env" ]; then
    # shellcheck disable=SC1090
    source "${HOME}/.config/cynic/env"
fi

# Extract SoT Parameters from backends.toml (single source of truth)
BACKEND_URL=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu']['base_url'])" 2>/dev/null)
BACKEND_MODEL=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu']['model'])" 2>/dev/null)
BACKEND_API_KEY_ENV=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu'].get('api_key_env', ''))" 2>/dev/null)

if [ -z "${BACKEND_URL:-}" ] || [ -z "${BACKEND_MODEL:-}" ]; then
    echo "✗ Missing qwen35-9b-gpu backend config in ${CONFIG_FILE}"
    exit 1
fi
if [ -z "${BACKEND_API_KEY_ENV:-}" ]; then
    echo "✗ Missing api_key_env for qwen35-9b-gpu in ${CONFIG_FILE}"
    exit 1
fi
BACKEND_API_KEY="${!BACKEND_API_KEY_ENV:-}"
if [ -z "${BACKEND_API_KEY:-}" ]; then
    echo "✗ Backend API key env '${BACKEND_API_KEY_ENV}' is not set"
    exit 1
fi
if [ -z "${CYNIC_API_KEY:-}" ]; then
    echo "✗ CYNIC_API_KEY is not set"
    exit 1
fi
if [ -z "${CYNIC_REST_ADDR:-}" ]; then
    echo "✗ CYNIC_REST_ADDR is not set"
    exit 1
fi

PATH="${HOME}/bin:${HOME}/.local/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"

mkdir -p "${HERMES_HOME}"

# ── Generate plan ──────────────────────────────────────────────────
PLAN_JSON="${HERMES_HOME}/ouroboros-plan.json"
REPORT_JSON="${OUROBOROS_REPORT_JSON_OVERRIDE:-${HERMES_HOME}/ouroboros-report.json}"
python3 "${PROJECT_DIR}/scripts/ouroboros_scorecard.py" --date "${RUN_DATE}" > "${PLAN_JSON}"

# Persist "running" status via kernel /observe (K10: agents use the platform)
if [ -n "${CYNIC_REST_ADDR:-}" ]; then
    ADDR="${CYNIC_REST_ADDR}"
    [[ "${ADDR}" != http* ]] && ADDR="http://${ADDR}"
    curl -s -X POST "${ADDR}/observe" \
        -H "Authorization: Bearer ${CYNIC_API_KEY}" \
        -H "Content-Type: application/json" \
        -d "{\"tool\":\"ouroboros\",\"target\":\"run ${RUN_DATE} starting\",\"domain\":\"research\",\"status\":\"running\"}" \
        >/dev/null 2>&1 || true
fi

# ── Run sovereign analysis ─────────────────────────────────────────
# Direct inference via Tailscale to sovereign GPU. No third-party agent.
export OPENAI_BASE_URL="${BACKEND_URL}"
export OPENAI_API_KEY="${BACKEND_API_KEY}"
export HERMES_MODEL="${BACKEND_MODEL}"

hermes_status="failed"
if python3 "${PROJECT_DIR}/scripts/ouroboros_run.py" \
    --plan "${PLAN_JSON}" \
    --report "${REPORT_JSON}" \
    2>&1 | tee -a "${LOG_FILE}"; then
    echo "✓ Sovereign run completed"
    hermes_status="completed"
else
    echo "✗ Sovereign run failed"
fi

# ── Persist final status via kernel /observe (K10: no agent-owned DBs) ──
RUN_FINISHED_AT="$(date --iso-8601=seconds -u)"
RUN_FINISH_EPOCH="$(date +%s)"
RUN_DURATION_S="$(python3 -c "print(round(${RUN_FINISH_EPOCH} - ${RUN_START_EPOCH}, 3))")"

if [ -n "${CYNIC_REST_ADDR:-}" ]; then
    ADDR="${CYNIC_REST_ADDR}"
    [[ "${ADDR}" != http* ]] && ADDR="http://${ADDR}"
    # Summary observation — the kernel tracks it, not a side DB
    REPOS_DONE=$(python3 -c "import json; r=json.load(open('${REPORT_JSON}')); print(r['run']['repos_completed'])" 2>/dev/null || echo "0")
    curl -s -X POST "${ADDR}/observe" \
        -H "Authorization: Bearer ${CYNIC_API_KEY}" \
        -H "Content-Type: application/json" \
        -d "{\"tool\":\"ouroboros\",\"target\":\"run ${RUN_DATE} ${hermes_status}: ${REPOS_DONE} repos in ${RUN_DURATION_S}s\",\"domain\":\"research\",\"status\":\"${hermes_status}\",\"context\":\"{\\\"duration_s\\\":${RUN_DURATION_S},\\\"model\\\":\\\"${BACKEND_MODEL}\\\",\\\"backend\\\":\\\"qwen35-9b-gpu\\\"}\"}" \
        >/dev/null 2>&1 || true
fi

if [ "${hermes_status}" != "completed" ]; then
    exit 1
fi
