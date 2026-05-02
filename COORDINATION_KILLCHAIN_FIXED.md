# Multi-Cortex Coordination Kill Chain — Fixed

## The Problem

**Observed state:** 60+ unmerged commits across 13 local branches + 25 remote branches (5-day accumulation).

**Root cause:** Coordination system built but not wired into session lifecycle.

## Kill Chain Breakdown

### 1. SESSION START (session-init.sh)
- Runs on every Claude Code session start
- **Extracts:** SESSION_ID from Claude context (line 144)
- **Derives:** AGENT_ID = "claude-${SESSION_ID:0:12}" (line 146)
- **Registers:** POST /coord/register with AGENT_ID (line 164-168) ✓
- **Persists:** Writes to /tmp/cynic-sessions/${AGENT_ID}.state (line 154-159) ✓
- **Problem:** AGENT_ID not exported downstream; SESSION_ID not reliably available to PreToolUse hooks

### 2. EDIT/WRITE (cynic-kernel/src/**)
- PreToolUse hook fires for Edit/Write on kernel files
- **Calls:** coord-claim.sh (line 67-74 in settings.json)
- **Problem (BEFORE FIX):** 
  - coord-claim.sh tries to extract SESSION_ID from INPUT (line 35)
  - If SESSION_ID missing → silent block (line 41-43)
  - Result: ALL kernel edits blocked or silently skipped
  - Cortexes work around by editing outside cynic-kernel/src/** (explains the branches)

### 3. RELEASE (session-stop.sh)
- Runs when Claude Code session ends
- **Goal:** Release all coordination claims via POST /coord/release
- **Problem (BEFORE FIX):**
  - session-stop.sh tried to get AGENT_ID from SESSION_ID in INPUT (line 14)
  - If SESSION_ID missing → silent exit (line 20), claims not released
  - Result: Claims leaked, TTL expiry (5min) was only cleanup mechanism

## What Was Fixed

### Fix 1: coord-claim.sh (PreToolUse hook)

**Before:**
```bash
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
if [[ -z "$SESSION_ID" ]]; then
    block "coordination requires session_id for kernel edits"
fi
AGENT_ID="claude-${SESSION_ID:0:12}"
```

**After:**
```bash
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
AGENT_ID=""
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    # Fallback: read from most recent session state file (set by session-init.sh)
    SESSION_STATE_DIR="/tmp/cynic-sessions"
    if [[ -d "$SESSION_STATE_DIR" ]]; then
        RECENT_STATE=$(ls -t "$SESSION_STATE_DIR"/*.state 2>/dev/null | head -1)
        if [[ -n "$RECENT_STATE" ]]; then
            AGENT_ID=$(grep -oP 'agent_id=\K[^ ]+' "$RECENT_STATE" 2>/dev/null || true)
        fi
    fi
fi
if [[ -z "$AGENT_ID" ]]; then
    block "coordination requires valid AGENT_ID (SESSION_ID missing and no session state file found)"
fi
```

**Improvement:**
- Primary path: Use SESSION_ID from INPUT (if Claude Code provides it)
- Fallback path: Discover AGENT_ID from /tmp/cynic-sessions/
- Explicit failure: Clear error message if AGENT_ID can't be determined

### Fix 2: session-stop.sh (Stop hook)

**Before:**
```bash
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    exit 0  # Silent exit, claims not released
fi
```

**After:**
```bash
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || true)
AGENT_ID=""
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    # Fallback: read from most recent session state file (set by session-init.sh)
    SESSION_STATE_DIR="/tmp/cynic-sessions"
    if [[ -d "$SESSION_STATE_DIR" ]]; then
        RECENT_STATE=$(ls -t "$SESSION_STATE_DIR"/*.state 2>/dev/null | head -1)
        if [[ -n "$RECENT_STATE" ]]; then
            AGENT_ID=$(grep -oP 'agent_id=\K[^ ]+' "$RECENT_STATE" 2>/dev/null || true)
        fi
    fi
fi
if [[ -z "$AGENT_ID" ]]; then
    echo "Warning: could not determine AGENT_ID for coordination release" >&2
    exit 0  # Explicit warning, let TTL cleanup handle it
fi
```

**Improvement:**
- Primary path: Use SESSION_ID from INPUT
- Fallback path: Discover AGENT_ID from session state file
- Explicit logging: Warn when AGENT_ID can't be determined (instead of silent exit)

## Expected Observable Change

**Before fix:**
- Cortex A edits cynic-kernel/src/foo.rs → hook blocks → cortex switches to cynic-python/bar.py
- Result: branches accumulate in cynic-python/, cynic-kernel/src/ stays pristine
- Multi-cortex coordination never enforced

**After fix:**
- Cortex A registers at session start → AGENT_ID written to /tmp/cynic-sessions/
- Cortex A edits cynic-kernel/src/foo.rs → hook reads AGENT_ID from state file → claims file → success
- Cortex B tries same file → hook claims → kernel returns 409 CONFLICT → cortex waits or escalates
- Result: mutual exclusion enforced, branches converge, coordination debt visible

## Falsification Test

**Hypothesis:** Coordination system now works end-to-end.

**Test:** 
1. Session starts → session-init.sh creates /tmp/cynic-sessions/claude-*.state
2. Edit cynic-kernel/src/pipeline/mod.rs → coord-claim.sh reads state file, gets AGENT_ID
3. curl shows agent has claim on cynic-kernel/src/pipeline/mod.rs
4. Session ends → session-stop.sh finds state file, releases claim
5. curl shows claim is gone

**Falsify:** If any step fails (claim not visible, release doesn't work), coordination still broken.

## References

- `.claude/hooks/session-init.sh` — writes session state (AGENT_ID persistent)
- `.claude/hooks/coord-claim.sh` — now reads state file as fallback
- `.claude/hooks/session-stop.sh` — now reads state file as fallback
- API.md → /coord/register, /coord/claim, /coord/release, /coord/who
- AGENTS.md → MC1-MC5 coordination rules
- settings.json → PreToolUse hook wired to coord-claim.sh on Edit/Write
