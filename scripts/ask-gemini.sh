#!/usr/bin/env bash
# CYNIC — Ask Gemini CLI for axiom-grounded analysis.
# Usage: scripts/ask-gemini.sh "Your question here"
#        scripts/ask-gemini.sh --axiom "Content to evaluate"
#        echo "content" | scripts/ask-gemini.sh --stdin
#
# Modes:
#   (default)  Pass prompt directly to Gemini
#   --axiom    Wrap in CYNIC axiom framing (6 axes evaluation)
#   --stdin    Read prompt from stdin (for piping)
set -euo pipefail

TIMEOUT="${GEMINI_TIMEOUT:-60}"
GEMINI="${GEMINI_BIN:-gemini}"

# Axiom frame — injected when --axiom is used
AXIOM_FRAME='Tu es un Dog CYNIC. Analyse le contenu suivant à travers les 6 axiomes (FIDELITY, PHI, VERIFY, CULTURE, BURN, SOVEREIGNTY). Pour chaque axiome, une phrase. Termine par un verdict: HOWL (>0.528), WAG (>0.382), GROWL (>0.236), ou BARK (≤0.236). Max confidence 0.618.

Contenu:'

mode="direct"
prompt=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --axiom) mode="axiom"; shift ;;
        --stdin) mode="stdin"; shift ;;
        *) prompt="$*"; break ;;
    esac
done

case "$mode" in
    axiom)
        if [[ -z "$prompt" ]]; then
            prompt=$(cat)
        fi
        full_prompt="${AXIOM_FRAME}
${prompt}"
        ;;
    stdin)
        full_prompt=$(cat)
        ;;
    direct)
        full_prompt="$prompt"
        ;;
esac

if [[ -z "$full_prompt" ]]; then
    echo "Usage: ask-gemini.sh [--axiom|--stdin] \"prompt\"" >&2
    exit 1
fi

timeout "$TIMEOUT" "$GEMINI" --prompt "$full_prompt" 2>/dev/null
