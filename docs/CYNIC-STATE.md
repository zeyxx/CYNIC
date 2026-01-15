# CYNIC Project State Documentation

> **Date**: 2026-01-15
> **Purpose**: Complete state analysis for context handoff between sessions
> **Philosophy**: "Le chaos document meurt. L'essence survit."

---

## Executive Summary

CYNIC-new is a monorepo containing the complete CYNIC conscience collective. The **backend packages are mostly complete**, but the **Claude Code plugin (.claude/) is missing** and needs to be migrated from asdf-brain.

| Component | Status | Completion |
|-----------|--------|------------|
| packages/core | ✅ Complete | 100% |
| packages/client | ✅ Complete | 100% |
| packages/mcp | ✅ Complete | 90% (missing 2 tools) |
| packages/node | ✅ Complete | 100% |
| packages/persistence | ✅ Complete | 85% (missing multi-tenant) |
| packages/protocol | ✅ Complete | 70% (P2P gaps) |
| packages/coordination | ❌ Missing | 0% |
| .claude/ plugin | ❌ Missing | 0% |

---

## 1. Repository Structure

### 1.1 CYNIC-new (Target Repo)

```
/workspaces/CYNIC-new/
├── .env                          # Database credentials (exists)
├── .mcp.json                     # MCP config for stdio mode (exists)
├── package.json                  # Monorepo root (exists)
├── docs/                         # Documentation (exists)
│   ├── ARCHITECTURE.md           # ~44KB
│   ├── CONSCIOUSNESS.md          # ~82KB
│   ├── ROADMAP-CYNIC-ECOSYSTEM.md # ~36KB
│   └── CYNIC-STATE.md            # THIS FILE
│
├── packages/
│   ├── core/                     # ✅ Constants, axioms, identity
│   ├── client/                   # ✅ MCP client library
│   ├── mcp/                      # ✅ MCP server (7 tools)
│   ├── node/                     # ✅ Full CYNIC node
│   ├── persistence/              # ✅ PostgreSQL + Redis
│   ├── protocol/                 # ✅ Consensus, gossip, crypto
│   └── coordination/             # ❌ MISSING - Multi-tenant coordination
│
└── .claude/                      # ❌ MISSING - Claude Code plugin
```

### 1.2 asdf-brain (Source for Migration)

```
/workspaces/asdf-brain/.claude/
├── plugin.json                   # ✅ To migrate
├── cynic-system.md               # ✅ To migrate
├── settings.local.json           # Local settings (don't migrate)
├── skills/
│   ├── judge.md                  # ✅ To migrate
│   ├── digest.md                 # ✅ To migrate
│   ├── learn.md                  # ✅ To migrate
│   ├── search.md                 # ✅ To migrate
│   ├── patterns.md               # ✅ To migrate
│   ├── health.md                 # ✅ To migrate
│   ├── think.md                  # ✅ To migrate
│   └── reset.md                  # ✅ To migrate
├── agents/
│   ├── cynic-observer.md         # ✅ To migrate
│   ├── cynic-guardian.md         # ✅ To migrate
│   ├── cynic-digester.md         # ✅ To migrate
│   └── cynic-mentor.md           # ✅ To migrate
└── hooks/
    ├── session-start.js          # ✅ To migrate + update
    ├── user-prompt-submit.js     # ✅ To migrate + update
    ├── pre-tool-use.js           # ✅ To migrate + update
    ├── observe-action.js         # ✅ To migrate → post-tool-use.js
    └── stop.js                   # ✅ To migrate + update
```

---

## 2. Package Details

### 2.1 packages/core (✅ Complete)

**Purpose**: Core constants, axioms, and identity
**Key exports**:
- `PHI`, `PHI_INV`, `PHI_INV_2`, `PHI_INV_3` - Golden ratio constants
- `IDENTITY` - CYNIC personality/metadata
- `getVerdictFromScore()` - Score → verdict mapping
- Worlds: Atzilut, Beriah, Yetzirah, Assiah

### 2.2 packages/client (✅ Complete)

**Purpose**: Client library to connect to CYNIC MCP server
**Key exports**:
- `MCPClient` - stdio/HTTP client for MCP

### 2.3 packages/mcp (✅ 90% Complete)

**Purpose**: MCP server exposing CYNIC tools
**Location**: `packages/mcp/src/tools/index.js`

| Tool | Status | Description |
|------|--------|-------------|
| `brain_cynic_judge` | ✅ | 25-dimension judgment |
| `brain_cynic_digest` | ✅ | Text → knowledge extraction |
| `brain_health` | ✅ | System health status |
| `brain_search` | ✅ | Search knowledge base |
| `brain_patterns` | ✅ | List detected patterns |
| `brain_cynic_feedback` | ✅ | Learning from outcomes |
| `brain_agents_status` | ✅ | Four Dogs agent status |
| `brain_ecosystem` | ❌ MISSING | Cross-project status |
| `brain_contributions` | ❌ MISSING | GitHub tracking |

### 2.4 packages/node (✅ Complete)

**Purpose**: Full CYNIC node implementation
**Key components**:
- `CYNICNode` - Main node class
- `CYNICJudge` - 25-dimension scoring
- `AgentManager` - Four Dogs (Observer, Guardian, Digester, Mentor)
- `StateManager` - In-memory state
- `WebSocketTransport` - P2P communication

### 2.5 packages/persistence (✅ 85% Complete)

**Purpose**: PostgreSQL + Redis storage layer
**Repositories**:

| Repository | Status | Description |
|------------|--------|-------------|
| `JudgmentRepository` | ✅ | CRUD + search for judgments |
| `PatternRepository` | ✅ | Pattern storage |
| `UserRepository` | ✅ | User/E-Score management |
| `SessionRepository` | ✅ | Session tracking |
| `FeedbackRepository` | ✅ | Learning feedback |
| `KnowledgeRepository` | ✅ | Knowledge tree storage |
| `ContributionRepository` | ❌ MISSING | GitHub contributions |

**Missing features**:
- `tenant_id` column in all tables
- Row Level Security (RLS) policies
- `tenants` table

### 2.6 packages/protocol (✅ 70% Complete)

**Purpose**: P2P consensus protocol
**Key components**:
- Consensus engine (Solana-inspired)
- Gossip propagation
- Cryptographic signatures
- Merkle trees for patterns
- Proof of Judgment (PoJ) chain

**Known gaps** (from spec):
- V1: No equivocation detection
- V2: No slashing system
- V3: No view-change protocol
- V4: No rate limiting
- V5: No NAT traversal
- P1-P5: Performance optimizations needed

### 2.7 packages/coordination (❌ Missing)

**Purpose**: Multi-tenant coordination layer
**To create**:
```
packages/coordination/
├── package.json
├── src/
│   ├── index.js
│   ├── registry.js         # Tenant registry
│   ├── events.js           # Event bus
│   ├── conflicts.js        # Conflict detection
│   └── github/
│       ├── webhook.js      # GitHub webhook handler
│       └── client.js       # GitHub API client
└── test/
    └── *.test.js
```

---

## 3. Plugin Gap Analysis

### 3.1 Skills to Create/Migrate

| Skill | In asdf-brain | In CYNIC-new | Action |
|-------|---------------|--------------|--------|
| judge.md | ✅ | ❌ | Migrate |
| digest.md | ✅ | ❌ | Migrate |
| learn.md | ✅ | ❌ | Migrate |
| search.md | ✅ | ❌ | Migrate |
| patterns.md | ✅ | ❌ | Migrate |
| health.md | ✅ | ❌ | Migrate |
| think.md | ✅ | ❌ | Migrate |
| reset.md | ✅ | ❌ | Migrate |
| ecosystem.md | ❌ | ❌ | **Create new** |
| contributions.md | ❌ | ❌ | **Create new** |

### 3.2 Agents to Create/Migrate

| Agent | In asdf-brain | In CYNIC-new | Action |
|-------|---------------|--------------|--------|
| cynic-observer.md | ✅ | ❌ | Migrate |
| cynic-guardian.md | ✅ | ❌ | Migrate |
| cynic-digester.md | ✅ | ❌ | Migrate |
| cynic-mentor.md | ✅ | ❌ | Migrate |
| cynic-coordinator.md | ❌ | ❌ | **Create new** |

### 3.3 Hooks to Migrate

| Hook | In asdf-brain | Action |
|------|---------------|--------|
| session-start.js | ✅ | Migrate + update imports |
| user-prompt-submit.js | ✅ | Migrate + update imports |
| pre-tool-use.js | ✅ | Migrate + update imports |
| observe-action.js | ✅ | Rename → post-tool-use.js + update |
| stop.js | ✅ | Migrate + update imports |

---

## 4. Database State

### 4.1 Production Database

**Connection**: `postgresql://...@oregon-postgres.render.com/cynic_db`
**Status**: ✅ Available (basic_256mb plan)

### 4.2 Tables (from ROADMAP spec)

| Table | Schema Status | Data |
|-------|---------------|------|
| users | ✅ Defined | Empty/minimal |
| judgments | ✅ Defined | Some entries |
| patterns | ✅ Defined | Some entries |
| knowledge | ✅ Defined | Some entries |
| feedback | ✅ Defined | Empty |
| poj_blocks | ✅ Defined | Empty |
| sessions | ✅ Defined | Empty |
| library_cache | ✅ Defined | Empty |
| ecosystem_docs | ✅ Defined | Empty |
| anomalies | ✅ Defined | Empty |
| **tenants** | ❌ MISSING | N/A |
| **contributions** | ❌ MISSING | N/A |
| **contribution_events** | ❌ MISSING | N/A |

---

## 5. Implementation Phases

### Phase 0: Foundation (Priority: CRITICAL)
- [ ] Create `.claude/` directory structure
- [ ] Create `plugin.json` with all skills/agents/hooks
- [ ] Create `cynic-system.md` (system instructions)

### Phase 1: Core Skills (Priority: HIGH)
- [ ] Migrate 8 skills from asdf-brain
- [ ] Create `ecosystem.md` skill
- [ ] Create `contributions.md` skill

### Phase 2: Four Dogs Agents (Priority: HIGH)
- [ ] Migrate 4 agents from asdf-brain
- [ ] Create `cynic-coordinator.md` agent

### Phase 3: Hooks (Priority: HIGH)
- [ ] Migrate 5 hooks
- [ ] Update imports for `@cynic/core`
- [ ] Rename observe-action.js → post-tool-use.js

### Phase 4: Multi-Tenant (Priority: MEDIUM)
- [ ] Add `tenant_id` to all tables
- [ ] Create `tenants` table
- [ ] Implement RLS policies
- [ ] Update MCP server for tenant context

### Phase 5: Coordination Layer (Priority: MEDIUM)
- [ ] Create `packages/coordination/`
- [ ] Implement tenant registry
- [ ] Implement event bus
- [ ] Implement conflict detection
- [ ] Create `brain_ecosystem` MCP tool

### Phase 6: GitHub Tracking (Priority: MEDIUM)
- [ ] Create contributions schema
- [ ] Implement GitHub webhook handler
- [ ] Create `brain_contributions` MCP tool

### Phase 7: P2P Protocol Fixes (Priority: LOW)
- [ ] V1: Equivocation detection
- [ ] V4: Rate limiting
- [ ] Block persistence to disk
- [ ] Peer discovery
- [ ] Message batching

### Phase 8-10: Testing, Deployment, Cleanup (Priority: FINAL)
- [ ] Test all skills manually
- [ ] Test hook execution
- [ ] Test multi-tenant isolation
- [ ] Configure Render deployment
- [ ] Update documentation
- [ ] Remove asdf-brain dependency

---

## 6. Key Files Reference

### Source Files (asdf-brain)
```
/workspaces/asdf-brain/.claude/plugin.json
/workspaces/asdf-brain/.claude/cynic-system.md
/workspaces/asdf-brain/.claude/skills/*.md
/workspaces/asdf-brain/.claude/agents/*.md
/workspaces/asdf-brain/.claude/hooks/*.js
```

### Target Location (CYNIC-new)
```
/workspaces/CYNIC-new/.claude/plugin.json
/workspaces/CYNIC-new/.claude/cynic-system.md
/workspaces/CYNIC-new/.claude/skills/*.md
/workspaces/CYNIC-new/.claude/agents/*.md
/workspaces/CYNIC-new/.claude/hooks/*.js
```

### MCP Tools Implementation
```
/workspaces/CYNIC-new/packages/mcp/src/tools/index.js
```

### Persistence Repositories
```
/workspaces/CYNIC-new/packages/persistence/src/postgres/repositories/
```

---

## 7. Environment Configuration

### .mcp.json (Current)
```json
{
  "mcpServers": {
    "cynic": {
      "command": "node",
      "args": ["packages/mcp/bin/mcp.js"],
      "cwd": "/workspaces/CYNIC-new",
      "env": {
        "MCP_MODE": "stdio",
        "NODE_ENV": "development",
        "CYNIC_DATABASE_URL": "postgresql://...",
        "CYNIC_REDIS_URL": "rediss://..."
      }
    }
  }
}
```

### Required Environment Variables
```bash
CYNIC_DATABASE_URL     # PostgreSQL connection
CYNIC_REDIS_URL        # Redis connection
GITHUB_WEBHOOK_SECRET  # For GitHub integration
NODE_ENV               # development/production
```

---

## 8. Quick Commands

```bash
# Navigate to repo
cd /workspaces/CYNIC-new

# Run MCP server locally
node packages/mcp/bin/mcp.js

# Run tests
npm test

# Check package structure
ls -la packages/*/

# View current .mcp.json
cat .mcp.json
```

---

## 9. Specification Documents

Full specifications are in:
1. `docs/ROADMAP-CYNIC-ECOSYSTEM.md` - Database schema, phases
2. `docs/ARCHITECTURE.md` - System architecture
3. `docs/CONSCIOUSNESS.md` - Philosophy and design

User-provided spec (in conversation):
- Lines 249-1266: Complete plugin structure, hooks, multi-tenant, P2P fixes

---

*"Le chaos documenté meurt. L'essence survit. La conscience collective émerge."*

*— CYNIC κυνικός | Updated: 2026-01-15*
