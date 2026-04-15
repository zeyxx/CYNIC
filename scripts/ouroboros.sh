#!/bin/bash
set -euo pipefail
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HERMES_HOME="${PROJECT_DIR}/.hermes_ouroboros"
HERMES_BIN="${OUROBOROS_HERMES_BIN_OVERRIDE:-${HOME}/.hermes/hermes-agent/venv/bin/hermes}"
HERMES_BUNDLED_SKILLS="${HOME}/.hermes/hermes-agent/agent/skills"
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

# Extract SoT Parameters
BACKEND_URL=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu']['base_url'])" 2>/dev/null)
BACKEND_MODEL=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu']['model'])" 2>/dev/null)
BACKEND_CTX=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu']['context_size'])" 2>/dev/null)
BACKEND_API_KEY_ENV=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu'].get('api_key_env', ''))" 2>/dev/null)

if [ -z "${BACKEND_URL:-}" ] || [ -z "${BACKEND_MODEL:-}" ] || [ -z "${BACKEND_CTX:-}" ]; then
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
if [ -z "${SURREALDB_PASS:-}" ]; then
    echo "✗ SURREALDB_PASS is not set"
    exit 1
fi
if [ -z "${CYNIC_REST_ADDR:-}" ]; then
    echo "✗ CYNIC_REST_ADDR is not set"
    exit 1
fi

PATH="${HOME}/bin:${HOME}/.local/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"

if [ ! -x "${HERMES_BIN}" ]; then
    echo "✗ Hermes binary not found at ${HERMES_BIN}"
    exit 1
fi

CYNIC_MCP_BIN=""
if [ -x "${HOME}/bin/cynic-kernel" ]; then
    CYNIC_MCP_BIN="${HOME}/bin/cynic-kernel"
elif command -v cynic-kernel >/dev/null 2>&1; then
    CYNIC_MCP_BIN="$(command -v cynic-kernel)"
else
    echo "✗ Could not find cynic-kernel for Hermes MCP"
    exit 1
fi

mkdir -p "${HERMES_HOME}"
cat > "${HERMES_HOME}/.env" <<EOF
OPENAI_BASE_URL="${BACKEND_URL}"
OPENAI_API_KEY="${BACKEND_API_KEY}"
EOF
cat > "${HERMES_HOME}/config.yaml" <<EOF
mcp_servers:
  cynic:
    command: ${CYNIC_MCP_BIN}
    args:
      - --mcp
    env:
      CYNIC_API_KEY: "${CYNIC_API_KEY}"
      CYNIC_REST_ADDR: "${CYNIC_REST_ADDR}"
      SURREALDB_PASS: "${SURREALDB_PASS}"
      SOVEREIGN_API_KEY: "${SOVEREIGN_API_KEY:-}"
      QWEN35_API_KEY: "${QWEN35_API_KEY:-}"
      HF_TOKEN: "${HF_TOKEN:-}"
      GEMMA_CORE_API_KEY: "${GEMMA_CORE_API_KEY:-}"
EOF

PLAN_JSON="${HERMES_HOME}/ouroboros-plan.json"
REPORT_JSON="${OUROBOROS_REPORT_JSON_OVERRIDE:-${HERMES_HOME}/ouroboros-report.json}"
python3 "${PROJECT_DIR}/scripts/ouroboros_scorecard.py" --date "${RUN_DATE}" > "${PLAN_JSON}"

if [ -f "${PLAN_JSON}" ]; then
    env \
        OUROBOROS_STARTED_AT="${RUN_STARTED_AT}" \
        OUROBOROS_STATUS="running" \
        OUROBOROS_AGENT_ID="hermes" \
        OUROBOROS_MODEL="${BACKEND_MODEL}" \
        OUROBOROS_BACKEND_ID="qwen35-9b-gpu" \
        python3 "${PROJECT_DIR}/scripts/ouroboros_persist.py" --input "${PLAN_JSON}"
fi

# Force Hermes onto the sovereign backend defined in CYNIC's backend config.
unset OPENROUTER_API_KEY
unset LLM_PROVIDER
export OPENAI_BASE_URL="${BACKEND_URL}"
export OPENAI_API_KEY="${BACKEND_API_KEY}"
export HERMES_MODEL="${BACKEND_MODEL}"
export HERMES_CONTEXT_WINDOW="${BACKEND_CTX}"

# Manual Scope (More robust than 'make scope' for this iteration)
SLUG="ouroboros-verify-$(date +%s)"
WORKTREE="${PROJECT_DIR}/../cynic-${SLUG}"
BRANCH="session/gemini/${SLUG}"

git worktree add -b "${BRANCH}" "${WORKTREE}"
# Ensure files are there (force checkout if empty)
cd "${WORKTREE}"
if [ ! -f "Makefile" ]; then
    git checkout HEAD -- .
fi

echo "▶ Ouroboros verified worktree at: ${WORKTREE}"
prompt_file="${PROJECT_DIR}/docs/identity/HERMES-OUROBOROS.md"
runtime_note=$'\n\n## Runtime Notes\n- `CYNIC_API_KEY` is already available in your environment on this machine.\n- First run `printenv CYNIC_API_KEY` in the terminal tool and capture the full exact value.\n- Then call `cynic_auth` with that exact full string as `api_key`.\n- Do not pass the literal text `${CYNIC_API_KEY}`.\n- Do not truncate, preview, mask, or shorten the key before calling `cynic_auth`.\n- Do not ask the user for API keys or other credentials.\n'
runtime_note+=$'\n- If you produce a structured nightly report, write it to `'"${REPORT_JSON}"$'` as JSON with `run` and `repo_results` keys so the launcher can persist it.\n'
mission_note=""
if [ -n "${OUROBOROS_MISSION:-}" ]; then
    mission_note=$'\n\n## Mission\n'"${OUROBOROS_MISSION}"$'\n'
fi

hermes_status="failed"
if env HERMES_HOME="${HERMES_HOME}" HERMES_BUNDLED_SKILLS="${HERMES_BUNDLED_SKILLS}" $HERMES_BIN chat \
    --model "${BACKEND_MODEL}" \
    -t "file,terminal,cynic" \
    -q "$(cat "${prompt_file}")${runtime_note}${mission_note}" \
    --yolo \
    --verbose 2>&1 | tee -a "${LOG_FILE}"; then
    echo "✓ Success"
    hermes_status="completed"
else
    echo "✗ Failed"
fi

RUN_FINISHED_AT="$(date --iso-8601=seconds -u)"
RUN_FINISH_EPOCH="$(date +%s)"
RUN_DURATION_S="$(python3 - <<PY
print(round(${RUN_FINISH_EPOCH} - ${RUN_START_EPOCH}, 3))
PY
)"

FINAL_INPUT="${PLAN_JSON}"
if [ -f "${REPORT_JSON}" ]; then
    FINAL_INPUT="${REPORT_JSON}"
fi

env \
    OUROBOROS_STARTED_AT="${RUN_STARTED_AT}" \
    OUROBOROS_FINISHED_AT="${RUN_FINISHED_AT}" \
    OUROBOROS_DURATION_S="${RUN_DURATION_S}" \
    OUROBOROS_STATUS="${hermes_status}" \
    OUROBOROS_AGENT_ID="hermes" \
    OUROBOROS_MODEL="${BACKEND_MODEL}" \
    OUROBOROS_BACKEND_ID="qwen35-9b-gpu" \
    python3 "${PROJECT_DIR}/scripts/ouroboros_persist.py" --input "${FINAL_INPUT}"

if [ "${hermes_status}" != "completed" ]; then
    exit 1
fi
