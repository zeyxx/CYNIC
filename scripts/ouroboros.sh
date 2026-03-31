#!/bin/bash
set -e
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
HERMES_HOME="${PROJECT_DIR}/.hermes_ouroboros"
HERMES_BIN="${HOME}/.hermes/hermes-agent/venv/bin/hermes"
HERMES_BUNDLED_SKILLS="${HOME}/.hermes/hermes-agent/agent/skills"
CONFIG_FILE="${HOME}/.config/cynic/backends.toml"

# Extract SoT Parameters
BACKEND_URL=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu']['base_url'])" 2>/dev/null)
BACKEND_MODEL=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu']['model'])" 2>/dev/null)
BACKEND_CTX=$(python3 -c "import tomllib, os; print(tomllib.load(open(os.path.expanduser('${CONFIG_FILE}'), 'rb'))['backend']['qwen35-9b-gpu']['context_size'])" 2>/dev/null)

# Force Auth Fallback
unset OPENROUTER_API_KEY
unset LLM_PROVIDER
export OPENAI_BASE_URL="${BACKEND_URL}"
export OPENAI_API_KEY="cynic-local"
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

if env HERMES_HOME="${HERMES_HOME}" HERMES_BUNDLED_SKILLS="${HERMES_BUNDLED_SKILLS}" $HERMES_BIN chat \
    --model "${BACKEND_MODEL}" \
    -t "file,terminal,cynic,github" \
    -q "$(cat "${prompt_file}")" \
    --yolo \
    --verbose 2>&1 | tee -a "${PROJECT_DIR}/cynic-ouroboros.log"; then
    echo "✓ Success"
else
    echo "✗ Failed"
    exit 1
fi
