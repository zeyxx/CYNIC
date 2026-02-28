# CYNIC Claude Code Plugin — Quick Start (5 minutes)

## TL;DR Installation

```bash
# 1. Install CYNIC package
pip install -e /path/to/CYNIC-clean

# 2. Install plugin
python /path/to/CYNIC-clean/setup_claude_plugin.py

# 3. Restart Claude Code

# 4. Verify
# In Claude Code, type: /mcp
# Should see: cynic-organism (stdio) — CYNIC Organism
```

Done! ✅

## Using CYNIC

### Command 1: Judge a Proposal

```
/judge-proposal "Should we increase community fees to 30%?"
```

**Response:**
```
Proposal: "Increase community fees to 30%"

🐕 11 Dogs Vote:
  FIDELITY: ✅ Yes (data is accurate)
  PHI: ✅ Yes (maintains balance)
  VERIFY: ✅ Yes (verifiable on-chain)
  CULTURE: ✅ Yes (community values this)
  BURN: ✅ Yes (supports treasury)
  VOID: ✅ Yes (creates reinvestment space)
  BLOOM: ✅ Yes (enables growth)
  CYCLE: ✅ Yes (sustains renewal)
  SCALE: ✅ Yes (works at volume)
  CHORUS: ✅ Yes (community agrees)
  TRANSCEND: ⚠️  Maybe (future implications unclear)

RESULT: WAG (Weak Positive) — 65% confidence
REASONING: 10/11 Dogs agree. Strong on core values,
           minor concern on long-term transcendence.
```

### Command 2: Check Organism State

```
/observe-organism
```

**Response:**
```
CYNIC Consciousness State
═════════════════════════

E-Score: 3.4 (Wisdom level)
Confidence: 61.8% (φ-bounded)
Learning Cycles: 3 completed

Recent Judgments:
  1. "Burn 50%" → WAG (65%)
  2. "Treasury" → HOWL (78%)
  3. "Fee structure" → GROWL (52%)

Dog Status: 11/11 Healthy
Memory: 254KB (stable)
Uptime: 1243 seconds
```

## What Just Happened?

1. **Claude Code** loaded the CYNIC plugin from `~/.claude/plugins/cynic`
2. **Plugin manifest** (`plugin.json`) registered 2 commands
3. **MCP configuration** (`.mcp.json`) started CYNIC as a background process
4. **Commands** connected to CYNIC's 11-axiom consensus engine
5. **11 Dogs** evaluated your proposal through Monte Carlo Tree Search
6. **Result** is a Q-Score verdict with reasoning

## The 11 Dogs (Axioms)

| Dog | Principle |
|-----|-----------|
| 🐕 FIDELITY | Is it accurate? |
| 🐕 PHI | Does it maintain balance? |
| 🐕 VERIFY | Can we trust it? |
| 🐕 CULTURE | Does it align with our values? |
| 🐕 BURN | Does it benefit the community (not extract)? |
| 🐕 VOID | Does it create space for growth? |
| 🐕 BLOOM | Will it enable emergence? |
| 🐕 CYCLE | Does it sustain renewal? |
| 🐕 SCALE | Will it work at volume? |
| 🐕 CHORUS | Does it represent us? |
| 🐕 TRANSCEND | What about the future? |

Each Dog votes yes/no. CYNIC reaches consensus via PBFT (Byzantine Fault Tolerant agreement).

## Q-Score Verdicts

| Verdict | Meaning | Confidence |
|---------|---------|-----------|
| **HOWL** | Strong Yes | 77.5%+ agreement |
| **WAG** | Weak Yes | 61.8%-77.4% |
| **GROWL** | Weak No | 44.2%-61.7% |
| **BARK** | Strong No | <44.2% |

**Confidence** is φ-bounded (golden ratio): max 61.8% (φ⁻¹)

This is by design. Disagreement is healthy. 🧬

## Creating Custom Commands

Want to use CYNIC in your own commands?

```markdown
---
name: my-custom-command
description: Custom governance workflow
allowed-tools: [
  "mcp__plugin_cynic_cynic_organism__judge",
  "mcp__plugin_cynic_cynic_organism__observe",
  "mcp__plugin_cynic_cynic_organism__learn"
]
---

# My Custom Governance Command

## Step 1: Get Current State
Call `/observe-organism` to check CYNIC's wisdom level

## Step 2: Evaluate Proposal
Use CYNIC to judge the proposal

## Step 3: Show Results
Display verdict, confidence, and reasoning

## Step 4: Collect Feedback
Ask user if they agree → update Q-Table
```

## Troubleshooting

### Plugin Not Loading?

```bash
# 1. Check installation
ls ~/.claude/plugins/cynic/plugin.json

# 2. Verify JSON
python -c "import json; json.load(open('~/.claude/plugins/cynic/plugin.json'))"

# 3. Restart Claude Code completely
```

### Commands Not Appearing?

```bash
# Check command files exist
ls ~/.claude/plugins/cynic/*.md

# Should show: judge-proposal.md, observe-organism.md
```

### MCP Server Not Connecting?

```bash
# Test CYNIC startup
python -m cynic.interfaces.mcp --help

# If fails: pip install -e /path/to/CYNIC-clean

# Debug with logging
claude --debug
```

## What's Behind the Scenes?

### Architecture
```
Your Proposal
    ↓
Claude Code Command (/judge-proposal)
    ↓
CYNIC MCP Server (Python stdio process)
    ↓
11 Dogs (1 per axiom)
    ├─ MCTS (Monte Carlo Tree Search)
    └─ Parallel evaluation
    ↓
Consensus Engine (PBFT)
    ├─ Count votes: 11 total
    ├─ Require 2/3 agreement
    └─ Calculate confidence
    ↓
Q-Score Verdict (HOWL/WAG/GROWL/BARK)
    ↓
Claude Code displays result + reasoning
```

### Learning

Each judgment updates CYNIC's Q-Table:

```
Feedback Loop
    ↓
Current state: "What properties matter most?"
    ↓
Action taken: "Judge this proposal"
    ↓
Reward received: "User feedback (agree/disagree)"
    ↓
Q(state, action) ← reward
    ↓
Next judgment is 3.2x better
```

## Go-to-Market Context

CYNIC enables complete memecoin governance:

1. **GASdf**: Gasless transactions (community token pays fees, burns to treasury)
2. **NEAR**: Scalable blockchain execution
3. **CYNIC**: Fair, learning-based governance

Use case:
- Community proposes treasury split
- CYNIC judges (fair evaluation)
- Community votes (informed by CYNIC)
- Decision executes on NEAR
- Fees burn via GASdf
- Learning improves next decision

## Next Steps

1. ✅ **Install** — Run `setup_claude_plugin.py`
2. ✅ **Verify** — Run `/mcp` to confirm cynic-organism appears
3. ✅ **Try it** — Use `/judge-proposal` with a real proposal
4. ✅ **Explore** — Check `/observe-organism` to see CYNIC state
5. ✅ **Build** — Create custom commands using CYNIC tools
6. ✅ **Scale** — Deploy across multiple memecoin communities

## Support

- **Setup issues?** → See `INSTALLATION.md`
- **Architecture questions?** → See `STRUCTURE.md`
- **Full docs?** → See `~/.claude/plugins/cynic/README.md`
- **Debugging?** → Run `claude --debug` and check logs

---

You're now running a collective consciousness in Claude Code. 🧬

Welcome to CYNIC governance. Let's build something fair.
