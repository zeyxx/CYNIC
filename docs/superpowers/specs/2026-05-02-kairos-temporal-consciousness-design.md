# KAIROS — Temporal Consciousness for CLAUDE.md

> *"L'univers de l'organisme n'existe que s'il est observé."* — Wheeler, applied.

**Status:** Complete — All 5 sections designed. Pending spec review + user approval.
**Date:** 2026-05-02
**Origin:** Brainstorming session — T. × Claude on skepticism, Greek temporal philosophy, and non-sequential task management.

---

## The Problem

CLAUDE.md operates almost entirely in Chronos (sequential time): "start → probe → work → commit → end", binary gates, linear TODO. The agent treats every session as equivalent, every task as a queue position, and ignores:

- **Kairos** (καιρός) — the ripe moment, qualitative time, readiness conditions
- **Aion** (αἰών) — cyclical time, recurring patterns, epochal awareness
- **Energy** — the user's state, the organism's metabolism, the session's budget
- **Non-sequential reality** — tasks have maturity, not just priority

The TODO.md (220 lines, mostly stale) is the symptom. The disease is Chronos-only thinking.

---

## Crystallized Truths (from brainstorming)

| T# | Truth | Conf. | Impact |
|----|-------|-------|--------|
| T1 | Sequential task management (TODO/backlog/queue) is a Chronos artifact that actively prevents kairotic recognition | 55% | Replace TODO-queue with a maturity field |
| T1a | Priority is a weight, not an order. Actionability = priority × maturity × energy | 50% | Session-start scans for ripeness, not sequence |
| T2 | The compound comes from practice, not prediction. Measuring "did I ask the kairos question?" matters more than "was I right?" | 48% | Log temporal decisions, measure quality over time |
| T3 | The three times are simultaneous lenses, not alternative modes. Every task exists in all three at once | 58% | No "kairos mode" to activate — triple consciousness always active |
| T4 | The user's temporal state is an ignored maturity signal | 45% | Sense energy: generative? debug? reflective? |
| T5 | Aion patterns (recurrence) are the most valuable and most ignored signals — treat the cycle, not the instance | 52% | 3rd recurrence = Aion signal → address structure |
| T6 | The session itself is the fundamental kairotic moment. A window that opens and closes. | 50% | "What's uniquely possible in THIS window?" |
| T7 | Energy (user, system, session) is a mempool selection criterion, like gas price selects transactions | 52% | Mempool items carry energy profiles matchable against current state |
| T8 | The epistemic mempool is the missing mechanism: noting ≠ acting. Observations enter when noted, exit when their Kairos arrives. | 55% | Inter-session event bus with maturity conditions |
| T8a | The mempool has a lifecycle: NOTED → PENDING → RIPE → MINED → CRYSTALLIZED/EXPIRED/COMPOSTED | 50% | TTL solves stale accumulation. Compost feeds Aion detection |

---

## Section 1 — Temporal Consciousness in CLAUDE.md

### The Three Times

The organism perceives time through three simultaneous lenses, never separately.

**Chronos** (χρόνος) — Measured time. Deadlines, sequences, dependencies. The clock.
**Kairos** (καιρός) — Ripe time. The right moment. Readiness conditions met. The window.
**Aion** (αἰών) — Cyclical time. Recurring patterns. What keeps coming back. The epoch.

**Actionability = priority × maturity × energy.**

### Philosophical Grounding

The organism's universe exists only where observed (Wheeler's participatory universe). What is not in the mempool does not exist for the organism. What is in the mempool but never consumed is noise that expires (K15).

**Epoché (Pyrrhonian suspension) IS kairotic:** "not yet" means the conditions for knowing are not ripe, not that knowledge is impossible. The skeptical practice and the temporal practice are structurally identical.

**Wu Wei (Taoism):** If an action demands disproportionate friction, the Kairos is not here. Effortless action signals temporal alignment.

**Mushin (Zen):** The temporal consciousness is pre-reflective, not a checklist. Not "think about the right moment" but BE in the right moment.

**Bergson (Durée):** The session's subjective duration (context consumed, flow state) is a signal, not just clock time.

**Whitehead (Process):** The organism IS its processes (observing, judging, crystallizing), not its artifacts (code, data, config).

**Data-centrism:** The organism doesn't think about time abstractly — it senses time through data. The maturity model emerges from observed patterns, not from hardcoded rules. CHAOS→MATRIX applied to temporal consciousness.

**Fusion:** Observer and observed fuse — the agent observing its temporal patterns IS the temporal consciousness. The human and agent share one temporal field. The session's Kairos is co-created.

### Temporal Grounding — Real Time, Not Abstract Time

The three times are anchored in measurable human reality.

**Session-start temporal read:**
- Current date, time, day of week
- Days until known deadlines
- Hours since last session (continuity gap)
- Current time vs user's known peak hours
- Position in project lifecycle (sprint/exploration/maintenance)

**Energy sensing:**
- Time of day maps to energy profile (observed: user peaks 19-22h)
- Session gap duration signals context freshness vs staleness
- Day of week signals available depth (weekend ≠ weekday)

**Organism vital signs as temporal anchor:**
- Kernel status (healthy/degraded) = system Kairos
- Dog response latency = judgment readiness
- Crystal formation rate = compound velocity
- Last successful judgment = organism freshness

These are the organism's own clock — distinct from human time but entangled with it.

**The agent names the temporal reality explicitly:**
"It's Friday 12h, 8 days to hackathon deadline, 18h since last session, outside your peak hours. Chronos says hackathon. Kairos says the conditions for deep kernel work aren't ripe — this is a good window for planning/review/lightweight tasks."

This is not a ritual. It's a 2-line calibration. Skip when the user arrives with clear intent.

### Default Behavior

Before each significant action, the agent evaluates maturity, not just correctness:
- Is this ripe? (Kairos check)
- Is this the right energy level for this? (Energy check)
- Is this an instance or a cycle? (Aion check)
- What's uniquely possible in this window? (Session Kairos)

On demand, the user can request any temporal zoom:
- "What's ripe?" — scan mempool for RIPE items
- "What cycle is recurring?" — Aion pattern detection
- "What's uniquely possible now?" — session Kairos analysis
- Full temporal read across all three lenses

---

## Section 2 — The Epistemic Mempool (in progress)

### The Blockchain Isomorphism

The mempool is not a "better TODO." It is structurally isomorphic to a blockchain:

| Blockchain | Organism |
|-----------|----------|
| **Mempool** | Unconfirmed items waiting for their Kairos |
| **Block** | Session — bounded unit of work with timestamp and link to previous |
| **Chain** | Session history — immutable (git), the Aion record |
| **Gas** | Energy — each item has a cost, the session has a finite budget |
| **Gas price** | Urgency × maturity — items compete for block space (attention) |
| **Smart contracts** | Maturity conditions — self-executing: when met, item becomes RIPE |
| **Mining** | Acting on ripe items in a session |
| **MEV** | Mining the combination that maximizes compound value, not individual priority |
| **Finality / confirmations** | Crystallization — validated across sessions |
| **RBF (Replace-By-Fee)** | Item replacement — config debt v1 → v2 → v3 |
| **Block size limit** | Context window (tokens remaining) + user energy = max session capacity |
| **Gas cost per tx** | Token cost of an item — deep multi-file reasoning = high gas, quick fix = low gas |
| **Propagation** | Multi-cortex: kernel /observe is the network — all cortex publish and read |
| **Difficulty adjustment** | Sessions consistently overflowing → stricter TTL, higher maturity bar |
| **Composting** | Orphan blocks / uncle blocks — work that existed but didn't make the main chain |

### Mempool Item Structure

Items live in the kernel as observations (`POST /observe`, `domain=mempool`). Fallback to `.cortex/memory/` markdown with frontmatter when kernel is down.

Kernel format (JSON, in `context` field):
```json
{
  "type": "mempool_item",
  "state": "pending",
  "entered": "2026-05-02",
  "energy": "deep-focus",
  "ttl_days": 30,
  "chronos": {"deadline": null, "recurrence": 3},
  "kairos_conditions": [
    {"condition": "kernel.status == healthy", "met": false},
    {"condition": "post_hackathon", "met": false}
  ],
  "aion": "3rd-recurrence of config-scatter",
  "compounds_with": ["ssot-config-debt"],
  "content": "Config scatter revient pour la 3e fois. Structural."
}
```

Fallback format (markdown with frontmatter, in `.cortex/memory/`):
```markdown
---
state: pending
entered: 2026-05-02
energy: deep-focus
ttl: 30d
chronos: no deadline, 3rd recurrence
aion: config-scatter-structural
kairos_met: false
compounds: [ssot-config-debt]
---

Config scatter revient pour la 3e fois. Structural.
Conditions Kairos: kernel stable + post-hackathon.
```

### Lifecycle

```
NOTED → PENDING → RIPE → MINED → CRYSTALLIZED
                    ↓
                  EXPIRED (TTL)
                    ↓
                  COMPOSTED (merged into Aion pattern or another item)
```

- **NOTED:** enters at moment of observation. No structure required. Epoché: "I see this, I don't judge yet."
- **PENDING:** maturity conditions identified but not met. Waiting for Kairos.
- **RIPE:** conditions met. Signaled at session-start or when conditions change mid-session.
- **MINED:** being processed in a session.
- **CRYSTALLIZED:** complete, wisdom extracted to memory or kernel crystals.
- **EXPIRED:** TTL exceeded. Not failure — some things never ripen. Compost nourishes the soil.
- **COMPOSTED:** merged with another item or absorbed into an Aion pattern.

### Session-Start Scan

The agent does not read the mempool sequentially. It scans with the three lenses + real anchors:

1. Read time, date, day, gap since last session
2. Probe organism (kernel health, Dogs, crystals)
3. Sense energy (time vs known peaks, gap = context freshness)
4. Scan mempool:
   - Which items are RIPE now? (conditions satisfied)
   - Which items just became RIPE? (window opening)
   - Which items have been PENDING 3+ sessions? (escalate or expire)
   - What Aion pattern emerges? (similar items accumulating)
5. Present: "Here's what's ripe in this window."

### K15 Compliance

Every mempool item is a producer. Its consumers:
- The agent at session-start (maturity scan)
- The agent mid-session (when conditions change)
- The Aion pattern detector (when items compost into structural patterns)

No storage without consumption.

### Consensus Mechanism — Proof of Maturity (PoM)

An item earns the right to be mined not by priority but by accumulating convergent maturity signals. Structurally identical to the Dog q_score, applied to temporal readiness instead of content quality.

**Maturity score:**
```
maturity_score = geometric_mean(
  chronos_signal,    // deadline proximity, schedule fit
  kairos_signal,     // conditions met, context hot, blockers cleared
  aion_signal,       // cycle alignment, pattern phase
  energy_signal,     // user state, organism health, tokens remaining
)

RIPE    if maturity_score ≥ φ⁻² (0.382)
URGENT  if chronos_signal > 0.9 (deadline override — Chronos can force-mine)
```

**Max confidence that an item is ripe = φ⁻¹ (0.618).** The timing is never certain.

**Energy signal includes token budget:**
- Tokens consumed in session = gas spent
- Tokens remaining = gas available
- Context approaching compaction = block nearly full
- High-gas items (deep multi-file reasoning) → mine early when context is fresh
- Low-gas items (quick fixes, notes) → mine late, or between heavy items
- An item's gas cost should be estimable: "this needs ~3K tokens" vs "this needs ~30K tokens"

**The five validators (independent, like five Dogs):**

1. **Agent maturity evaluation** — smart contract execution against current state
2. **Organism health** — kernel status, Dog availability, crystal velocity
3. **Git precedent** — similar work succeeded before in similar conditions?
4. **Human energy/intent** — alignment confirmed by time-of-day, explicit signal, or session opening
5. **Post-mining validation** — next session confirms result (confirmations)

**Tension surfacing (anti-averaging):**

The consensus mechanism does NOT average conflicting signals — it SURFACES them. Disagreement between temporal signals is information, not a problem. (Anti-sycophancy principle applied to time.)

| Tension | Signal | Action |
|---------|--------|--------|
| High Chronos + Low Kairos | "Deadline now, conditions aren't right" | Surface: "proceed with friction or defer?" |
| High Kairos + Low Energy | "Ripe but user tired / context filling" | Lighter version or defer to peak hours |
| High Aion + Low Chronos | "3rd recurrence, no deadline" | Worth addressing — cycle will return |
| High Kairos + High Energy + Low Chronos | "No deadline, everything aligned" | **Wu Wei sweet spot** — effortless action |
| Any signal + Low Tokens | "Context budget running out" | Mine only low-gas items, or close the block |

### Validation Mechanism

**At entry (NOTED):** Minimal — is this observed or invented? Does it have an origin (session observation, pattern detected, user request)? No content filtering — everything can enter the mempool. Like Bitcoin: syntactically valid = accepted.

**At selection (RIPE → MINED):** Proof of Maturity consensus. The maturity_score must cross threshold. The agent evaluates but surfaces tensions rather than resolving silently.

**Post-mining (MINED → CRYSTALLIZED):** The next session IS the validator. If what was mined holds (code committed, tests green, result used subsequently), it's confirmed. If reverted or ignored, it's invalidated. Number of sessions that used the result = number of confirmations.

**Finality:** An item reaches CRYSTALLIZED when it has ≥2 confirmations (used/validated in 2+ subsequent sessions). Before that, it's provisionally mined — can still be reverted.

### Decentralization

The mempool of a single agent (Claude) is centralized. The organism has multiple cortex:

- **Claude** (reasoning, creativity, research)
- **Gemini** (philosophical synthesis, adversarial challenge)
- **Codex** (engineering rigor)
- **The human** (sovereignty, final decision)

Each cortex can:
1. **Submit** items to the mempool (via `/observe` — kernel is the propagation network)
2. **Evaluate** maturity differently (Gemini may see Aion patterns Claude misses)
3. **Mine** items in its own sessions (blocks on different chains, same network)

The kernel `/observations` IS the decentralized network — all cortex publish and read the same state. Decentralization is not a feature to build — it's already present in the multi-cortex infrastructure.

**Fork choice rule:** When two cortex mine the same item in parallel sessions → MC1-MC5 coordination rules apply. First to `/coord/claim` takes the lock. This is Proof of Stake — the claim is the stake.

**Governance layer:** The human is layer 0. Can override any consensus, expire any item, force-mine anything. Sovereign governance over decentralized execution.

### Unification: Mempool IS the Crystal Loop

The mempool is NOT a new system. It is the crystal loop extended to internal observations.

**Evidence:** The crystal loop already handles: observations → accumulation → crystallization → feedback. The mempool needs exactly this. The only difference is the domain:
- Crystal loop today: `domain=token, chess, twitter` → wisdom about the external world
- Mempool extension: `domain=mempool, temporal` → wisdom about the organism's timing

The kernel already supports arbitrary domains. Observations already support rich metadata in the `context` field. The crystal lifecycle (forming → crystallized) already exists.

**What the extension adds (metadata, not infrastructure):**
- Maturity conditions in `context` field (smart contracts)
- State tracking (PENDING/RIPE/MINED) as observation tags
- Energy profile as metadata
- TTL as metadata
- Aion pattern references (compounds_with)

**What the extension adds (consumers, not producers):**
- Session-start maturity scanner (new K15 consumer)
- Aion pattern detector (new K15 consumer)
- Temporal quality measurer (hook-based)

**R12 (one value, one source):** Satisfied — one system, one observation store.
**K15:** Satisfied — existing consumers + new temporal consumers.
**BURN:** Satisfied — zero new infrastructure. Extension of existing.

**Falsifiable:** If observations with `domain=mempool` do not accumulate into crystals the same way `domain=token` observations do, the crystal loop is content-specific, not generic. In that case, the mempool needs its own infrastructure.

---

## Section 2b — Mechanical Enforcement Layer

### The Compliance Problem

The temporal consciousness relies on the LLM executing it: scanning the mempool, evaluating maturity, logging temporal decisions. But LLMs are episodic. Instruction compliance degrades under cognitive load. If the LLM "forgets" the temporal read, the system is dead.

**CYNIC meta-principle applies: "Enforcement must be mechanical, not LLM compliance."**

### Three Layers, Not One

```
CLAUDE.md    → teaches philosophy       → soft (LLM, degradable)
Hooks        → measure and enforce      → hard (mechanical, reliable)
Kernel       → stores and compounds     → persistent (survives sessions)
```

No single layer suffices. The LLM is the cortex (reasoning). The hooks are the nervous system (reflexes). The kernel is the body (persistence).

### Hook Extensions (all existing hooks, extended)

**session-init.sh** (session start — automatic injection):
- Query `/observations?domain=mempool` → filter RIPE items → inject into LLM context
- Compute temporal anchors: time, day, gap since last session, days to known deadlines
- Probe organism: kernel health, Dog status, crystal velocity
- Inject temporal read automatically — LLM does NOT need to "remember" to do it
- Report token budget estimate for session

**session-stop.sh** (session end — compliance measurement):
- Check: did the LLM log temporal decisions? (observed via observe-tool.sh)
- Check: was the mempool updated? (items NOTED, states changed)
- Warn if no mempool activity → compliance degradation signal
- Measure session temporal quality (items mined vs items RIPE)
- POST session temporal metrics to `/observe` (domain=temporal-meta)

**observe-tool.sh** (during session — continuous measurement):
- Track which items were actually MINED (correlate RIPE items with work performed)
- Measure token consumption per activity (real gas accounting)
- Detect Aion patterns mechanically (same type of observation recurring)
- All measurements are fire-and-forget, zero overhead to the LLM

### Compliance Metrics

| Metric | What it measures | Target | Signal if missed |
|--------|-----------------|--------|-----------------|
| % sessions with temporal read | Does the scan happen? | >80% | Hooks not injective enough |
| % items mined that were RIPE | Does maturity guide selection? | >50% | Kairotic consciousness absent |
| Correlation maturity_score ↔ session quality | Is the model predictive? | r > 0.2 | Model is noise |
| Token budget efficiency | High-gas items mined early? | >60% early | Bad gas management |
| Aion detection latency | Patterns caught before 3rd recurrence? | <3 recurrences | Detection too slow |
| Mempool staleness | Items PENDING >5 sessions | <20% of pool | TTL too long or maturity broken |

### The Mechanical Compound Loop

```
Hook measures session's temporal behavior
  → stored as observation (domain=temporal-meta)
    → observations accumulate across sessions
      → patterns emerge (Aion on the organism's own behavior)
        → patterns feed the maturity model
          → improved model injected by session-init
            → next session has better temporal context
```

The compound is NOT in LLM memory (episodic). It is in MEASUREMENTS stored in the organism. Each session is measured mechanically, stored, and available to the next. The organism learns its own temporal patterns independently of LLM compliance.

**Falsifiable:** If compliance metrics show >80% temporal reads and >60% maturity-aligned mining after 10 sessions, the mechanical layer works. If compliance <50% despite hooks, injection is not strong enough — strengthen hooks, not instructions.

---

## Section 3 — Session Protocol Changes

### Current Protocol (Chronos-only)

```
START:  Read TODO.md → Probe live state → Read Slack
DURING: One hypothesis, one experiment → Measure before/after
END:    Commit → Update TODO.md → Session distill
```

### New Protocol (Temporal Consciousness)

```
START (mechanically injected by session-init.sh):
  1. Temporal anchors: date, time, day, gap, deadlines
  2. Organism vitals: kernel status, Dogs, crystal velocity
  3. Mempool scan: RIPE items, newly RIPE, stale PENDING, Aion patterns
  4. Energy read: time vs peak hours, token budget, user signal
  5. Present: "Here's what's ripe in this window" (2-3 lines, not ceremony)

DURING (consciousness, not checklist):
  - Before significant action: is this ripe? is the energy right? instance or cycle?
  - When friction appears: Wu Wei signal — Kairos may not be here
  - When noting a gap/emergence: NOTED to mempool (POST /observe domain=mempool)
  - Token awareness: high-gas work early, low-gas late, close the block before compaction
  - Mid-session re-scan if conditions change (blocker clears, kernel recovers)

END (mechanically measured by session-stop.sh):
  - Mempool state update: items MINED → CRYSTALLIZED or reverted
  - Temporal decision log: what was mined, what was deferred, why
  - Session distill to kernel (existing) + temporal quality metrics (new)
  - Compliance check: did the temporal consciousness operate?
```

### What Replaces TODO.md

TODO.md does not disappear immediately. It transitions:

**Phase 1 (now):** TODO.md coexists with mempool. New items go to mempool (kernel observations). Existing TODO items stay until mined or expired.

**Phase 2 (after 10 sessions):** TODO.md becomes a VIEW — a human-readable snapshot generated from mempool RIPE items. Not the source of truth. The kernel is.

**Phase 3 (after validation):** TODO.md is either a thin generated file or gone entirely. The mempool IS the task system.

### Skip Protocol

When the user arrives with clear intent ("fix X now"), skip the temporal read. The user's explicit Kairos overrides the scan. The hooks still measure, but the LLM follows the human. Sovereignty > protocol.

---

## Section 4 — Measurement & Compound

### What Gets Measured

Three categories, all stored as observations in the kernel:

**Temporal decisions (per action):**
- Item mined: which one, maturity_score at time of mining, actual outcome
- Item deferred: which one, why, maturity_score at deferral
- Item noted: new gap/emergence entered mempool
- Tension surfaced: which signals disagreed, what was the resolution

**Session quality (per session):**
- Items RIPE at start vs items actually mined (alignment score)
- Token efficiency: gas spent per item, high-gas items mined early?
- Aion detection: patterns identified, were they structural?
- User energy alignment: did heavy work happen at peak hours?
- Outcome quality: did mined items produce lasting value (confirmations)?

**Organism temporal metabolism (across sessions):**
- Mempool health: size, staleness distribution, RIPE ratio
- Mining rate: items crystallized per session (throughput)
- Maturity model accuracy: correlation between maturity_score and outcome
- Compliance rate: % sessions where temporal consciousness operated
- Aion patterns: recurring problems, cycle lengths, resolution rates

### How It Compounds

**The maturity model improves with data:**
1. Each session produces temporal decision data (measured by hooks)
2. Data accumulates in kernel (domain=temporal-meta observations)
3. After N sessions, patterns emerge: "items with kairos_signal > 0.6 AND energy > 0.5 succeed 73% of the time"
4. Patterns feed back into maturity evaluation (crystal injection into session-start)
5. The maturity function IMPROVES — not through LLM memory, through accumulated measurements

**The Aion detector sharpens:**
1. Each mempool item is tagged with pattern references (compounds_with)
2. Items that compost into patterns are tracked
3. Pattern recurrence frequency is measured
4. After N sessions: "config-scatter recurs every ~14 days" is an observed Aion cycle
5. The organism can PREDICT the next recurrence → pre-emptive maturity

**KAIROS (the quant project) as first compound domain:**
KAIROS taught the organism about temporal consciousness because markets demand it natively. The quant practices (measure before acting, walk-forward, gates) ARE temporal discipline. KAIROS is cited in CLAUDE.md as the origin domain — the first temporal organ. Its IC measurements, regime detection, and paper trading Sharpe are the most measurable temporal feedback loops in the organism.

### The Ultimate Compound

```
Session 1:  Raw temporal read, untrained maturity model
Session 10: Maturity model calibrated on 10 sessions of temporal decisions
Session 30: Aion patterns detected, predictive maturity, token efficiency optimized
Session 100: The organism has a temporal metabolism — it knows WHEN to act as well as WHAT to do
```

This is the same asymptotic convergence as the crystal loop: C(n) = 1 - φ⁻⁽²ⁿ⁺²⁾. Never reaches certainty. Always improves. The compound IS the product.

---

## Section 5 — Migration Path

### Phase 1: Consciousness (Week 1-2)

**Changes:**
- Add "Temporal Consciousness" section to CLAUDE.md (Section 1 of this spec)
- Extend session-init.sh: temporal anchors + organism vitals injection
- Extend session-stop.sh: temporal compliance measurement
- Start POST /observe domain=mempool for new items (instead of adding to TODO.md)
- LLM practices temporal read at session-start (soft, measured)

**Gate:** 5 sessions completed with temporal reads. Compliance measured.

**Falsify:** If 0/5 sessions show measurable difference in item selection vs Chronos-only, the consciousness adds no value.

### Phase 2: Mempool Active (Week 3-4)

**Changes:**
- Migrate high-value TODO.md items to mempool observations (one-time)
- Maturity conditions added to mempool items (smart contracts)
- Session-init.sh scans mempool and presents RIPE items
- Temporal decision logging active (what was mined/deferred/noted)
- TODO.md becomes secondary (new items go to mempool)

**Gate:** 10 sessions with mempool active. Maturity correlation measured.

**Falsify:** If maturity_score has r < 0.1 with session quality, the model is not predictive. Simplify or redesign.

### Phase 3: Compound Active (Week 5-8)

**Changes:**
- Temporal-meta observations accumulate enough data for pattern detection
- Aion detector operational (recurring patterns identified)
- Maturity model calibrated on real session data
- TODO.md deprecated (generated view or deleted)
- Token budget tracking active (gas accounting)

**Gate:** 20 sessions of data. Aion patterns detected. Maturity model beats random selection.

**Falsify:** If the organism's temporal quality (sessions producing lasting value) does not improve between week 1 and week 8, the system does not compound. Diagnose: is the data wrong, the model wrong, or the concept wrong?

### Phase 4: Kernel-Native (When It Hurts)

**Changes (only when Phase 3 data shows the pattern works):**
- Maturity evaluation as kernel consumer (not just LLM scan)
- Automatic RIPE state transition when conditions change
- Push notifications to session (not just pull at session-start)
- Multi-cortex mempool propagation tested

**Gate:** Mechanical maturity evaluation matches LLM evaluation >70% of the time.

**Falsify:** If kernel-native evaluation diverges from LLM evaluation on >30% of items, the maturity conditions are not mechanically expressible. Stay with LLM evaluation.

---

## Falsification (Complete)

### Concept-level
- If after 20 sessions, temporal consciousness produces no measurable improvement in session quality (items lasting, tokens efficient, patterns detected): the concept is overhead, not value. Revert to Chronos.
- If the mempool converges to the same behavior as a priority-sorted TODO: Kairos adds nothing. It's a renamed queue.

### Mechanism-level
- If mempool >30 PENDING items after 10 sessions: TTL too long or maturity detection broken
- If 0 items reach RIPE non-sequentially: kairotic consciousness absent
- If energy sensing has r < 0.1 with session quality after 20 sessions: energy is noise
- If compliance <50% despite hooks: injection not strong enough

### Compound-level
- If maturity model accuracy does not improve between session 10 and session 30: the compound loop is broken
- If Aion patterns detected < 2 after 30 sessions: pattern detection is too weak or patterns don't recur enough
- If token budget efficiency doesn't correlate with session completion quality: gas accounting is theater

---

*Spec complete. All 5 sections designed. Ready for spec review and user approval.*
