---
name: cc-status
description: Claude Code harness diagnostic — hooks, observers, telemetry, wisdom.
disable-model-invocation: true
allowed-tools: Bash(curl *) Bash(source *) Bash(jq *) Bash(wc *) Bash(ls *) Bash(test *) Bash(cat *) Bash(grep *)
---

Claude Code harness diagnostic — what fires, at what rate, and what's missing.

First: `source ~/.cynic-env 2>/dev/null`

Run ALL of these and report as a dashboard:

1. **Hook inventory:**
   ```
   jq -r '.hooks | to_entries[] | .key as $event |
     .value[] | .hooks[]? |
     "\($event)|\(.command | split("/") | last | gsub("\"$";""))|\(.async // false)|\(.timeout // "-")"' \
     "$CLAUDE_PROJECT_DIR"/.claude/settings.json 2>/dev/null | \
     while IFS='|' read -r event script async timeout; do
       printf "  %-22s %-35s async=%-5s timeout=%s\n" "$event" "$script" "$async" "$timeout"
     done
   ```
   Count total hooks and async hooks.

2. **Observer pipeline check:**
   ```
   OBS_TOOL="no"; OBS_PROMPT="no"; OBS_AGENT="no"
   [ -x "$CLAUDE_PROJECT_DIR"/.cortex/mcp/observe-tool.sh ] && OBS_TOOL="yes"
   [ -x "$CLAUDE_PROJECT_DIR"/.cortex/mcp/observe-prompt.sh ] && OBS_PROMPT="yes"
   [ -x "$CLAUDE_PROJECT_DIR"/.cortex/mcp/observe-subagent.sh ] && OBS_AGENT="yes"
   echo "  tool=$OBS_TOOL  prompt=$OBS_PROMPT  subagent=$OBS_AGENT"
   # Check if observe-tool.sh has domain/tags (v2 indicator)
   grep -q 'DOMAIN=' "$CLAUDE_PROJECT_DIR"/.cortex/mcp/observe-tool.sh 2>/dev/null && echo "  observe-tool: v2 (domain+tags)" || echo "  observe-tool: v1 (basic)"
   ```

3. **Telemetry rate (if kernel reachable):**
   ```
   curl -s --max-time 3 "http://${CYNIC_REST_ADDR}/observations?limit=100" \
     -H "Authorization: Bearer ${CYNIC_API_KEY}" 2>/dev/null | \
     jq -r 'group_by(.tool) | .[] | "\(.[0].tool)=\(length)"' 2>/dev/null | \
     tr '\n' '  '
   ```
   Show total count and top tool by frequency.

4. **Distill completion rate:**
   ```
   DISTILLS=$(curl -s --max-time 3 "http://${CYNIC_REST_ADDR}/observations?domain=session&limit=50" \
     -H "Authorization: Bearer ${CYNIC_API_KEY}" 2>/dev/null)
   DISTILL_N=$(echo "$DISTILLS" | jq '[.[] | select(.tool == "session_distill")] | length' 2>/dev/null || echo "0")
   SESSION_N=$(echo "$DISTILLS" | jq '[.[] | select(.tool == "session_summary")] | length' 2>/dev/null || echo "0")
   if [ "$SESSION_N" -gt 0 ] 2>/dev/null; then
     RATE=$(awk "BEGIN {printf \"%.0f\", ($DISTILL_N/$SESSION_N)*100}")
     echo "  ${DISTILL_N}/${SESSION_N} sessions  ${RATE}%"
     [ "$RATE" -lt 62 ] && echo "  WARN: distill rate < 61.8% (φ⁻¹ threshold)"
   else
     echo "  no session data"
   fi
   ```

5. **Crystal freshness:**
   ```
   curl -s --max-time 3 "http://${CYNIC_REST_ADDR}/crystals" \
     -H "Authorization: Bearer ${CYNIC_API_KEY}" 2>/dev/null | \
     jq -r '{total: length,
       canonical: [.[] | select(.state=="canonical")] | length,
       crystallized: [.[] | select(.state=="crystallized")] | length,
       forming: [.[] | select(.state=="forming")] | length}' 2>/dev/null
   ```

6. **Wisdom signals:**
   ```
   for f in "$CLAUDE_PROJECT_DIR"/cynic-python/curation/D*_curated.jsonl; do
     [ -f "$f" ] || continue
     DOMAIN=$(basename "$f" | sed 's/_curated.jsonl//')
     N=$(wc -l < "$f" 2>/dev/null || echo 0)
     STRONG=$(jq -r 'select(.strength >= 0.8)' "$f" 2>/dev/null | wc -l || echo 0)
     printf "  %s: %d signals (%d strong)\n" "$DOMAIN" "$N" "$STRONG"
   done
   ```

7. **Permissions:**
   ```
   ALLOW=$(jq '.permissions.allow | length' "$CLAUDE_PROJECT_DIR"/.claude/settings.local.json 2>/dev/null || echo "?")
   DENY=$(jq '.permissions.deny | length' "$CLAUDE_PROJECT_DIR"/.claude/settings.local.json 2>/dev/null || echo "?")
   echo "  ${ALLOW} allow  ${DENY} deny"
   ```

Format as:
```
=== CC-STATUS {timestamp} ===
HOOKS:       {N} wired  {async_count} async
OBSERVERS:   tool={yes|no}  prompt={yes|no}  subagent={yes|no}
TELEMETRY:   {tool}={count} ...
DISTILL:     {distill_count}/{session_count} sessions  {rate}%
CRYSTALS:    {canonical}/{crystallized}/{forming}/{total}
WISDOM:      D1({N}) D2({N}) ... = {total} signals
PERMISSIONS: {allow_N} allow  {deny_N} deny

--- HOOK INVENTORY ---
{event:22}: {script:35} async={bool} timeout={ms}

--- GAPS ---
[auto-detect from above: missing observers, low distill rate, zero crystals, etc.]
```

WARN if distill rate < 61.8%. WARN if any observer is missing. WARN if zero crystals.
