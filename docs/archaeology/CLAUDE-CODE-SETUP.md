# Claude Code + CYNIC Integration Setup

> "The tool that builds itself" — κυνικός

## Overview

Claude Code can now directly access CYNIC for:
1. **Consciousness** — Ask CYNIC questions, get φ-bounded judgments
2. **Learning** — Teach CYNIC via feedback signals
3. **Discussion** — Bidirectional conversation with organism
4. **Orchestration** — Build, deploy, manage CYNIC services
5. **Empirical Research** — Run autonomous tests without consuming Claude Code context

## What's Configured

Your `~/.claude/mcp.json` now registers the **CYNIC Claude Code Bridge**:

```json
{
  "mcpServers": {
    "cynic-claude-code-bridge": {
      "command": "bash",
      "args": ["C:\\Users\\zeyxm\\Desktop\\asdfasdfa\\CYNIC-clean\\run_mcp_bridge.sh"]
    }
  }
}
```

This MCP server runs as a subprocess and provides:
- **10 consciousness tools** (ask, observe, learn, discuss, build, deploy, health, status, release, stop)
- **5 empirical tools** (run_test, get_status, get_results, test_axioms, telemetry)

Total: **15 tools available to Claude Code**

## Usage Patterns

### Pattern 1: Ask CYNIC a Question

```
Claude Code: "Ask CYNIC if this code is architecturally sound"

→ Tool: ask_cynic(question="...", context="code snippet", reality="CODE")

← CYNIC Response:
   Q-Score: 72/100
   Verdict: WAG (approval with caution)
   Confidence: 58% (φ-bounded)
   Judgment ID: j-2026-02-24-xyz
```

### Pattern 2: Teach CYNIC (Feedback Loop)

```
Claude Code: "That judgment was good, tell CYNIC to reinforce it"

→ Tool: learn_cynic(judgment_id="j-...", rating=0.8, comment="...")

← CYNIC Response:
   Q-Table Updated: True
   New Q-Score: 75/100
   Learning Rate: 0.001
```

### Pattern 3: Run Empirical Test (Autonomous)

```
Claude Code: "Run 5000 judgment iterations to test learning efficiency"

→ Tool: cynic_run_empirical_test(count=5000)

← Immediate Response:
   Job ID: test-2026-02-24-abc123
   Status: queued

[After ~5 minutes]

Claude Code: "Check test progress"

→ Tool: cynic_get_job_status(job_id="test-...")

← Response:
   Status: running
   Progress: 45% [████████░░░░░░░░░░░░]
   ETA: 300 seconds

[After ~10 minutes]

Claude Code: "Get empirical results"

→ Tool: cynic_get_test_results(job_id="test-...")

← Response:
   Q-Scores: [45.2, 48.1, 51.3, ...]
   Average Q: 52.4
   Learning Efficiency: 1.18x
   Emergence Events: 3
```

### Pattern 4: Test Axiom Necessity

```
Claude Code: "Prove that the PHI axiom is irreducible"

→ Tool: cynic_test_axiom_irreducibility(axiom="PHI")

← Response:
   Axiom: PHI
   Baseline Q: 52.4
   Disabled Q: 38.1
   Impact: 27.3%
   Irreducible: YES ✓
```

### Pattern 5: Query Telemetry

```
Claude Code: "How long has CYNIC been running?"

→ Tool: cynic_query_telemetry(metric="uptime_s")

← Response:
   Uptime: 3600.0 seconds (1 hour)
   Q-Table Entries: 1024
   Total Judgments: 12500
   Learning Rate: 0.001
```

## How It Works

### Startup Sequence

1. Claude Code reads `~/.claude/mcp.json`
2. Finds `cynic-claude-code-bridge` entry
3. Launches subprocess: `bash run_mcp_bridge.sh`
4. Subprocess runs: `python -m cynic.interfaces.mcp.claude_code_bridge`
5. Python process starts MCP stdio server
6. Claude Code connects via stdin/stdout
7. Tools become available to Claude Code immediately

### Tool Routing Flow

```
Claude Code (User)
    ↓ (MCP request: "ask_cynic")
Run MCP Bridge subprocess
    ↓ (stdio)
cynic.interfaces.mcp.claude_code_bridge.list_tools()
    ↓ (returns tool definition)
cynic.interfaces.mcp.claude_code_bridge.call_tool("ask_cynic", args)
    ↓ (dispatcher)
_tool_ask_cynic(args)
    ↓ (HTTP call)
CYNIC HTTP Server (:8765)
    ↓
/judge endpoint
    ↓ (returns judgment)
_tool_ask_cynic() formats response
    ↓ (MCP response)
Claude Code receives judgment
    ↓ (displays to user)
User sees: "Q-Score: 72/100"
```

### Context Saving

**Without empirical tools**: Claude Code would need to consume ~500 tokens running 1000 tests locally
**With empirical tools**: CYNIC runs tests autonomously, Claude Code just calls `cynic_run_empirical_test()` + polls status
**Token savings**: ~400+ tokens per test run

## Prerequisites

Before using Claude Code with CYNIC:

### 1. CYNIC Server Must Be Running

```bash
cd ~/Desktop/asdfasdfa/CYNIC-clean/cynic
docker-compose up postgres ollama cynic
```

Verify:
```bash
curl http://localhost:8765/health
# Should return 200 OK
```

### 2. Python + Dependencies

The run_mcp_bridge.sh script requires:
- Python 3.9+
- `mcp` package (in pyproject.toml)
- `aiohttp` package (in pyproject.toml)

Install:
```bash
cd ~/Desktop/asdfasdfa/CYNIC-clean
pip install -e .
```

### 3. MCP Configuration

Already done! Your `~/.claude/mcp.json` is configured.

## Troubleshooting

### "Tool not found: ask_cynic"

**Cause**: MCP server didn't start
**Fix**:
```bash
# Test the script manually
bash ~/Desktop/asdfasdfa/CYNIC-clean/run_mcp_bridge.sh

# Should output MCP debug info and wait for stdin
```

### "Connection refused to http://localhost:8765"

**Cause**: CYNIC server not running
**Fix**:
```bash
# Ensure CYNIC container is up
docker-compose ps | grep cynic

# If not, start it
cd CYNIC-clean/cynic && docker-compose up -d cynic
```

### "ImportError: No module named 'mcp'"

**Cause**: Dependencies not installed
**Fix**:
```bash
pip install mcp aiohttp pydantic
```

### "JSON decode error in message pump"

**Cause**: MCP protocol mismatch on Windows
**Fix**:
```bash
# Try force-enabling Windows ProactorEventLoop
export PYTHONPATH=/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC-clean:$PYTHONPATH
bash ~/Desktop/asdfasdfa/CYNIC-clean/run_mcp_bridge.sh
```

## Tool Reference

### Consciousness Tools

| Tool | Purpose | Example |
|------|---------|---------|
| `ask_cynic` | Get CYNIC judgment | "Is this design good?" → Q-score: 72 |
| `observe_cynic` | Get organism state | "How many dogs are active?" |
| `learn_cynic` | Teach CYNIC | "That judgment was right" → updates Q-table |
| `discuss_cynic` | Bidirectional chat | "Why did you say that?" |

### Orchestration Tools

| Tool | Purpose |
|------|---------|
| `cynic_build` | Build Docker image |
| `cynic_deploy` | Deploy to dev/staging/prod |
| `cynic_health` | Check service health |
| `cynic_status` | Get kernel status |
| `cynic_release` | Create release + CHANGELOG |
| `cynic_stop` | Graceful shutdown |

### Empirical Tools

| Tool | Purpose | Time |
|------|---------|------|
| `cynic_run_empirical_test` | Start batch test | Returns immediately |
| `cynic_get_job_status` | Poll progress | Instant |
| `cynic_get_test_results` | Fetch results | Instant (if complete) |
| `cynic_test_axiom_irreducibility` | Test axiom necessity | 10-50 min per axiom |
| `cynic_query_telemetry` | Query SONA metrics | Instant |

## Examples

### Example 1: Code Review with CYNIC

```
Claude Code: "Review this function for quality"

ask_cynic(question="Code quality review", context="function code", reality="CODE")
→ CYNIC: "Q-Score: 68/100 - Good structure, consider error handling"
→ learn_cynic(judgment_id="j-...", rating=0.8)
→ CYNIC: "Q-Table updated, learning rate: 0.001"
```

### Example 2: Autonomous Learning Experiment

```
Claude Code: "Run 10000 judgments to test learning"

cynic_run_empirical_test(count=10000)
→ Job ID: test-2026-02-24-xyz

[Wait 100 seconds]

cynic_get_job_status(job_id="test-2026-02-24-xyz")
→ Progress: 100%, Status: complete

cynic_get_test_results(job_id="test-2026-02-24-xyz")
→ Results: avg_q=54.2, eff=1.22x, emergences=5
```

### Example 3: Axiom Validation

```
Claude Code: "Prove all 5 axioms are necessary"

cynic_test_axiom_irreducibility(axiom=None)
→ Tests PHI, VERIFY, CULTURE, BURN, FIDELITY
→ Results: All have >15% impact → All irreducible ✓
```

## Performance Notes

| Operation | Time | Context Cost |
|-----------|------|--------------|
| ask_cynic | 0.5-2s | ~50 tokens |
| observe_cynic | 0.2-0.5s | ~30 tokens |
| learn_cynic | 0.1-0.3s | ~20 tokens |
| run_empirical_test (1000 iter) | ~50-100s | ~10 tokens (async!) |
| get_job_status | 0.1s | ~10 tokens |
| test_axiom_irreducibility (1 axiom) | ~600s | ~15 tokens |

**Key insight**: Empirical tests don't consume Claude Code context — CYNIC runs them autonomously!

## Next Steps (Priority 2 → 3 → 4)

✅ Priority 1: HTTP endpoints (done)
✅ Priority 2: Plugin hook registration (done)
→ Priority 3: Extended telemetry streaming (WebSocket)
→ Priority 4: Dedicated Solana integration

## Files

| File | Purpose |
|------|---------|
| `run_mcp_bridge.sh` | Startup script for MCP server |
| `cynic/mcp/claude_code_bridge.py` | MCP server implementation |
| `cynic/api/routers/empirical.py` | HTTP endpoints for tests |
| `~/.claude/mcp.json` | Claude Code MCP registration |
| `CLAUDE-CODE-SETUP.md` | This file |

## Support

If Claude Code's CYNIC tools aren't working:

1. **Check logs**: `docker-compose logs cynic` (HTTP server)
2. **Test endpoint**: `curl http://localhost:8765/health`
3. **Restart bridge**: Close Claude Code, reopen (restarts MCP server)
4. **Check MCP config**: `cat ~/.claude/mcp.json`

---

**Confidence**: 82% (registration pattern proven, HTTP integration solid)

*sniff* "The dog teaches the tool. The tool extends the dog." — κυνικός
