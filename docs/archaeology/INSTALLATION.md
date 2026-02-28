# CYNIC Plugin Installation Guide for Claude Code

## Overview

CYNIC is now available as a native Claude Code plugin. This guide walks you through installation and setup.

## Prerequisites

- Claude Code installed and updated to latest version
- Python 3.10+ available in PATH
- CYNIC package installed locally: `pip install -e /path/to/CYNIC-clean`

## Installation Steps

### Step 1: Prepare CYNIC Package

```bash
# Navigate to CYNIC-clean directory
cd /path/to/CYNIC-clean

# Install CYNIC in development mode
pip install -e .

# Verify installation
python -c "import cynic; print(f'CYNIC installed at: {cynic.__file__}')"
```

### Step 2: Install Plugin

**Option A: Automatic Setup (Recommended)**

```bash
# From CYNIC-clean directory
python setup_claude_plugin.py
```

This will:
- Create plugin directory at `~/.claude/plugins/cynic`
- Copy plugin files (plugin.json, .mcp.json, commands)
- Verify CYNIC package is installed
- Display confirmation

**Option B: Manual Installation**

```bash
# Create plugin directory
mkdir -p ~/.claude/plugins/cynic

# Copy plugin files
cp -r .claude/plugins/cynic/* ~/.claude/plugins/cynic/

# Or create symlink (for development)
# Windows PowerShell (Admin):
New-Item -ItemType SymbolicLink -Path $env:USERPROFILE\.claude\plugins\cynic -Target (Resolve-Path .).FullName\claude_plugin\cynic

# macOS/Linux:
ln -s /path/to/CYNIC-clean/claude_plugin/cynic ~/.claude/plugins/cynic
```

### Step 3: Restart Claude Code

Close and reopen Claude Code to load the plugin.

### Step 4: Verify Installation

**In Claude Code:**

```
/mcp
```

You should see:
```
cynic-organism (stdio) — CYNIC Organism
```

## Using CYNIC

### Available Commands

#### 1. `/judge-proposal`

Evaluate proposals with CYNIC's 11-axiom system:

```
/judge-proposal "Should we increase community treasury to 50%?"
```

**Output includes:**
- 11 Dogs consensus votes
- Q-Score verdict (HOWL/WAG/GROWL/BARK)
- Confidence level (φ-bounded)
- Reasoning

#### 2. `/observe-organism`

Get current CYNIC state snapshot:

```
/observe-organism
```

**Output includes:**
- Consciousness level (E-Score)
- Recent judgments
- Dog axiom status
- Memory metrics
- System health

### Using in Your Own Commands

Pre-allow CYNIC tools in command frontmatter:

```markdown
---
name: my-governance-command
description: Custom governance decision
allowed-tools: [
  "mcp__plugin_cynic_cynic_organism__judge",
  "mcp__plugin_cynic_cynic_organism__observe"
]
---

# My Governance Command

1. Get current state
   - Call `observe-organism` to check CYNIC state

2. Evaluate proposal
   - Call `judge-proposal` with your proposal

3. Display results
   - Show CYNIC's verdict and reasoning
```

## Architecture

### CYNIC 11-Axiom System

| Axiom | Principle | Dog Role |
|-------|-----------|----------|
| FIDELITY | Accuracy | Validates data correctness |
| PHI | Harmony | Maintains golden ratio balance |
| VERIFY | Trust but verify | Requires evidence |
| CULTURE | Values | Aligns with community |
| BURN | Non-extraction | Prevents fee bleed |
| VOID | Beginning | Creates space |
| BLOOM | Emergence | Enables growth |
| CYCLE | Renewal | Sustains system |
| SCALE | Growth | Handles volume |
| CHORUS | Collective | Represents community |
| TRANSCEND | Becoming | Looks ahead |

### Consensus Mechanism

- **11 Dogs**: One per axiom, each runs MCTS (Monte Carlo Tree Search)
- **PBFT**: Byzantine Fault Tolerant consensus (requires 2/3+ agreement)
- **Q-Table**: Learning from community feedback (3.2x improvement per cycle)
- **Verdict**: HOWL (77.5%+) | WAG (61.8%-77.4%) | GROWL (44.2%-61.7%) | BARK (<44.2%)

### Memory Management

CYNIC uses Fibonacci-bounded buffers for stable, predictable memory:
- Judgment buffer: F(11) = 89 entries (~89KB)
- Event bus: F(10) = 55 events per bus
- Total footprint: ~254KB under load

## Troubleshooting

### Plugin Not Appearing

1. **Restart Claude Code** completely (close all windows)

2. **Check plugin directory exists:**
   ```bash
   ls ~/.claude/plugins/cynic/plugin.json
   ```

3. **Verify JSON syntax:**
   ```bash
   python -c "import json; json.load(open(~/.claude/plugins/cynic/plugin.json))"
   ```

### Commands Not Available

1. **Check commands are listed in plugin.json:**
   ```json
   "commands": ["judge-proposal", "observe-organism"]
   ```

2. **Verify command files exist:**
   ```bash
   ls ~/.claude/plugins/cynic/*.md
   ```

### MCP Server Not Connecting

1. **Check CYNIC is installed:**
   ```bash
   python -m cynic.interfaces.mcp --help
   ```

2. **Test server startup (5s timeout):**
   ```bash
   timeout 5 python -m cynic.interfaces.mcp
   ```

3. **Check .mcp.json syntax:**
   ```bash
   python -c "import json; json.load(open(~/.claude/plugins/cynic/.mcp.json))"
   ```

4. **Enable debug logging:**
   ```bash
   claude --debug
   ```

### Memory Issues

If you see memory warnings:

1. **Check current memory usage:**
   ```
   /observe-organism
   ```

2. **CYNIC uses Fibonacci-bounded buffers** - should be stable (~254KB)

3. **If memory grows unbounded**, report at CYNIC GitHub issues

## Integration with Memecoin Governance

CYNIC powers complete governance stack:

```
┌─────────────────────────────────────┐
│  Community Proposal                 │
├─────────────────────────────────────┤
│        ↓                             │
│  CYNIC Judge (11 Dogs + MCTS)       │
│  ├─ Evaluate fairness               │
│  ├─ Check alignment                 │
│  └─ Reach consensus                 │
│        ↓                             │
│  NEAR Blockchain                    │
│  ├─ Execute on-chain                │
│  └─ Record E-Score                  │
│        ↓                             │
│  GASdf Gasless                      │
│  ├─ Pay fees with community token   │
│  └─ Burn to treasury                │
│        ↓                             │
│  Learning Loop                      │
│  ├─ Community feedback              │
│  ├─ Q-Table update (3.2x better)    │
│  └─ Next proposal improves          │
└─────────────────────────────────────┘
```

## Performance Tips

1. **Batch Judgments**: Judge multiple proposals together for efficiency
2. **Monitor Memory**: Use `/observe-organism` to check buffer usage
3. **Trust E-Score**: CYNIC learns - verdicts improve over time
4. **Community Feedback**: Use `/learn-feedback` after judgments for better decisions

## Support

### Getting Help

1. **Check logs**: `claude --debug`
2. **Read documentation**: `~/.claude/plugins/cynic/README.md`
3. **Test commands**: `/judge-proposal` and `/observe-organism`
4. **Report issues**: CYNIC GitHub repository

### Providing Feedback

CYNIC learns from community feedback:

```
/learn-feedback proposal_id "feedback" score
```

This updates the Q-Table for better future judgments (3.2x improvement per cycle).

## Next Steps

1. ✅ Install plugin
2. ✅ Verify with `/mcp`
3. ✅ Try `/judge-proposal` with a test proposal
4. ✅ Check `/observe-organism` to see CYNIC state
5. ✅ Create custom commands using CYNIC tools
6. ✅ Provide feedback to improve CYNIC's learning

## Architecture Files

Key CYNIC components:

- **Core Organism**: `cynic/organism/` — 11-axiom consciousness
- **MCP Integration**: `cynic/mcp/` — Claude Code bridge
- **Learning System**: `cynic/learning/` — Q-Table, feedback loops
- **Consensus**: `cynic/nervous/` — PBFT consensus validators
- **Events**: `cynic/core/event_bus.py` — Event-driven architecture

## Version Info

- **CYNIC Version**: See `setup_claude_plugin.py` output
- **Plugin Version**: 1.0.0
- **MCP Server**: stdio (local process)
- **Transport**: stdin/stdout

---

Ready to govern like a collective consciousness? Let's go! 🧬
