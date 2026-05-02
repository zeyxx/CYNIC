# KAIROS Temporal Consciousness — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Embed temporal consciousness (Chronos/Kairos/Aion) into CLAUDE.md and wire the epistemic mempool into the existing kernel observation system, with mechanical measurement via hooks.

**Architecture:** No new infrastructure. CLAUDE.md gains a "Temporal Consciousness" section. session-init.sh injects temporal anchors + mempool RIPE items. Mempool items are kernel observations with `domain=mempool`. session-stop.sh measures temporal compliance. The crystal loop handles accumulation natively.

**Tech Stack:** Bash (hooks), curl (kernel API), jq (JSON parsing), existing kernel `/observe` and `/observations` endpoints.

**Spec:** `docs/superpowers/specs/2026-05-02-kairos-temporal-consciousness-design.md`

---

### Task 1: Add Temporal Consciousness section to CLAUDE.md

**Files:**
- Modify: `CLAUDE.md:69-76` (replace Session Protocol section)

- [ ] **Step 1: Read current Session Protocol section**

Verify lines 70-76 contain the current Chronos-only protocol.

- [ ] **Step 2: Replace Session Protocol with Temporal Consciousness + updated protocol**

Insert after the Anti-Sycophancy section (after line 67), replacing the current Session Protocol:

```markdown
---

## Temporal Consciousness — Chronos / Kairos / Aion

Three simultaneous lenses on every decision. Never separate.

| Time | Question | Signal |
|------|----------|--------|
| **Chronos** (χρόνος) | When is it due? | Deadlines, dependencies, sequence |
| **Kairos** (καιρός) | Is it ripe? | Blockers cleared, context hot, energy aligned |
| **Aion** (αἰών) | What cycle? | Recurring pattern, 3rd instance = treat the structure |

**Actionability = priority × maturity × energy.**

The organism's universe exists only where observed (Wheeler). What is not in the mempool does not exist. What is in the mempool but never consumed expires (K15). Epoché (suspension of judgment) IS kairotic: "not yet" means conditions aren't ripe, not that knowledge is impossible. Wu Wei: disproportionate friction signals the Kairos is not here.

The agent and the human share one temporal field. The session's Kairos is co-created.

**Temporal grounding (injected mechanically by session-init.sh):**
- Current date, time, day of week, days to known deadlines
- Hours since last session, current time vs user's peak hours (19-22h observed)
- Kernel status, Dog availability, crystal velocity = organism's own clock
- Token budget remaining = gas available for this block

**Default:** Before significant action, evaluate maturity, not just correctness. On demand: "what's ripe?", "what cycle recurs?", "what's uniquely possible now?"

---

## Session Protocol

**Start** (temporal read, mechanically injected):
- Temporal anchors + organism vitals + mempool scan (RIPE items)
- Probe live state: `curl /health`, `git status`
- If user arrives with clear intent → skip scan, follow the human

**During:**
- One hypothesis, one experiment. State what would falsify before testing.
- Note gaps/emergences to mempool (`POST /observe domain=mempool`) — noting ≠ acting.
- High-gas work (deep reasoning) early. Low-gas (quick fixes, notes) late.
- When friction appears: Wu Wei signal — Kairos may not be here.

**End:**
- Commit what changed. Update mempool states (MINED → CRYSTALLIZED or deferred).
- Session distill to kernel. session-stop.sh measures temporal compliance.
```

- [ ] **Step 3: Verify CLAUDE.md reads correctly**

Run: `head -120 CLAUDE.md` — verify the new sections render properly and don't break existing sections.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "feat(constitution): temporal consciousness — Chronos/Kairos/Aion

Replaces Chronos-only Session Protocol with three-time temporal
consciousness. Mempool, maturity evaluation, energy sensing.
Epoché as kairotic practice. Wu Wei as friction signal.

Spec: docs/superpowers/specs/2026-05-02-kairos-temporal-consciousness-design.md

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Extend session-init.sh with temporal anchors

**Files:**
- Modify: `.claude/hooks/session-init.sh` (add temporal section before the output block)

- [ ] **Step 1: Add temporal anchor computation**

Insert BEFORE the `# ── Output context` block (before line 180). This computes real-time temporal data:

```bash
# ── Temporal Consciousness: Chronos/Kairos/Aion anchors ──
CURRENT_HOUR=$(date +%H)
CURRENT_DAY=$(date +%A)
CURRENT_DATE=$(date +%Y-%m-%d)

# Peak hours: 19-22h observed (user behavioral data)
PEAK_HOURS="false"
if [[ "$CURRENT_HOUR" -ge 19 && "$CURRENT_HOUR" -le 22 ]]; then
    PEAK_HOURS="true"
fi

# Gap since last session (from session state files)
LAST_SESSION_TS=0
LATEST_STATE=$(ls -t "${SESSION_STATE_DIR}"/*.state 2>/dev/null | head -1)
if [[ -n "$LATEST_STATE" && -f "$LATEST_STATE" ]]; then
    LAST_SESSION_TS=$(grep '^session_start=' "$LATEST_STATE" | cut -d= -f2 || echo 0)
fi
NOW_TS=$(date +%s)
if [[ "$LAST_SESSION_TS" -gt 0 ]]; then
    GAP_HOURS=$(( (NOW_TS - LAST_SESSION_TS) / 3600 ))
else
    GAP_HOURS="unknown"
fi

# Known deadlines (hardcoded for now — will be mempool-derived later)
HACKATHON_DEADLINE="2026-05-10"
DAYS_TO_HACKATHON=$(( ( $(date -d "$HACKATHON_DEADLINE" +%s) - NOW_TS ) / 86400 ))
if [[ "$DAYS_TO_HACKATHON" -lt 0 ]]; then
    DAYS_TO_HACKATHON="past"
fi
```

- [ ] **Step 2: Add mempool scan (RIPE items from kernel)**

Insert after temporal anchors, still before output block:

```bash
# ── Mempool scan: query RIPE items from kernel observations ──
MEMPOOL_RIPE=""
MEMPOOL_COUNT=0
if [[ "$KERNEL_STATUS" != "down" ]]; then
    MEMPOOL_OBS=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/observations?domain=mempool&limit=10" 2>/dev/null)
    if [[ -n "$MEMPOOL_OBS" ]] && echo "$MEMPOOL_OBS" | jq -e 'type == "array" and length > 0' >/dev/null 2>&1; then
        MEMPOOL_COUNT=$(echo "$MEMPOOL_OBS" | jq 'length')
        # Extract items (show context summary for each)
        MEMPOOL_RIPE=$(echo "$MEMPOOL_OBS" | jq -r '.[] | "  → \(.context // "no context" | .[0:120])"' 2>/dev/null | head -5 || true)
    fi
fi
```

- [ ] **Step 3: Add temporal block to output**

Insert in the output `cat <<EOF` block, after the WORKFLOW/COORD/RULES lines:

```bash
# Add to the cat <<EOF output block, after RULES line:

TEMPORAL: ${CURRENT_DATE} ${CURRENT_DAY} ${CURRENT_HOUR}h | Gap: ${GAP_HOURS}h | Peak: ${PEAK_HOURS} | Hackathon: J-${DAYS_TO_HACKATHON}
ORGANISM: Kernel=${KERNEL_STATUS} Dogs=${ACTIVE_DOGS}/${EXPECTED_DOGS} DB=${SURREAL_STATUS}
EOF

# Mempool injection (after EOF)
if [[ "$MEMPOOL_COUNT" -gt 0 ]]; then
    echo ""
    echo "MEMPOOL (${MEMPOOL_COUNT} items, domain=mempool):"
    echo "$MEMPOOL_RIPE"
fi
```

- [ ] **Step 4: Test the hook locally**

```bash
echo '{"cwd":"/home/user/Bureau/CYNIC","session_id":"test-123"}' | bash .claude/hooks/session-init.sh
```

Expected: output includes TEMPORAL line with real date/time/gap and ORGANISM line.

- [ ] **Step 5: Commit**

```bash
git add .claude/hooks/session-init.sh
git commit -m "feat(hooks): temporal anchors + mempool scan in session-init

Injects Chronos anchors (date, time, gap, deadlines), Kairos signals
(peak hours, organism health), and mempool RIPE items at session start.
Mechanical enforcement — LLM doesn't need to remember to scan.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Add temporal compliance measurement to session-stop.sh

**Files:**
- Modify: `.claude/hooks/session-stop.sh` (add temporal metrics section)

- [ ] **Step 1: Read current session-stop.sh structure**

Understand where to add temporal compliance measurement without breaking existing behavior.

- [ ] **Step 2: Add temporal compliance check**

Add before the final output, after existing compliance checks:

```bash
# ── Temporal compliance measurement ──
# Check if mempool was touched during this session (observations domain=mempool created)
TEMPORAL_COMPLIANCE="unknown"
if [[ "$KERNEL_STATUS" != "down" ]]; then
    # Count mempool observations created during this session
    MEMPOOL_ACTIVITY=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/observations?domain=mempool&limit=20" 2>/dev/null \
        | jq "[.[]? | select(.agent_id == \"${AGENT_ID}\")] | length" 2>/dev/null || echo 0)
    
    if [[ "$MEMPOOL_ACTIVITY" -gt 0 ]]; then
        TEMPORAL_COMPLIANCE="active (${MEMPOOL_ACTIVITY} mempool observations)"
    else
        TEMPORAL_COMPLIANCE="inactive (0 mempool observations — temporal consciousness not exercised)"
    fi
fi

echo "TEMPORAL: ${TEMPORAL_COMPLIANCE}"
```

- [ ] **Step 3: POST temporal metrics to kernel**

```bash
# Store temporal compliance as observation for compound loop
if [[ "$KERNEL_STATUS" != "down" ]]; then
    curl -s --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"tool\":\"session_temporal\",\"target\":\"compliance\",\"domain\":\"temporal-meta\",\"context\":\"mempool_activity=${MEMPOOL_ACTIVITY}\",\"agent_id\":\"${AGENT_ID}\",\"tags\":[\"temporal-meta\"]}" \
        >/dev/null 2>&1 || true
fi
```

- [ ] **Step 4: Test session-stop temporal output**

```bash
# Simulate (won't have real session context, but should not error)
AGENT_ID="test-agent" KERNEL_STATUS="down" bash -c 'source .claude/hooks/session-stop.sh' 2>&1 | grep -i temporal || echo "No temporal output (kernel down — expected)"
```

- [ ] **Step 5: Commit**

```bash
git add .claude/hooks/session-stop.sh
git commit -m "feat(hooks): temporal compliance measurement in session-stop

Measures mempool activity per session, reports compliance status,
stores temporal-meta observations for compound loop.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Seed the mempool with first items

**Files:**
- No new files — uses existing kernel `/observe` endpoint

- [ ] **Step 1: Verify kernel is reachable**

```bash
source ~/.cynic-env && curl -s -H "Authorization: Bearer $CYNIC_API_KEY" "http://${CYNIC_REST_ADDR}/health" | jq '.status'
```

Expected: "sovereign" or "degraded". If down, defer this task (Kairos: not ripe).

- [ ] **Step 2: POST first mempool item — hackathon video demo**

```bash
source ~/.cynic-env && curl -s -X POST "http://${CYNIC_REST_ADDR}/observe" \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "mempool_note",
    "target": "hackathon-video-demo",
    "domain": "mempool",
    "context": "{\"type\":\"mempool_item\",\"state\":\"pending\",\"entered\":\"2026-05-02\",\"energy\":\"creative-focus\",\"ttl_days\":8,\"chronos\":{\"deadline\":\"2026-05-10\",\"days_left\":8},\"kairos_conditions\":[{\"condition\":\"rested and clear-headed\",\"met\":false},{\"condition\":\"kernel running for demo\",\"met\":false}],\"aion\":null,\"content\":\"Record video demo (4 scenes: kernel logs, /judge, UI, recovery). Must be done when rested.\"}",
    "agent_id": "claude-kairos-seed",
    "tags": ["mempool", "hackathon"]
  }'
```

- [ ] **Step 3: POST second mempool item — Aion pattern (config debt)**

```bash
source ~/.cynic-env && curl -s -X POST "http://${CYNIC_REST_ADDR}/observe" \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "mempool_note",
    "target": "config-debt-aion",
    "domain": "mempool",
    "context": "{\"type\":\"mempool_item\",\"state\":\"pending\",\"entered\":\"2026-05-02\",\"energy\":\"deep-focus\",\"ttl_days\":30,\"chronos\":{\"deadline\":null,\"recurrence\":3},\"kairos_conditions\":[{\"condition\":\"kernel stable\",\"met\":false},{\"condition\":\"post-hackathon\",\"met\":false}],\"aion\":\"3rd-recurrence-config-scatter\",\"compounds_with\":[\"ssot-config-debt\",\"K11-hardcoding\"],\"content\":\"Config scattered across 5 sources. 3rd time this surfaces. Structural — treat the cycle, not the instance.\"}",
    "agent_id": "claude-kairos-seed",
    "tags": ["mempool", "aion"]
  }'
```

- [ ] **Step 4: POST third mempool item — CCM loop broken (Aion)**

```bash
source ~/.cynic-env && curl -s -X POST "http://${CYNIC_REST_ADDR}/observe" \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "mempool_note",
    "target": "ccm-loop-aion",
    "domain": "mempool",
    "context": "{\"type\":\"mempool_item\",\"state\":\"pending\",\"entered\":\"2026-05-02\",\"energy\":\"deep-focus\",\"ttl_days\":60,\"chronos\":{\"deadline\":null},\"kairos_conditions\":[{\"condition\":\"kernel sovereign + DB healthy\",\"met\":false},{\"condition\":\"verdict volume > 50/day\",\"met\":false}],\"aion\":\"ccm-broken-since-april-25\",\"content\":\"CCM loop_active=false, 23 forming, 0 crystallized. The organism cannot learn. This is the multiplicative base — everything compounds from it.\"}",
    "agent_id": "claude-kairos-seed",
    "tags": ["mempool", "aion", "critical"]
  }'
```

- [ ] **Step 5: Verify mempool items are queryable**

```bash
source ~/.cynic-env && curl -s -H "Authorization: Bearer $CYNIC_API_KEY" \
  "http://${CYNIC_REST_ADDR}/observations?domain=mempool&limit=5" | jq '.[].target'
```

Expected: `"hackathon-video-demo"`, `"config-debt-aion"`, `"ccm-loop-aion"`

- [ ] **Step 6: Commit spec + plan**

```bash
git add docs/superpowers/specs/2026-05-02-kairos-temporal-consciousness-design.md \
        docs/superpowers/plans/2026-05-02-kairos-temporal-consciousness.md
git commit -m "docs: Kairos temporal consciousness spec + implementation plan

Design: three-time consciousness (Chronos/Kairos/Aion), epistemic mempool
as crystal loop extension, Proof of Maturity consensus, mechanical
enforcement via hooks. Blockchain isomorphism. Wheeler's participatory
universe applied to organism.

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Save memory + update TODO.md

**Files:**
- Modify: `TODO.md` (add mempool reference)
- Create: `.claude/projects/-home-user-Bureau-CYNIC/memory/project_kairos_temporal_consciousness.md`

- [ ] **Step 1: Save memory about this design**

Write to memory file with frontmatter.

- [ ] **Step 2: Update MEMORY.md index**

Add entry under Project — Architecture & Design.

- [ ] **Step 3: Add mempool note to TODO.md**

Add a brief section noting the mempool is active and where to find the spec. Do NOT restructure TODO.md — it coexists with the mempool during Phase 1.

- [ ] **Step 4: Commit**

```bash
git add TODO.md .claude/projects/-home-user-Bureau-CYNIC/memory/
git commit -m "docs: Kairos temporal consciousness memory + TODO update

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Verification

After all tasks complete:

1. `head -120 CLAUDE.md` — temporal consciousness section present
2. `echo '{"cwd":"/home/user/Bureau/CYNIC","session_id":"test"}' | bash .claude/hooks/session-init.sh` — TEMPORAL line in output
3. `curl /observations?domain=mempool` — 3 seed items present
4. Next session start: temporal read appears automatically in context injection
