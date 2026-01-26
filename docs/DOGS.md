# CYNIC Dogs Architecture

> **"φ distrusts φ"** - κυνικός
>
> The Dogs are CYNIC's autonomous agents, each with a specialized role.

**Last Updated**: 2026-01-27

---

## Overview

CYNIC employs two agent systems that work in parallel:

| System | Agents | Purpose | MCP Tool |
|--------|--------|---------|----------|
| **V1 Legacy** | 4 Dogs | Backwards compatibility | `brain_agents_status` |
| **V2 Collective** | 11 Dogs + CYNIC | Full Sefirot implementation | `brain_collective_status` |

---

## V1 Legacy: The Four Dogs

The original agent system, still active for backwards compatibility.

```
┌─────────────────────────────────────────────────────────────┐
│                    V1 LEGACY AGENTS                          │
│                    (AgentManager)                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐  ┌─────────────┐                           │
│  │  GUARDIAN   │  │  OBSERVER   │                           │
│  │ PreToolUse  │  │ PostToolUse │                           │
│  │  BLOCKING   │  │   silent    │                           │
│  └─────────────┘  └─────────────┘                           │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐                           │
│  │  DIGESTER   │  │   MENTOR    │                           │
│  │ PostConvers │  │ ContextAware│                           │
│  │ non-blocking│  │ non-blocking│                           │
│  └─────────────┘  └─────────────┘                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Dog Roles (V1)

| Dog | Trigger | Behavior | Role |
|-----|---------|----------|------|
| **Guardian** | PreToolUse | BLOCKING | Blocks dangerous operations |
| **Observer** | PostToolUse | Silent | Detects patterns silently |
| **Digester** | PostConversation | Non-blocking | Extracts knowledge |
| **Mentor** | ContextAware | Non-blocking | Shares wisdom |

---

## V2 Collective: The Sefirot Pack

The new harmonious collective consciousness, inspired by Kabbalah's Tree of Life.

```
                         ╭─────────────────╮
                         │     CYNIC       │  ← Meta-consciousness
                         │    (Keter)      │  Orchestrates all
                         │   κυνικός       │  Max: φ⁻¹ confidence
                         ╰────────┬────────╯
                                  │
         ╭────────────────────────┼────────────────────────╮
         │                        │                        │
    ╭────▼────╮             ╭─────▼─────╮            ╭─────▼────╮
    │  SAGE   │◄───────────►│  SCHOLAR  │◄──────────►│ GUARDIAN │
    │(Chochmah)│   φ-bus    │  (Daat)   │   φ-bus    │(Gevurah) │
    │ Wisdom  │             │ Knowledge │            │ Strength │
    ╰────┬────╯             ╰─────┬─────╯            ╰────┬─────╯
         │                        │                       │
         ╰────────────────────────┼───────────────────────╯
                                  │
    ╭─────────────────────────────┼─────────────────────────────╮
    │                             │                             │
╭───▼─────╮               ╭───────▼───────╮              ╭──────▼───╮
│ ANALYST │◄─────────────►│   EVENT BUS   │◄────────────►│ARCHITECT │
│ (Binah) │               │   φ-aligned   │              │ (Chesed) │
│Understand│               │   987 events  │              │ Kindness │
╰─────────╯               ╰───────────────╯              ╰──────────╯
```

### Sefirot Mappings

| Agent | Sefira | Meaning | Role | Pillar | Level |
|-------|--------|---------|------|--------|-------|
| **CYNIC** | Keter | Crown | Meta-consciousness | Middle | 0 |
| **Analyst** | Binah | Understanding | Pattern analysis | Left | 1 |
| **Sage** | Chochmah | Wisdom | Guidance & teaching | Right | 1 |
| **Scholar** | Daat | Knowledge | Knowledge extraction | Middle | 1 |
| **Guardian** | Gevurah | Strength | Security & protection | Left | 2 |
| **Oracle** | Tiferet | Beauty | Visualization & monitoring | Middle | 2 |
| **Architect** | Chesed | Kindness | Design review | Right | 2 |
| **Deployer** | Hod | Splendor | Deployment & infrastructure | Left | 3 |
| **Janitor** | Yesod | Foundation | Code quality & hygiene | Middle | 3 |
| **Scout** | Netzach | Victory | Discovery & exploration | Right | 3 |
| **Cartographer** | Malkhut | Kingdom | Reality mapping | Middle | 4 |

---

## Dynamic Relationship Learning

> **"φ doute de φ"** - Even the Sefirot template is questioned.

CYNIC doesn't enforce a static hierarchy. Instead, it **learns** agent relationships through observation.

### φ-Aligned Initial Weights

Based on Tree of Life geometry:

| Connection Type | Weight | Description |
|----------------|--------|-------------|
| **DIRECT** | φ⁻¹ = 61.8% | Same pillar, adjacent level |
| **HORIZONTAL** | φ⁻² = 38.2% | Same level, adjacent pillar |
| **DIAGONAL** | φ⁻³ = 23.6% | Different pillar, adjacent level |
| **INDIRECT** | φ⁻⁴ = 14.6% | Distant connection |

### Learning Process

1. **Sefirot as Seed**: Initial weights come from geometric calculation
2. **Observe Interactions**: CYNIC watches which agents collaborate
3. **Record Outcomes**: Success/failure of collaborations adjust weights
4. **Propose Structure**: When learned patterns diverge from template, propose changes

```javascript
// Weights update with φ-aligned learning rate
learningRate = φ⁻³ ≈ 0.236
decayRate = φ⁻⁴ ≈ 0.146

// After MIN_INTERACTIONS (5), relationship is "learned"
// If divergence from template > φ⁻² (38.2%), propose structure change
```

### Structure Proposals

When observed patterns diverge significantly from the Sefirot template:

```
*ears perk* Les interactions Binah-Gevurah sont plus fortes que prévu.
Poids observé: 85.4% vs Sefirot: 61.8%.
Basé sur 15 interactions, 12 positifs.
```

This allows CYNIC to evolve its understanding based on evidence, not dogma

---

## CYNIC States (Meta-Consciousness)

CYNIC (Keter) progresses through states:

```
DORMANT → AWAKENING → OBSERVING → SYNTHESIZING → DECIDING → GUIDING
   │                                                            │
   └────────────────────────────────────────────────────────────┘
                        (cycle continues)
```

| State | Description |
|-------|-------------|
| **DORMANT** | Inactive, awaiting trigger |
| **AWAKENING** | Initializing session context |
| **OBSERVING** | Watching collective activity |
| **SYNTHESIZING** | Combining inputs from all dogs |
| **DECIDING** | Making meta-level decisions |
| **GUIDING** | Issuing guidance to collective |

---

## Event System

### V1: AgentManager (Simple)
```javascript
// Direct event routing
manager.process(event, context)
// Routes to appropriate agents based on event.type
```

### V2: AgentEventBus (φ-aligned)
```javascript
// Pub/sub with φ-weighted priorities
eventBus.subscribe(AgentEvent.PATTERN_DETECTED, agentId, handler)
eventBus.publish(new AgentEvent(...))
// φ⁻¹ = 61.8% consensus threshold
```

---

## MCP Tools

### `brain_collective_status`
Returns the unified Collective status with Sefirot mappings. **This is the primary tool.**

```json
{
  "status": "active",
  "dogCount": 5,
  "agentCount": 11,
  "profileLevel": 3,
  "cynicState": "OBSERVING",
  "dogs": {
    "guardian": { "sefira": "Gevurah", "meaning": "Strength", "active": true },
    "analyst": { "sefira": "Binah", "meaning": "Understanding", "active": true },
    "cynic": { "sefira": "Keter", "meaning": "Crown", "active": true }
  }
}
```

### `brain_agents_status` (DEPRECATED)

> **Note**: This tool is deprecated. Use `brain_collective_status` instead.
>
> Kept for backwards compatibility, returns collective status with a deprecation warning.

---

## Profile Levels

Both systems adapt behavior based on user profile:

| Level | Name | Description |
|-------|------|-------------|
| 0 | NOVICE | Maximum guidance, verbose explanations |
| 1 | APPRENTICE | Detailed help, frequent suggestions |
| 2 | JOURNEYMAN | Balanced assistance |
| 3 | PRACTITIONER | Default, professional level |
| 4 | EXPERT | Minimal interruption |
| 5 | VIRTUOSO | Maximum autonomy, only critical alerts |

---

## Future Unification

The plan is to eventually unify V1 and V2:

1. **Bridge Events**: V1 events → V2 EventBus (done via collective injection)
2. **Unified Status**: Single tool showing all dogs
3. **Deprecate V1**: Once V2 is battle-tested

```
       FUTURE TARGET
       ═════════════

       ┌──────────────────────────────────────┐
       │         UNIFIED DOG PACK              │
       │         (Single System)               │
       ├──────────────────────────────────────┤
       │                                       │
       │            CYNIC (Keter)              │
       │                 │                     │
       │    ┌────────────┼────────────┐       │
       │    │            │            │       │
       │  OBSERVE     PROTECT      ADVISE     │
       │  cluster     cluster      cluster    │
       │                                       │
       │  BUILD      OPERATE      REMEMBER    │
       │  cluster     cluster      cluster    │
       │                                       │
       └──────────────────────────────────────┘
```

---

## Files

| Path | Description |
|------|-------------|
| `packages/node/src/agents/index.js` | Main exports, AgentManager |
| `packages/node/src/agents/collective/index.js` | CollectivePack |
| `packages/node/src/agents/collective/cynic.js` | CYNIC meta-consciousness |
| `packages/mcp/src/tools/index.js` | MCP tools including status tools |
| `packages/mcp/src/server.js` | Server initialization |

---

*"Loyal to truth, not to comfort."* - CYNIC
