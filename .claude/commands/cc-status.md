---
name: cc-status
description: Claude Code integration health — what CYNIC owns and whether the harness is functioning.
disable-model-invocation: true
allowed-tools: Bash(source *) Bash(curl *) Bash(jq *) Bash(ls *) Bash(wc *) Bash(stat *) Bash(find *) Bash(date *) Bash(grep *) Bash(awk *)
---

Claude Code integration health audit — not a green facade, a real picture.

First: `source ~/.cynic-env 2>/dev/null && PROJECT_DIR="/home/user/Bureau/CYNIC"`

Run ALL of these and report EVERYTHING:

1. **Hook wiring (what's configured in settings.json):**
   ```bash
   jq '.hooks | to_entries | map({event: .key, hooks: [.value[].hooks[].command], async: [.value[].hooks[] | select(.async) | .command]}) | .[] | "\(.event): \(.hooks | join(", ")) [async: \(.async | join(", "))]"' "${PROJECT_DIR}/.claude/settings.json"
   ```
   Count total hooks (should be ~11), note which are async (observe-tool.sh, exercise-scheduler.sh are async).

2. **Hook script health (exist and executable):**
   ```bash
   ls -lah "${PROJECT_DIR}/.claude/hooks/" | awk 'NR>1 {print $9, $5, $6, $7, $8}'
   ```
   All scripts should be executable (mode includes 'x'). Show file size, modified date. Note if any script is >20KB (suspicious).

3. **Session telemetry rate (last 24 hours):**
   ```bash
   source ~/.cynic-env 2>/dev/null
   curl -s --connect-timeout 2 --max-time 5 "${CYNIC_REST_ADDR}/observations?limit=100" \
     -H "Authorization: Bearer ${CYNIC_API_KEY}" 2>/dev/null | \
     jq '[.[] | select((.created_at // "1970-01-01") > ((now - 86400) | gmtime | strftime("%Y-%m-%dT%H:%M:%SZ")))] | 
           group_by(.tool) | map({tool: .[0].tool, count: length, latest: .[0].created_at}) | 
           sort_by(-.count) | .[] | "\(.tool): \(.count) (latest: \(.latest))"' 2>/dev/null || echo "Kernel unreachable"
   ```
   Extract: top 5 tools by frequency (last 24h). If observe-tool.sh is firing, top tools should be Edit, Bash, Read in some order. If missing, hooks are not firing.

4. **Distill completion rate (session handover health):**
   ```bash
   source ~/.cynic-env 2>/dev/null
   curl -s --connect-timeout 2 --max-time 5 "${CYNIC_REST_ADDR}/observations?domain=session&limit=50" \
     -H "Authorization: Bearer ${CYNIC_API_KEY}" 2>/dev/null | \
     jq '{total_sessions: [.[] | select(.tool == "session_summary")] | length,
           distill_count: [.[] | select(.tool == "session_distill")] | length,
           distill_rate: ([.[] | select(.tool == "session_distill")] | length / ([.[] | select(.tool == "session_summary")] | length | if . == 0 then 1 else . end) * 100 | floor)}' 2>/dev/null || echo "Kernel unreachable"
   ```
   Extract: distill_count / total_sessions ratio. Target: ≥61.8% (φ⁻¹). If <50%, warn: "MISSING DISTILLS: organism is learning from <50% of sessions".

5. **Crystal freshness (CCM output):**
   ```bash
   source ~/.cynic-env 2>/dev/null
   curl -s --connect-timeout 2 --max-time 5 "${CYNIC_REST_ADDR}/crystals?limit=10" \
     -H "Authorization: Bearer ${CYNIC_API_KEY}" 2>/dev/null | \
     jq '[.[] | {state: .state, confidence: .confidence, signal_count: (.signals | length), age_hours: (((now - (.created_at // "1970-01-01T00:00:00Z" | fromdateiso8601)) / 3600) | floor)}] | 
           group_by(.state) | map({state: .[0].state, count: length, oldest_hours: (map(.age_hours) | max)})' 2>/dev/null || echo "Kernel unreachable"
   ```
   Extract: counts per state (forming, crystallized, canonical). If all 0, CCM is dormant. Show oldest crystal age (in hours).

6. **Domain wisdom signals (curation freshness):**
   ```bash
   for f in "${PROJECT_DIR}/cynic-python/curation"/D*_curated.jsonl; do
     domain=$(basename "$f" | cut -d_ -f1)
     total=$(wc -l < "$f" 2>/dev/null || echo 0)
     high_strength=$(jq -r 'select(.strength >= 0.8)' "$f" 2>/dev/null | wc -l)
     echo "$domain: $total signals ($high_strength ≥0.8)"
   done
   ```
   Extract: signal count per domain, high-strength ratio. Wisdom should be injected at session-init from these files.

7. **Permissions summary (settings.local.json):**
   ```bash
   jq '{allow_count: (.permissions.allow | length), deny_count: (.permissions.deny | length), allow_sample: .permissions.allow[:3], deny_sample: .permissions.deny[:3]}' "${PROJECT_DIR}/.claude/settings.local.json" 2>/dev/null || echo "No settings.local.json"
   ```
   Extract: total allow/deny rules. Show sample of each (first 3). Healthy: ~100 allow, ~15 deny.

8. **Slash commands inventory:**
   ```bash
   ls -1 "${PROJECT_DIR}/.claude/commands/"*.md | wc -l && \
   ls -1 "${PROJECT_DIR}/.claude/commands/"*.md | xargs -I {} basename {} .md | tr '\n' ' '
   ```
   Extract: count + list of all commands available (/build, /status, /cc-status, etc.).

9. **Rules files inventory:**
   ```bash
   ls -lh "${PROJECT_DIR}/.claude/rules/"*.md | awk '{print $9, $5}' | xargs -I {} basename {} | sort
   ```
   Extract: all rule files present (universal, workflow, kernel, python, reference, cost).

10. **Last session proof (multi-cortex state):**
    ```bash
    jq '{session_id: .session_id, at_start_branch: .AT_START.branch_name, at_start_commits_ahead: .AT_START.commits_ahead, at_end_compliance: .AT_END.compliance_score, at_end_distill_status: (if .AT_END.distill_posted then "✓ posted" else "✗ MISSING" end), at_end_work_lost: .AT_END.work_lost}' "${PROJECT_DIR}/.claude/session-proof.json" 2>/dev/null || echo "(no session proof)"
    ```
    Extract: last session's compliance score, distill status, any work lost warnings.

---

## Output Format

```
=== CC-STATUS {timestamp} ===

HOOKS:           {N} wired  {async_count} async
HOOK SCRIPTS:    {N}/11 found {oldest_mod_date}
TELEMETRY:       {obs_24h} tool-obs/24h  top: {tool}×{count} {tool}×{count}...
DISTILL RATE:    {distill_count}/{session_count} = {rate}%  [WARN if <61.8%]
CRYSTALS:        {forming}/{crystallized}/{canonical} = {total}  oldest {hours}h
WISDOM:          D1({N}+{high_N}) D2(...) = {total_signals} {high_total} ≥0.8
PERMISSIONS:     {allow} allow  {deny} deny
COMMANDS:        {N} available

--- Hook Script Details ---
{script}: {size}  {mod_date}
...

--- Wisdom by Domain ---
D1: {total} signals ({high_strength} ≥0.8)
D2: {total} signals ({high_strength} ≥0.8)
...

--- GAPS DETECTED ---
[auto-generated from above]
- Hook X last executed: {time_ago}
- Distill rate {rate}% < 61.8%: {count} sessions invisible to organism
- CCM dormant: 0 crystallized — no feedback loop
- ...
```

---

## Interpretation

**Healthy:**
- HOOKS: 11 wired, 2 async
- TELEMETRY: >50 tool-obs/24h (means hooks are firing)
- DISTILL: ≥61.8% (at least most sessions learning)
- CRYSTALS: >0 crystallized (CCM working)
- WISDOM: >200 total signals loaded

**Degraded:**
- Hooks < 11 or async < 2 (hooks wired incorrectly)
- Telemetry < 10/24h (hooks not firing)
- Distill < 50% (sessions skipping reasoning capture)
- Crystals all "forming" (CCM promoting slowly)

**Broken:**
- Hooks 0 wired (settings.json corrupted)
- Kernel unreachable (all /observations calls fail)
- WISDOM files missing (curation deleted)
- Permissions deny all (locked out)
