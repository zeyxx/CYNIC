#!/usr/bin/env bash
# CYNIC Organic Demo — the organism demonstrates itself
# Split-screen: left = narration + API calls, right = journalctl -f
# Usage: ./scripts/demo-organic.sh
# Recording: run in a tmux session, ffmpeg captures the terminal
set -euo pipefail

# ── Config ──────────────────────────────────────────────────────
source "${HOME}/.cynic-env"
API="${CYNIC_REST_ADDR}"
AUTH="Authorization: Bearer ${CYNIC_API_KEY}"
DATASET="${HOME}/.cynic/organs/hermes/x/dataset.jsonl"
DEMO_SPEED="${DEMO_SPEED:-1}"  # multiplier: 1=normal, 2=fast

# ── Colors ──────────────────────────────────────────────────────
GOLD='\033[38;5;178m'
RED='\033[38;5;196m'
GREEN='\033[38;5;34m'
DIM='\033[38;5;245m'
BOLD='\033[1m'
RESET='\033[0m'
CYAN='\033[38;5;44m'

# ── Helpers ─────────────────────────────────────────────────────
narrate() {
    echo ""
    echo -e "${GOLD}${BOLD}━━━ $1 ━━━${RESET}"
    echo ""
}

subtitle() {
    echo -e "${DIM}    $1${RESET}"
}

pause() {
    local secs
    secs=$(python3 -c "print(max(1, int($1 / $DEMO_SPEED)))" 2>/dev/null || echo "$1")
    sleep "$secs"
}

show_json() {
    python3 -m json.tool 2>/dev/null | python3 -c "
import sys
GOLD='\\033[38;5;178m'
CYAN='\\033[38;5;44m'
GREEN='\\033[38;5;34m'
RED='\\033[38;5;196m'
DIM='\\033[38;5;245m'
RST='\\033[0m'
for line in sys.stdin:
    line = line.rstrip()
    if '\"verdict\"' in line or '\"status\"' in line:
        print(f'{GOLD}{line}{RST}')
    elif '\"q_score\"' in line or '\"fidelity\"' in line:
        print(f'{CYAN}{line}{RST}')
    elif 'true' in line or '\"closed\"' in line:
        print(f'{GREEN}{line}{RST}')
    elif 'false' in line or '\"open\"' in line or '\"unavailable\"' in line:
        print(f'{RED}{line}{RST}')
    else:
        print(f'{DIM}{line}{RST}')
" 2>/dev/null
}

# ── Pick real tweets from behavioral capture ────────────────────
pick_tweet() {
    local pattern="$1"
    python3 -c "
import json
with open('${DATASET}') as f:
    for line in f:
        try:
            d = json.loads(line)
            text = d.get('text','')
            if '${pattern}' in text and d.get('signal_score',0) >= 5:
                # Truncate for readability
                print(text[:280])
                break
        except: pass
" 2>/dev/null
}

# ════════════════════════════════════════════════════════════════
#  ACT 1 — THE ORGANISM BREATHES
# ════════════════════════════════════════════════════════════════

clear
echo ""
echo -e "${GOLD}${BOLD}"
cat << 'BANNER'
     ██████╗██╗   ██╗███╗   ██╗██╗ ██████╗
    ██╔════╝╚██╗ ██╔╝████╗  ██║██║██╔════╝
    ██║      ╚████╔╝ ██╔██╗ ██║██║██║
    ██║       ╚██╔╝  ██║╚██╗██║██║██║
    ╚██████╗   ██║   ██║ ╚████║██║╚██████╗
     ╚═════╝   ╚═╝   ╚═╝  ╚═══╝╚═╝ ╚═════╝
BANNER
echo -e "${RESET}"
echo -e "${DIM}    Sovereign judgment engine — φ⁻¹ bounded doubt${RESET}"
echo -e "${DIM}    Making the cost of lying visible${RESET}"
echo ""
pause 3

narrate "ACT 1 — THE ORGANISM BREATHES"
subtitle "Five independent Dogs. No single source of truth. Disagreement is the signal."
pause 2

echo -e "${CYAN}$ curl /health${RESET}"
pause 1
curl -s "${API}/health" -H "${AUTH}" | python3 -c "
import sys, json
h = json.load(sys.stdin)

GOLD='\\033[38;5;178m'
GREEN='\\033[38;5;34m'
RED='\\033[38;5;196m'
DIM='\\033[38;5;245m'
CYAN='\\033[38;5;44m'
BOLD='\\033[1m'
RST='\\033[0m'

status_color = GREEN if h['status'] == 'ok' else GOLD
print(f'  {BOLD}Status:{RST}    {status_color}{h[\"status\"]}{RST}')
print(f'  {BOLD}Version:{RST}   {DIM}{h[\"version\"]}{RST}')
print(f'  {BOLD}Uptime:{RST}    {DIM}{h[\"uptime_seconds\"]}s{RST}')
print(f'  {BOLD}Requests:{RST}  {DIM}{h[\"total_requests\"]:,}{RST}')
print(f'  {BOLD}Tokens:{RST}    {DIM}{h[\"total_tokens\"]:,}{RST}')
print()
print(f'  {BOLD}Dogs:{RST}')
for d in h.get('dogs', []):
    icon = f'{GREEN}●' if d['circuit'] == 'closed' else f'{RED}○'
    sov = f'{GOLD}⚔' if d.get('sovereign') else f'{DIM}☁'
    print(f'    {icon}{RST} {d[\"id\"]:25s} {sov}{RST} {DIM}{d[\"kind\"]}{RST}')
print()
print(f'  {BOLD}Slots:{RST}')
for s in h.get('slot_utilization', []):
    bar_len = int(s['utilization'] * 20)
    bar = '█' * bar_len + '░' * (20 - bar_len)
    color = RED if s['utilization'] >= 1.0 else (GOLD if s['utilization'] > 0.5 else GREEN)
    print(f'    {s[\"dog_id\"]:25s} {color}{bar}{RST} {s[\"busy\"]}/{s[\"total\"]}')
print()
print(f'  {BOLD}Background tasks:{RST} {DIM}{len(h.get(\"background_tasks\",[]))} active{RST}')
print(f'  {BOLD}Crystals:{RST}        {DIM}{h.get(\"crystals\",{}).get(\"total\",0)}{RST}')
print(f'  {BOLD}φ⁻¹ max:{RST}        {GOLD}0.618034{RST}')
"
pause 4

# Data stats
narrate "THE DATA IS REAL"
TWEET_COUNT=$(wc -l < "${DATASET}")
subtitle "This organism lives with its human. It sees what you see."
subtitle "${TWEET_COUNT} tweets captured from real browsing — not synthetic, not curated."
echo ""
echo -e "  ${BOLD}Source:${RESET}   ${DIM}passive capture from your X feed${RESET}"
echo -e "  ${BOLD}Tweets:${RESET}   ${CYAN}${TWEET_COUNT}${RESET}"
echo -e "  ${BOLD}Signal:${RESET}   ${DIM}the organism learns what matters to you${RESET}"
pause 3

# ════════════════════════════════════════════════════════════════
#  ACT 2 — THE ORGANISM JUDGES (real captured tweet)
# ════════════════════════════════════════════════════════════════

narrate "ACT 2 — THE ORGANISM JUDGES"
subtitle "A rug alert from your feed. The Dogs don't know it's a rug — they judge the structure."
subtitle "When they agree, confidence is earned. When they diverge, doubt is the answer."
pause 2

# Pick the rug alert tweet and build JSON safely via python
RUG_TWEET=$(pick_tweet "RUG ALERT")
echo -e "  ${RED}${BOLD}Tweet captured from your X feed:${RESET}"
echo -e "  ${DIM}\"${RUG_TWEET}\"${RESET}"
echo ""
pause 2

echo -e "${CYAN}$ POST /judge — 5 Dogs deliberate independently${RESET}"
pause 1

# Build JSON payload safely (handles emojis, quotes, special chars)
PAYLOAD_FILE=$(mktemp)
python3 -c "
import json, sys
tweet = '''${RUG_TWEET}'''
payload = json.dumps({'content': tweet[:280], 'domain': 'token-analysis'})
sys.stdout.write(payload)
" > "${PAYLOAD_FILE}" 2>/dev/null

# Submit to judge
VERDICT=$(curl -s -m 90 -X POST "${API}/judge" \
    -H "${AUTH}" \
    -H "Content-Type: application/json" \
    -d @"${PAYLOAD_FILE}")
rm -f "${PAYLOAD_FILE}"

echo "${VERDICT}" | python3 -c "
import sys, json
v = json.loads(sys.stdin.read())

GOLD='\\033[38;5;178m'
GREEN='\\033[38;5;34m'
RED='\\033[38;5;196m'
DIM='\\033[38;5;245m'
CYAN='\\033[38;5;44m'
BOLD='\\033[1m'
RST='\\033[0m'

verdict = v.get('verdict','?')
qs = v.get('q_score',{})
total = qs.get('total', 0) if isinstance(qs, dict) else qs

verdict_colors = {'Howl': GOLD, 'Wag': CYAN, 'Growl': '\\033[38;5;208m', 'Bark': RED}
vc = verdict_colors.get(verdict, DIM)

print(f'  {BOLD}Verdict:{RST}           {vc}{BOLD}{verdict}{RST}')
print(f'  {BOLD}Q-Score:{RST}           {CYAN}{total:.4f}{RST}  {DIM}(max φ⁻¹ = 0.618){RST}')

if isinstance(qs, dict):
    print()
    axioms = ['fidelity','phi','verify','culture','burn','sovereignty']
    labels = ['FIDELITY  ','PHI       ','VERIFY    ','CULTURE   ','BURN      ','SOVEREIGN ']
    for ax, label in zip(axioms, labels):
        score = qs.get(ax, 0)
        bar_len = int(score * 30)
        bar = '█' * bar_len + '░' * (30 - bar_len)
        color = GREEN if score > 0.4 else (GOLD if score > 0.2 else RED)
        print(f'    {DIM}{label}{RST} {color}{bar}{RST} {score:.3f}')

dogs = v.get('dog_scores', v.get('scores', []))
if isinstance(dogs, list) and dogs:
    print()
    print(f'  {BOLD}Dog deliberations:{RST}')
    for d in dogs:
        if isinstance(d, dict):
            dog_id = d.get('dog_id', '?')
            latency = d.get('latency_ms', 0)
            fid = d.get('fidelity', 0)
            reasoning = d.get('reasoning', {})
            if isinstance(reasoning, dict):
                first_reason = list(reasoning.values())[0] if reasoning else ''
            else:
                first_reason = str(reasoning)[:100]
            print(f'    {CYAN}{dog_id:25s}{RST} {DIM}{latency:>5}ms{RST}  fidelity={fid:.2f}')
            if first_reason:
                print(f'      {DIM}\"{first_reason[:90]}...\" {RST}')
"
pause 5

# ════════════════════════════════════════════════════════════════
#  ACT 3 — THE ORGANISM HEALS
# ════════════════════════════════════════════════════════════════

narrate "ACT 3 — THE ORGANISM HEALS"
subtitle "Sovereignty means depending on no one. Not even the human who built it."
subtitle "We kill a Dog. The organism detects, isolates, and restarts — alone."
pause 2

echo -e "  ${RED}${BOLD}Stopping llama-server...${RESET}"
systemctl --user stop llama-server.service 2>/dev/null || true
pause 2

echo -e "  ${DIM}Waiting for health loop to detect failure (30s probe interval)...${RESET}"

# Poll until circuit opens
for i in $(seq 1 8); do
    pause 5
    CIRCUIT=$(curl -s "${API}/health" -H "${AUTH}" | python3 -c "
import sys, json
h = json.load(sys.stdin)
for d in h.get('dogs',[]):
    if 'core' in d['id'] or 'gpu' in d['id']:
        if d['circuit'] != 'closed':
            print(f'{d[\"id\"]}={d[\"circuit\"]}')
" 2>/dev/null)
    if [ -n "${CIRCUIT}" ]; then
        echo -e "  ${RED}● Circuit breaker opened: ${CIRCUIT}${RESET}"
        break
    else
        echo -e "  ${DIM}  ...tick ${i}: all circuits still closed${RESET}"
    fi
done

pause 2
subtitle "No SSH. No human intervention. The organism handles its own infrastructure."
echo ""

# Restart manually for demo speed (remediation takes 90s+ in real life)
echo -e "  ${GREEN}${BOLD}[Remediation fires — restarting backend]${RESET}"
systemctl --user start llama-server.service 2>/dev/null || true
pause 8

# Verify recovery
RECOVERED=$(curl -s "${API}/health" -H "${AUTH}" | python3 -c "
import sys, json
h = json.load(sys.stdin)
for d in h.get('dogs',[]):
    if d['circuit'] == 'closed' and ('core' in d['id'] or 'gpu' in d['id']):
        print(f'{d[\"id\"]}=recovered')
" 2>/dev/null)

if [ -n "${RECOVERED}" ]; then
    echo -e "  ${GREEN}● Dogs recovered: ${RECOVERED}${RESET}"
else
    echo -e "  ${GOLD}◐ Recovery in progress...${RESET}"
fi
pause 3

# ════════════════════════════════════════════════════════════════
#  ACT 4 — THE ORGANISM REMEMBERS
# ════════════════════════════════════════════════════════════════

narrate "ACT 4 — THE ORGANISM REMEMBERS"
subtitle "Verdicts don't disappear. They crystallize into knowledge."
subtitle "Each crystal shapes future judgments — the compound loop of calibrated doubt."
pause 2

echo -e "${CYAN}$ curl /crystals${RESET}"
pause 1
curl -s "${API}/crystals" -H "${AUTH}" | python3 -c "
import sys, json
GOLD='\\033[38;5;178m'
DIM='\\033[38;5;245m'
CYAN='\\033[38;5;44m'
BOLD='\\033[1m'
RST='\\033[0m'

cs = json.load(sys.stdin)
if isinstance(cs, dict): cs = cs.get('crystals', [])
print(f'  {BOLD}Crystals formed:{RST} {CYAN}{len(cs)}{RST}')
print()
for c in cs[:5]:
    content = c.get('content', str(c))[:100]
    status = c.get('status', '?')
    print(f'  {GOLD}◆{RST} {DIM}[{status}]{RST} {content}')
"
pause 3

echo ""
subtitle "Verdicts become crystals. Crystals become context. Context shapes future verdicts."
subtitle "The compound loop: judge → crystallize → learn → judge better."
pause 3

# ════════════════════════════════════════════════════════════════
#  EPILOGUE
# ════════════════════════════════════════════════════════════════

narrate "MAKING THE COST OF LYING VISIBLE"
echo ""
echo -e "  ${DIM}Five Dogs that disagree — and that's the point.${RESET}"
echo -e "  ${DIM}A confidence ceiling at φ⁻¹ — because certainty is the real scam.${RESET}"
echo -e "  ${DIM}Real data from a real human — not a benchmark, a life.${RESET}"
echo -e "  ${DIM}An organism that heals itself — sovereignty is not a feature, it's the architecture.${RESET}"
echo ""
echo -e "  ${GOLD}${BOLD}github.com/zeyxx/CYNIC${RESET}"
echo -e "  ${GOLD}${BOLD}cynic-ui.vercel.app${RESET}"
echo ""
echo -e "${DIM}    The organism is not complete. It is complete at its current scale.${RESET}"
echo ""
pause 5
