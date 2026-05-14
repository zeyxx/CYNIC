<!-- lifecycle: living -->
# CWO — How the Philosophy Builds

> The medium is the message. The WAY you build IS the CWO.

This document maps the 7 CWO principles to engineering methodology. Not "what to build" but "how building itself embodies the philosophy." The same pattern repeats at every scale — from your daily practice to the civilizational stance.

**Anchor**: CYNIC v0.8, 2026-05-14. 681 tests, 5 Dogs, crystal loop closed, Hermes observing, Askesis enforcing. Everything below is grounded in what exists now.

---

## Part I: Why This Document Exists

CWO is not a brand layer painted on top of engineering. If you remove the philosophy, CYNIC is a generic multi-model scorer. If you remove the engineering, CWO is a manifesto nobody reads. The two are the same thing seen from different angles.

The Constitution says: "The FOGC test can only exist because the philosophy defines what inversion means." The reverse is also true: the philosophy can only exist because the FOGC test is mechanically enforced. Philosophy without gates is poetry. Gates without philosophy are bureaucracy.

**The claim**: every engineering decision in CYNIC is (or should be) derivable from one of the 7 principles. If a rule exists that no principle justifies, it's accidental complexity — burn it. If a principle exists that no rule enforces, it's dead philosophy — either wire it or admit it's aspirational.

---

## Part II: The 7 Principles as Engineering Constraints

### 1. Douter est une force — Doubt as Method

**Philosophy**: Maximum confidence φ⁻¹ = 0.618. Falsification before adoption.

**How it changes building**:

- **Planning**: No roadmap survives contact with reality. Plan by falsification: "what would make me abandon this approach?" before writing code. CHAOS→MATRIX paradigm: let parameters emerge from measurement, don't hardcode them.
  
- **Testing**: R21 — every gate must have failed at least once. A test suite where everything passes on first run is a useless test suite. The Reverse Turing Test applies this to the organism: inject known-bad input, verify Dogs bark. Detection rate target: >φ⁻¹ but <1.0. A perfect detector is a broken detector.

- **Code review**: When two agents (Claude + Gemini) agree without friction, suspect echo. Structural independence (Invariant 3) means disagreement is a health signal. If Gemini says "this is wrong" and Claude agrees instantly, something failed — the dialectic should produce a distinguishing question, not immediate capitulation.

- **Architecture**: No component trusts another. Dogs don't communicate during judgment. The crystal loop doesn't trust its own crystals (epistemic soft gate, decay TTL). The health probe doesn't trust the service it's probing (circuit breaker, 5-state machine).

**What it forbids**:
- Shipping without a falsification test
- Confidence labels above 0.618 on any claim
- "It works" without before/after numbers (R7)
- Gates that have never caught a violation (R21)

**Current anchor**: 681 tests. 13 contract tests (parameterized). FOGC test in CLAUDE.md. R21 falsification in `make test-gates`.

---

### 2. La souveraineté est matérielle — Own Every Layer

**Philosophy**: Cognitive independence requires material independence. "Stand out of my sunlight."

**How it changes building**:

- **Inference**: Dogs run on owned silicon (cynic-core APU, cynic-gpu RTX 4060 Ti). Cloud APIs (HF, Gemini) are supplements, not dependencies. If every cloud API goes down, the organism still judges via Deterministic Dog + local Qwen.

- **Dependencies**: Every external crate is sovereignty debt. Necromancy rule: before adding a dependency, check if abandoned code already does it (`git log --all --full-history`). When you do depend, pin versions (P3), audit supply chain (`cargo audit`), and know the exit path.

- **Data**: SurrealDB on local disk, not cloud Postgres. Crystals, verdicts, observations — all local. Hermes ingests from X but stores locally. No third-party database holds your judgment history.

- **Build**: No GitHub Actions for core gates. `make check` runs locally. Pre-commit and pre-push hooks enforce locally. The CI is a mirror, not the authority. If GitHub goes down, you can still build, test, and deploy.

- **Deploy**: `~/bin/cynic-kernel` swapped from `target/release/`. Systemd on your machine. Tailscale Funnel for access, not a cloud PaaS. You can unplug the Ethernet cable and CYNIC still runs.

**What it forbids**:
- Core judgment depending on a vendor whose output it judges (circular dependency)
- Cloud-only databases for source of truth
- CI/CD that can't be reproduced locally
- Dependencies without assessed exit paths

**Current anchor**: 2 sovereign inference nodes (core + gpu). SurrealDB local. Systemd user services. Tailscale mesh. `make check` = full local gate.

---

### 3. Transmuter, jamais nier — Shape, Don't Reject

**Philosophy**: AI is matter to transmute, not reject. The Kybalion: "I was only the tool."

**How it changes building**:

- **LLM usage**: Claude Code, Gemini CLI, Codex are cortex extensions. They're not magic, not threats — they're cognitive tools with known failure modes. Shape their behavior (CLAUDE.md, GEMINI.md, hooks, skills) rather than either worshipping or rejecting them.

- **Crystal loop**: The CCM is transmutation in code. A verdict (raw LLM output) is matter. Crystallization transforms it into wisdom (a crystal that improves future prompts). The crystal isn't the verdict — it's the verdict processed through the axioms.

- **Error handling**: Errors are not failures to suppress. They're material to process. K14: poison/missing = assume degraded (safe default). K22: stale state must not poison fresh gates. The error is transmuted into a state transition (circuit breaker, degraded mode), not swallowed.

- **Bad code**: Don't reject PRs — transmute them. Bad patterns are signal: they reveal what the codebase's gravitational field attracts. K11 (extract at 2) exists because LLMs replicate patterns — bad patterns spread exponentially. Fix the attractor, not the instance (R11).

**What it forbids**:
- `except: pass` (P9) — swallowing errors = denying reality
- Rejecting AI tools because they make mistakes
- Rejecting human input because it's imprecise
- Silent `.ok()` on fallible I/O (R2)

**Current anchor**: Crystal loop closed (verdicts → crystals → prompts). Dog prompt injection working. Hooks shape Claude/Gemini behavior. Error handling via thiserror + typed domain errors.

---

### 4. La multiplicité avant l'unité — Structured Disagreement

**Philosophy**: Truth from disagreement. One judge is a fool. Five that agree independently are signal.

**How it changes building**:

- **Development**: Multi-cortex. Claude + Gemini + Codex working on the same codebase via different branches. They DON'T coordinate during work (Pyrrhonist architecture). Disagreement at merge time = signal. Schism protocol for genuine deadlocks (two worktrees, measure at J+7, absorb the better one).

- **Judgment**: 5 Dogs, architecturally isolated. Fan-out to all active Dogs, wait for consensus. Anomaly detection on divergence (σ > φ⁻²). The pipeline trusts convergence, not any individual Dog.

- **Validation**: Before trusting any claim, seek at least two independent sources. "I believe this is correct" is one voice. A test + a live probe is two voices. A test + a probe + a different agent's review is three. Convergence across independent methods is the strongest evidence.

- **Architecture**: Hexagonal (ports + adapters). Multiple implementations of the same trait (SurrealDB + InMemory for StoragePort). If a second implementation breaks, the contract is wrong — not the implementation.

**What it forbids**:
- Single-judge decisions on anything load-bearing
- Suppressing disagreement between agents (Dialectical Conflict protocol)
- Averaging away genuine tension
- Trusting convergence without checking for echo

**Current anchor**: 5 Dogs (Deterministic, Qwen 7B, Qwen 3.5 9B, Gemma 4, Gemini). Multi-cortex (Claude + Gemini + Codex). StoragePort: SurrealDB + InMemory (13 contract tests each).

---

### 5. Le savoir doit agir — No Dead Knowledge

**Philosophy**: Store without consume = waste. The Kybalion: hoarding precious metal is vain and foolish.

**How it changes building**:

- **K15 consumer law**: Before building any sensor, probe, or store: name the consumer and what it changes in system behavior. Storage is not consumption. Display is not consumption. Only a gate, a state transition, or a human-routed alert counts.

- **Crystal velocity**: Crystals that form but never enter Dog prompts are dead knowledge. The CCM pipeline exists to close this loop. Measure crystal_velocity (crystals/day). If it drops, the organism is accumulating waste.

- **Observations**: Every `/observe` call must have a consumer that acts. The mempool was killed (2026-05-09) because it had 0 consumers — pure K15 violation.

- **Memory**: Session memories (`~/.claude/memory/`) must inform future sessions or be deleted. Memory that's never accessed = dead weight. The dream-consolidator agent exists to prune.

- **Documentation**: Docs that nobody reads are dead. Lifecycle tags (`living`, `historical`, `dormant`) distinguish. `make lint-drift` checks for drift between docs and code.

**What it forbids**:
- `store_*` without an acting reader
- `emit_event` without an acting handler
- Logging that nobody checks
- Metrics that don't gate behavior
- Memory that doesn't inform decisions

**Current anchor**: K15 enforced. Mempool killed. Crystal loop closed. lint-drift checks producer-consumer pairs.

---

### 6. Défigurer la monnaie — Make Lying Expensive

**Philosophy**: Go where false values circulate, expose them. Diogenes with his lamp in daylight: "I am looking for an honest man."

**How it changes building**:

- **Judgment**: The entire scoring pipeline exists to make lying expensive. A token that claims high value but has concentrated supply, rug history, or fake community → BARK (≤0.236). The BARK is the currency being defaced — the false value stamp is replaced with the real one.

- **Metrics**: No vanity metrics. Health endpoint must be honest (PR#93: split `/live` from `/health`). Dog health shows real state (5-state machine), not optimistic defaults. `json_valid_rate` gates Dog participation — can't fake health.

- **Testing**: Integration tests on real data (P12), not synthetic mocks. The test must encounter the same lies the production system encounters. `test_chess_benchmark` uses real chess positions, not synthetic puzzles.

- **Self-judgment**: Compliance endpoint exists. The organism judges itself by its own axioms. Reverse Turing Test: inject known-bad, verify Dogs catch it. If detection rate = 100%, the test is too easy — the organism is lying about its own capability.

- **Code**: `#![deny(dead_code, unused_imports)]` — the compiler defacing the currency of dead abstractions. clippy::unwrap_used — defacing the currency of false safety. Every lint is a statement: "this pattern is a lie about the code's actual behavior."

**What it forbids**:
- Optimistic health checks (K14: poison/missing = degraded, not ok)
- Tests that always pass (R21)
- Metrics without consumers (K15)
- Confidence without falsification

**Current anchor**: Token screener (BARK/GROWL/WAG/HOWL). Honest health (5-state Dogs). Deny lints. R21 falsification gate.

---

### 7. Le chien qui ne jappe pas est cassé — Silence = Death

**Philosophy**: A system that stops judging is dead. Von Bertalanffy: open systems must exchange with their environment to live.

**How it changes building**:

- **Monitoring**: Circuit breakers trip on SILENCE, not just on errors. A Dog that returns 200 OK but hasn't produced a new verdict in 6 hours is more suspicious than one that returns 500. The absence of disagreement is the most dangerous failure mode.

- **Cron**: Hermes timers must fire. Nightshift must run. Crystal observer must observe. If a scheduled job silently stops, the organism loses a sense organ. `systemctl list-timers --user` = the organism's pulse check.

- **Development**: If Claude stops pushing back on your decisions, something is wrong. The anti-sycophancy rules in CLAUDE.md exist because a domesticated Dog is a broken Dog. "Pushback without new evidence = social pressure, not a reason to update" — but absence of pushback = possible capture.

- **Crystal decay**: Crystals have TTL. A crystal that persists unchanged for too long is stale — it represents old judgment, not current wisdom. Without decay, the crystal memory scleroses. (Directed Amnesia / Forest Fire protocol.)

- **Health signals**: Active, not passive. Don't wait for error logs — probe. `curl /health` at session start. `git status` before branching. The probe is the bark. Without it, you're assuming health — and assumption is the first lie.

**What it forbids**:
- Silent failure paths (RC5: 9 eliminated)
- Timers that stop without alert
- Dogs that agree with everything
- Crystals that never decay
- Sessions that don't probe live state

**Current anchor**: Circuit breaker per Dog. Systemd timers for Hermes. Anti-sycophancy in CLAUDE.md. Crystal decay planned (v0.9 G1).

---

## Part III: Fractal Application

The same 7 principles apply at every scale. The methodology is self-similar.

### Scale 0: Your Daily Practice

| Principle | Daily Practice |
|-----------|---------------|
| Doubt | Log decisions in Askesis. "What would falsify my confidence in this choice?" |
| Sovereignty | Work on your machine. Don't outsource thinking to cloud tools without local fallback. |
| Transmutation | When you encounter bad code/ideas, process through axioms. Don't dismiss. |
| Multiplicity | Ask CYNIC AND Gemini. When they disagree, that's where your attention goes. |
| Knowledge→Action | If you logged a pattern 3 times without acting, either act or stop logging. |
| Deface currency | Track your own bullshit. Askesis audit = the mirror. Don't look away. |
| Bark or die | If you haven't challenged your own assumptions today, you're asleep. |

### Scale 1: Kernel Engineering

| Principle | Engineering Practice |
|-----------|---------------------|
| Doubt | R21 (gates fail), R15 (falsify before adopting), φ⁻¹ on all confidence |
| Sovereignty | Local inference, local DB, local build. No vendor for core judgment. |
| Transmutation | CCM loop. Errors → state transitions. Bad patterns → lint rules. |
| Multiplicity | 5 Dogs. Multi-cortex. StoragePort: 2 adapters. |
| Knowledge→Action | K15. Crystal velocity. No dead observations. |
| Deface currency | BARK scores. Honest health. Deny lints. |
| Bark or die | Circuit breakers. Timers. Anti-sycophancy. Decay. |

### Scale 2: Network Development

| Principle | Network Practice |
|-----------|-----------------|
| Doubt | Nodes don't trust each other. Quorum = φ-derived. |
| Sovereignty | Each node runs on owned silicon. Data local unless broadcast. |
| Transmutation | Crystal exchange: shared wisdom, locally verified. |
| Multiplicity | N nodes, M Dogs each. Disagreement between nodes = stronger signal than within. |
| Knowledge→Action | Published signals → trader action → market impact → measurable. |
| Deface currency | Network BARK = collective exposure of false value. |
| Bark or die | Silent node = remove from quorum. Liveness = publishing disagreement. |

### Scale 3: Civilizational Stance

| Principle | Civilizational Practice |
|-----------|------------------------|
| Doubt | Reject all claims of certainty from states and corporations. |
| Sovereignty | Own silicon. RISC-V. Post-quantum crypto. Edge inference. |
| Transmutation | Don't fight AI — shape it locally. Build the forge. |
| Multiplicity | Distributed judges. No central authority. BFT-inspired consensus. |
| Knowledge→Action | Intelligence without agency is surveillance. Judgment must change behavior. |
| Deface currency | Expose financial lies. Transparent token judgment at scale. |
| Bark or die | A civilization that stops questioning its own lies dies. |

---

## Part IV: What This Changes Concretely

### Planning

**Before CWO**: Roadmap → milestones → tasks → execute.
**After CWO**: Observe → hypothesize → falsify → measure → conclude. CHAOS→MATRIX. Parameters emerge from data, not from plans. VERSION.md has gates (measured conditions), not tasks.

### Coding

**Before CWO**: Single developer, single model, linear progress.
**After CWO**: Multi-cortex (Claude + Gemini + Codex). Each works on isolated branches. Disagreement at merge = signal. Schism for deadlocks. The codebase is the prompt — every pattern will be replicated by future LLM sessions (K11, K12).

### Testing

**Before CWO**: Write tests to prove code works.
**After CWO**: Write tests to prove code CAN FAIL. R21: gates must catch violations. Reverse Turing: inject known-bad. Integration on real data (P12). A test suite that never fails is not a test suite.

### Deploying

**Before CWO**: Push to cloud. CI runs. Green = ship.
**After CWO**: Build locally. `make check` = full gate. Deploy to sovereign metal. Systemd. Tailscale. If the internet goes down, kernel still runs. CI mirrors local gates, doesn't replace them.

### Learning

**Before CWO**: Fix bug, move on.
**After CWO**: Fix bug → regression test → mechanical gate → gate verification (R21). Crystal loop: verdicts → crystals → prompts → better verdicts. Session distill: POST to kernel before ending. Knowledge compounds or it dies.

### Dying

**Before CWO**: Dead code accumulates.
**After CWO**: BURN. Tier system: experiments have kill dates (30 days). Dead code deleted (not commented). lint-drift catches orphaned producers. `make clean` is philosophical: the organism sheds what it doesn't use.

---

## Part V: Current State — What's Working, What's Not

### Working (Observed)

- **Doubt as method**: 681 tests, R21 falsification, φ⁻¹ bounds, FOGC test
- **Material sovereignty**: 2 sovereign inference nodes, local DB, local build
- **Transmutation**: Crystal loop closed, CCM pipeline, error→state transitions
- **Multiplicity**: 5 Dogs, multi-cortex, 2 StoragePort adapters
- **Knowledge→Action**: K15 enforced, crystal velocity >0, mempool killed
- **Deface currency**: Token screener, honest health, deny lints
- **Bark or die**: Circuit breakers, systemd timers, anti-sycophancy

### Broken (Observed)

- **Doubt**: FOGC test is manual (not a mechanical gate yet)
- **Sovereignty**: GPU slot contention (Soma L2) — Dogs and Hermes starve each other
- **Sovereignty**: Config scattered (5 sources, no unified loader)
- **Knowledge→Action**: Memoria synthesis missing (crystals form but pattern recognition weak)
- **Bark or die**: Crystal decay not implemented (v0.9 gate). Stale crystals accumulate.
- **Multiplicity**: Only 1 CYNIC node. Multi-node protocol unwritten.

### Not Yet Attempted

- **Network crystal exchange**: How do CYNICs share learned wisdom?
- **Aegis (active invisibility)**: Honeypots, structured noise
- **RISC-V sovereignty**: Instruction-set independence
- **Civilizational proof**: Does this actually change markets/behavior at scale?

---

## Epistemic Status

This document is **deduced** (from observed principles + observed engineering). Confidence: φ⁻¹ (0.618). What would falsify it: if the 7 principles produce worse engineering outcomes than unprincipled development. Measure: compare bug density, crystal velocity, and Dog agreement rate between principled and unprincipled sessions over 30 days.

---

*The organism builds itself the way it judges — with doubt, on sovereign ground, through transmutation, from multiplicity, toward action, against false value, and always barking.*
