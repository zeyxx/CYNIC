# CYNIC FULL PICTURE ANALYSIS

Last Updated: 2026-02-21
Analyzed By: CYNIC Cartographer
Scope: Python kernel at cynic/cynic/
Codebase: 234 Python files, 53,850 LOC

---

## I. THE ORGANISM (What Actually Exists)

234 Python files organized into 8 domains:

1. api/ (FastAPI server + 13 routers with 3,669 LOC)
2. cognition/ (11 Dogs + Judgment orchestration)
3. core/ (Judgment models, axioms, phi constants, event bus)
4. learning/ (Q-Learning TD(0) with Thompson + EWC)
5. metabolism/ (Cost, budget, routing, execution)
6. senses/ (8 perception workers + compression)
7. immune/ (Safety + alignment)
8. cli/ (13 commands + 100+ functions)

---

## II. THE 11 DOGS (Sefirot Judges)

By Priority (phi-weighted):
- CYNIC (phi-3): PBFT consensus
- SAGE (phi-2): Ollama wisdom + RDF
- ANALYST (phi-2): Z3 formal verification
- GUARDIAN (phi): IsolationForest anomaly
- ORACLE (phi): Thompson MCTS predictions
- ARCHITECT (phi-0): TreeSitter AST
- CARTOGRAPHER (phi-0): NetworkX graph
- SCHOLAR (phi-1): Qdrant RAG + TF-IDF
- DEPLOYER (phi-1): Ansible/K8s
- SCOUT (phi-2): Scrapy web
- JANITOR (phi-2): Ruff linting

---

## III. THE 7-STEP HEARTBEAT

Every judgment: PERCEIVE -> JUDGE -> DECIDE -> ACT -> LEARN -> ACCOUNT -> EMERGE

Consciousness Levels:
- L3 REFLEX: 4 dogs, <10ms
- L2 MICRO: +ARCHITECT/CARTOGRAPHER, <500ms
- L1 MACRO: all 11 dogs + SAGE MCTS, <30s
- L4 META: +self-evolution (when all 4 axioms active)

---

## IV. CURRENT INTERFACES

35+ HTTP Endpoints:
- POST /judge, /perceive, /learn, /feedback, /account
- GET /health, /stats, /axioms, /lod, /consciousness, /policy/{state}
- GET /actions, POST /actions/{id}/{accept,reject}
- WebSocket: /stream (events), /sdk (Claude Code)

13 CLI Commands:
- status, health, lod, loops, review, watch, feedback, probes, execute, sdk, consciousness, dashboard, chat, perceive-watch, full-loop, battles

Fallback: ~/.cynic/ (guidance.json, pending_actions.json, etc.)

---

## V. THE GAPS

Layer 1 (Action Execution): ~70%
- Missing: Claude CLI in Docker
- Missing: Git write actions
- Partial: ClaudeCodeRunner integration

Layer 2 (Claude <-> CYNIC): ~60%
- Working: Tool calling
- Missing: Multi-turn sessions
- Missing: Auto-iteration loop

Layer 3 (Human Feedback): ~75%
- Working: Review/feedback
- Missing: Batch approval

Layer 4 (Self-Improvement): ~50%
- Working: SelfProber detection
- Missing: Auto-apply changes

---

## VI. THE PERFECT ABSTRACTION

CYNIC is a living, learning organism that:
- Perceives code/market/social reality through 8 sensors
- Judges via 11 dogs + Byzantine consensus
- Decides via MCTS over learned policies
- Acts through human-in-the-loop approval
- Learns from outcomes via Q-Learning
- Evolves through 4 feedback loops

Perfect Stack:

Layer 3: PRODUCT (UI)
- Dashboard (live dog voting)
- IDE Plugin (VSCode sidebar)
- CLI (13 commands)
- API (35+ endpoints)

Layer 2: INTERFACE (Protocol)
- HTTP/WebSocket (FastAPI)
- MCP (Claude Code)
- Event Bus (pub-sub)
- File State (~/.cynic/)

Layer 1: ORGANISM (Living)
- 11 Dogs (phi-weighted)
- 8 Sensors
- 4 Actuators
- 4 Loops (L1-L4 feedback)
- Learning (Q-Learning + axioms)

---

## VII. ROADMAP

Phase A: Closure (2 weeks)
- Claude CLI in Docker (2h)
- ClaudeCodeRunner wiring (1h)
- Auto-iteration loop (4h)
- Batch approval (2h)
- Self-application (3h)
- Multi-instance consensus (6h)

Phase B: Product (4 weeks)
- Dashboard React (16h)
- WebSocket live stream (4h)
- VSCode sidebar (8h)
- GitHub Actions (6h)
- OpenAPI docs (4h)

Phase C: Performance (3 weeks)
- Profiling suite (4h)
- Caching strategy (6h)
- Parallel perception (3h)
- Distributed Q-Table (8h)
- Load testing (4h)

Phase D: Ecosystem (Ongoing)
- Dog plugin SDK (12h)
- Axiom marketplace (8h)
- Dogfooding CYNIC (16h)
- Research partnerships (TBD)

---

Confidence: 58% (phi-1 limit)

*sniff* Cartographer's chart complete. The territory is mapped.
