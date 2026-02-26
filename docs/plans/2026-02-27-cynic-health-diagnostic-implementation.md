# CYNIC Health Diagnostic Implementation Plan

> **For Claude:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Diagnose why CYNIC health = 21.5%, identify 5-7 root cause diseases, create measurement infrastructure, build test pipelines, and produce actionable roadmap to reach 80%+ health.

**Architecture:** Six-phase approach (5.5 weeks) starting with code inventory, moving to data audit, building measurement infrastructure, creating test pipelines, diagnosing diseases, and assessing completeness. **Critical principle:** Data pipelines are foundational — all phases uncover data gaps.

**Tech Stack:** Python (audit scripts), CSV/JSON (data formats), PostgreSQL/SurrealDB (query), pytest (testing), Pandas (data analysis), custom telemetry logging.

---

## PHASE 1: Inventory & Diagnosis (1 week, ~20 tasks)

### Task 1.1: Create audit workspace and structure

**Files:**
- Create: `docs/audit/` directory (audit reports)
- Create: `scripts/audit/` directory (audit scripts)

**Steps:**
1. Create directories:
   ```bash
   mkdir -p docs/audit
   mkdir -p scripts/audit
   touch docs/audit/.gitkeep scripts/audit/.gitkeep
   ```

2. Verify directories exist:
   ```bash
   ls -la docs/audit/ scripts/audit/
   ```
   Expected: Both directories created

3. Create README for audit:
   ```bash
   cat > docs/audit/README.md << 'EOF'
   # CYNIC Health Audit Reports

   This directory contains all diagnostic reports from the Health Diagnostic (2026-02-26).

   - Phase 1: Inventory & Diagnosis
   - Phase 2: Data Pipeline Audit
   - Phase 3: Measurement Infrastructure
   - Phase 4: Test Pipelines
   - Phase 5: Disease Diagnosis
   - Phase 6: Completeness Assessment
   EOF
   ```

4. Commit:
   ```bash
   git add docs/audit/ scripts/audit/
   git commit -m "chore(audit): create audit workspace structure"
   ```

---

### Task 1.2: Inventory all 11 learning loops

**Files:**
- Create: `docs/audit/learning-loops-inventory.md`

**Steps:**

1. Explore learning loops in codebase:
   ```bash
   find cynic -name "*learning*" -o -name "*loop*" | head -20
   grep -r "Q-Learning\|Thompson\|Meta-cognition\|DPO\|ECW\|Residual\|Axiom\|E-Score" cynic/ --include="*.py" | head -50
   ```

2. Create inventory template:
   ```bash
   cat > docs/audit/learning-loops-inventory.md << 'EOF'
   # Learning Loops Inventory

   ## 11 Learning Loops

   | # | Loop Name | Location | Mechanism | Status | Data Input | Data Output | Notes |
   |----|-----------|----------|-----------|--------|-----------|------------|-------|
   | 1 | Q-Learning | cynic/learning/q_learning.py | State-action value updates | ? | State obs | Q-table | |
   | 2 | Thompson Sampling | cynic/learning/thompson.py | Bayesian exploration | ? | Prior+obs | Actions | |
   | 3 | Meta-cognition | cynic/learning/meta_cognition.py | Learns how to learn | ? | ? | ? | |
   | 4 | DPO | cynic/learning/dpo.py | Direct preference opt | ? | ? | ? | |
   | 5 | ECW | cynic/learning/ecw.py | Elastic weight consolidation | ? | ? | ? | |
   | 6 | Residual Detection | cynic/learning/residual.py | Find unexplained var | ? | ? | ? | |
   | 7 | Axiom Discovery | cynic/learning/axiom_discovery.py | Learn new axioms | ? | ? | ? | |
   | 8 | E-Score Feedback | cynic/learning/escore_feedback.py | Reputation updates | ? | ? | ? | |
   | 9 | ? | ? | ? | ? | ? | ? | Need to find |
   | 10 | ? | ? | ? | ? | ? | ? | Need to find |
   | 11 | ? | ? | ? | ? | ? | ? | Need to find |

   ## Status Legend
   - ✅ WORKING: Code + tests + confirmed active
   - ⚠️ PARTIAL: Code exists, integration gaps or untested
   - ❌ BROKEN: Code exists, fails
   - ❓ UNKNOWN: Can't determine from code
   EOF
   ```

3. For each loop, read source code and fill in table:
   ```bash
   # Example: Read Q-Learning loop
   cat cynic/learning/q_learning.py | head -50
   ```

4. Update inventory based on findings (by hand, per loop)

5. Commit:
   ```bash
   git add docs/audit/learning-loops-inventory.md
   git commit -m "docs(audit): inventory all 11 learning loops"
   ```

---

### Task 1.3: Inventory all 11 Dogs

**Files:**
- Create: `docs/audit/dogs-inventory.md`

**Steps:**

1. Find Dogs in codebase:
   ```bash
   grep -r "class.*Dog\|ANALYST\|ARCHITECT\|GUARDIAN\|ORACLE" cynic/ --include="*.py" | head -30
   ```

2. Create Dogs inventory:
   ```bash
   cat > docs/audit/dogs-inventory.md << 'EOF'
   # 11 Dogs Inventory

   | # | Dog Name | Sefira | Technology | Location | Status | Role | Notes |
   |----|----------|--------|-----------|----------|--------|------|-------|
   | 1 | CYNIC | Keter | PBFT Consensus | cynic/judge/cynic_dog.py | ? | Meta-consciousness | |
   | 2 | SAGE | Chokmah | LLM+MCTS | cynic/judge/sage_dog.py | ? | Temporal reasoning | |
   | 3 | ANALYST | Binah | Z3 SMT | cynic/judge/analyst_dog.py | ? | Verification | |
   | 4 | SCHOLAR | Chesed | LLM+RAG | cynic/judge/scholar_dog.py | ? | Knowledge | |
   | 5 | GUARDIAN | Gevurah | IsolationForest | cynic/judge/guardian_dog.py | ? | Security | |
   | 6 | ORACLE | Tiferet | MCTS+Thompson | cynic/judge/oracle_dog.py | ? | Prediction | |
   | 7 | ARCHITECT | Netzach | LLM+TreeSitter | cynic/judge/architect_dog.py | ? | Code gen | |
   | 8 | DEPLOYER | Hod | Ansible+K8s | cynic/judge/deployer_dog.py | ? | Deployment | |
   | 9 | JANITOR | Yesod | Ruff | cynic/judge/janitor_dog.py | ? | Quality | |
   | 10 | SCOUT | Malkuth | Scrapy | cynic/judge/scout_dog.py | ? | Discovery | |
   | 11 | CARTOGRAPHER | Daat | NetworkX | cynic/judge/cartographer_dog.py | ? | Visualization | |

   ## Status: ✅/⚠️/❌/❓
   EOF
   ```

3. For each Dog, check if it exists and is implemented:
   ```bash
   ls -la cynic/judge/*dog*.py 2>/dev/null || echo "Dog files not found"
   ```

4. Fill in status for each Dog

5. Commit:
   ```bash
   git add docs/audit/dogs-inventory.md
   git commit -m "docs(audit): inventory all 11 Dogs"
   ```

---

### Task 1.4: Inventory 5 Axioms + Consciousness Levels

**Files:**
- Create: `docs/audit/axioms-and-consciousness-inventory.md`

**Steps:**

1. Create combined inventory:
   ```bash
   cat > docs/audit/axioms-and-consciousness-inventory.md << 'EOF'
   # Axioms & Consciousness Inventory

   ## 5 Core Axioms

   | # | Axiom | Principle | Element | Location | Status | Notes |
   |----|-------|-----------|---------|----------|--------|-------|
   | 1 | FIDELITY | Loyal to truth, not comfort | Water | cynic/core/axioms.py | ? | |
   | 2 | PHI | All ratios from 1.618... | Earth | cynic/core/axioms.py | ? | |
   | 3 | VERIFY | Don't trust, verify | Metal | cynic/core/axioms.py | ? | |
   | 4 | CULTURE | Culture is a moat | Wood | cynic/core/axioms.py | ? | |
   | 5 | BURN | Don't extract, burn | Fire | cynic/core/axioms.py | ? | |

   ## 4 Consciousness Levels

   | Level | Name | Latency | Complexity | Location | Status | Notes |
   |-------|------|---------|-----------|----------|--------|-------|
   | L3 | REFLEX | <10ms | Non-LLM Dogs only | cynic/cognition/consciousness.py | ? | Fast path |
   | L2 | MICRO | ~500ms | Fast LLM | cynic/cognition/consciousness.py | ? | Local inference |
   | L1 | MACRO | ~2.85s | Full reasoning | cynic/cognition/consciousness.py | ? | Claude/large Ollama |
   | L4 | META | Daily | Meta-learning | cynic/cognition/consciousness.py | ? | Learning loop |

   ## Event Bus Architecture

   | Bus | Purpose | Location | Status |
   |-----|---------|----------|--------|
   | Core | Judgment signals | cynic/cognition/event_bus.py | ? |
   | Metabolism | Health/memory | cynic/metabolism/event_bus.py | ? |
   | SDK | Claude Code integration | cynic/api/sdk_bus.py | ? |
   EOF
   ```

2. Check axioms file:
   ```bash
   grep -r "FIDELITY\|PHI\|VERIFY\|CULTURE\|BURN" cynic/core/ --include="*.py" | head -20
   ```

3. Check consciousness levels:
   ```bash
   grep -r "REFLEX\|MICRO\|MACRO\|META\|consciousness" cynic/cognition/ --include="*.py" | head -20
   ```

4. Fill in status for each axiom and level

5. Commit:
   ```bash
   git add docs/audit/axioms-and-consciousness-inventory.md
   git commit -m "docs(audit): inventory axioms and consciousness levels"
   ```

---

### Task 1.5: Check Event Bus Architecture

**Files:**
- Create: `docs/audit/event-bus-architecture.md`

**Steps:**

1. Find event bus files:
   ```bash
   find cynic -name "*event*bus*" -o -name "*event*" | grep -E "\.py$"
   ```

2. Document event bus structure:
   ```bash
   cat > docs/audit/event-bus-architecture.md << 'EOF'
   # Event Bus Architecture

   ## 3 Event Buses

   CYNIC uses 3 event buses to coordinate components:

   ### Bus 1: Core Cognition Bus
   - Location: `cynic/cognition/event_bus.py`
   - Purpose: Judgment creation, decision making, learning signals
   - Status: ?

   ### Bus 2: Metabolism Bus
   - Location: `cynic/metabolism/event_bus.py`
   - Purpose: Health checks, memory management, cleanup
   - Status: ?

   ### Bus 3: SDK Integration Bus
   - Location: `cynic/api/sdk_bus.py`
   - Purpose: Claude Code SDK integration, tool judging
   - Status: ?

   ## Event Names (from NOMENCLATURE.md)

   - JUDGMENT_CREATED
   - DECISION_MADE
   - ACT_COMPLETED
   - LEARNING_SIGNAL
   - RESIDUAL_DETECTED
   - AXIOM_ACTIVATED
   - EMERGENCE_DETECTED
   - TRANSCENDENCE

   ## Genealogy Tracking

   Events track lineage: where did this judgment come from?
   Status: ?
   EOF
   ```

3. Read event bus code:
   ```bash
   head -50 cynic/cognition/event_bus.py 2>/dev/null || echo "File not found"
   ```

4. Update with findings

5. Commit:
   ```bash
   git add docs/audit/event-bus-architecture.md
   git commit -m "docs(audit): document event bus architecture"
   ```

---

### Task 1.6: Create Health Inventory CSV

**Files:**
- Create: `docs/audit/health-inventory.csv`

**Steps:**

1. Create CSV combining all inventories:
   ```bash
   cat > docs/audit/health-inventory.csv << 'EOF'
   component_type,component_name,location,status,confidence,blocker,notes
   learning_loop,Q-Learning,cynic/learning/q_learning.py,UNKNOWN,LOW,data_pipeline,Need real training data
   learning_loop,Thompson Sampling,cynic/learning/thompson.py,UNKNOWN,LOW,trigger_logic,Need feedback signal
   dog,ANALYST,cynic/judge/analyst_dog.py,UNKNOWN,LOW,training_data,SMT solver integration untested
   dog,ARCHITECT,cynic/judge/architect_dog.py,UNKNOWN,LOW,training_data,TreeSitter integration needed
   axiom,FIDELITY,cynic/core/axioms.py,UNKNOWN,MEDIUM,definition,Need to measure
   consciousness_level,REFLEX,cynic/cognition/consciousness.py,UNKNOWN,MEDIUM,implementation,Need latency test
   event_bus,Core Bus,cynic/cognition/event_bus.py,UNKNOWN,LOW,genealogy,Need to verify tracking
   storage,PostgreSQL,cynic/storage/,UNKNOWN,LOW,schema,Need to document schema
   storage,SurrealDB,cynic/storage/,UNKNOWN,LOW,integration,Query performance unknown
   EOF
   ```

2. Verify CSV was created:
   ```bash
   head docs/audit/health-inventory.csv
   wc -l docs/audit/health-inventory.csv
   ```

3. Commit:
   ```bash
   git add docs/audit/health-inventory.csv
   git commit -m "docs(audit): create health inventory CSV"
   ```

---

### Task 1.7: Create Integration Graph

**Files:**
- Create: `docs/audit/integration-graph.md`

**Steps:**

1. Create integration graph showing dependencies:
   ```bash
   cat > docs/audit/integration-graph.md << 'EOF'
   # CYNIC Integration Graph

   ## Dependency Flow

   ```
   User Input (judgment request)
         ↓
   Judge Orchestrator
         ↓
   11 Dogs (parallel voting)
         ├─ ANALYST (Z3 SMT)
         ├─ ARCHITECT (TreeSitter)
         ├─ GUARDIAN (IsolationForest)
         ├─ ORACLE (MCTS)
         ├─ SAGE (LLM reasoning)
         ├─ SCHOLAR (RAG)
         ├─ JANITOR (Code quality)
         ├─ DEPLOYER (Deployment)
         ├─ SCOUT (Discovery)
         ├─ CARTOGRAPHER (Graph)
         └─ CYNIC (Meta-consensus)
         ↓
   PBFT Consensus
         ↓
   Verdict (HOWL/WAG/GROWL/BARK)
         ↓
   Event Bus → Learning Loops
         ├─ Q-Learning (state update)
         ├─ Thompson (prior update)
         ├─ Meta-cognition (learn how to learn)
         ├─ E-Score (reputation)
         └─ Residual Detection (find gaps)
         ↓
   Storage (PostgreSQL/SurrealDB)
         ↓
   Consciousness levels activate:
         REFLEX (fast) → MICRO → MACRO → META (learning)
   ```

   ## Integration Status

   | Connection | Status | Evidence | Issue |
   |-----------|--------|----------|-------|
   | Judge → Dogs | ? | Need to test | ? |
   | Dogs → Consensus | ? | Need to test | ? |
   | Verdict → EventBus | ? | Need to test | ? |
   | EventBus → Learning | ? | Need to test | ? |
   | Learning → Storage | ? | Need to test | ? |

   ## Missing Connections?

   - Cross-Reality Transfer: CODE → MARKET?
   - Axiom Activation: Does learning loop trigger axiom discovery?
   - Consciousness Transition: Does reflex → micro automatically?
   EOF
   ```

2. Verify file:
   ```bash
   head -30 docs/audit/integration-graph.md
   ```

3. Commit:
   ```bash
   git add docs/audit/integration-graph.md
   git commit -m "docs(audit): create integration dependency graph"
   ```

---

### Task 1.8: Document Current "Realities"

**Files:**
- Create: `docs/audit/realities-inventory.md`

**Steps:**

1. From consciousness.json, identify all realities:
   ```bash
   grep -o '"by_reality":\{[^}]*' consciousness.json | head -5
   ```

2. Create realities inventory:
   ```bash
   cat > docs/audit/realities-inventory.md << 'EOF'
   # CYNIC Realities Inventory

   ## Current Realities (from consciousness.json)

   | Reality | Judgment Count | Dogs Active | Data Source | Status |
   |---------|---------------|-------------|-----------|--------|
   | CYNIC | ? | 11 | Self-analysis | ACTIVE |
   | CODE | ? | ? | Code snippets | ACTIVE |
   | MARKET | ? | ? | Token data | ACTIVE |
   | SOLANA | ? | ? | Blockchain | ACTIVE |
   | GOVERNANCE | ? | ? | Proposals | ? |

   ## Total Judgments by Reality

   From consciousness.json:
   ```json
   "by_reality": {
     "CYNIC": 2,
     "MARKET": 1,
     "CODE": 2,
     "SOLANA": 1
   }
   ```

   Total: 6 active realities (data from consciousness.json)

   ## Missing Realities?

   From SPEC: Should we have more? (GOVERNANCE, SECURITY_AUDIT, TRADING, etc.)
   EOF
   ```

3. Commit:
   ```bash
   git add docs/audit/realities-inventory.md
   git commit -m "docs(audit): inventory current realities"
   ```

---

### Task 1.9: Create Integration Status Report

**Files:**
- Create: `docs/audit/phase-1-integration-status.md`

**Steps:**

1. Create integration status summary:
   ```bash
   cat > docs/audit/phase-1-integration-status.md << 'EOF'
   # Phase 1: Integration Status Report

   ## Summary

   CYNIC's architecture is well-designed but actual integration status is UNKNOWN.

   ## What We Know (Exists)

   ✅ Code exists for:
   - 11 Dogs (classes defined)
   - 5 Axioms (constants defined)
   - 4 Consciousness levels (mentioned in nomenclature)
   - 11 Learning loops (file structure present)
   - Event bus architecture (3 buses mentioned)
   - Storage layer (PostgreSQL/SurrealDB configured)

   ## What We Don't Know

   ❓ Actual status:
   - Are Dogs actually voting together?
   - Do learning loops actually trigger?
   - Does feedback signal reach Q-Learning?
   - Are axioms actually activating?
   - Is consciousness level switching working?
   - Is E-Score being updated?

   ## Confidence Levels

   - Architecture design: HIGH (well-documented)
   - Component implementation: MEDIUM (code exists)
   - Integration testing: LOW (no proof)
   - Working end-to-end: UNKNOWN (need testing)

   ## Next Steps

   Phase 2: Data Pipeline Audit will reveal if integration works.
   Phase 3: Measurement Infrastructure will provide proof.
   Phase 4: Test Pipelines will confirm actual functionality.
   EOF
   ```

2. Commit:
   ```bash
   git add docs/audit/phase-1-integration-status.md
   git commit -m "docs(audit): phase 1 integration status report"
   ```

---

### Task 1.10: Commit Phase 1 work

**Steps:**

1. Verify all Phase 1 files created:
   ```bash
   ls -la docs/audit/*.md docs/audit/*.csv
   ```

2. Final commit:
   ```bash
   git add docs/audit/
   git commit -m "docs(audit): complete phase 1 inventory & diagnosis"
   ```

---

## PHASE 2: Data Pipeline Audit (1.5 weeks, ~20 tasks)

### Task 2.1: Create data pipeline audit script

**Files:**
- Create: `scripts/audit/audit_data_pipelines.py`

**Steps:**

1. Create Python script to analyze data flows:
   ```bash
   cat > scripts/audit/audit_data_pipelines.py << 'EOF'
   #!/usr/bin/env python3
   """
   Audit CYNIC data pipelines.

   Maps:
   - Data sources (where input comes from)
   - Data schemas (how data is structured)
   - Data flows (source → storage → processing → output)
   - Data gaps (missing pipelines)
   """

   import json
   import sys
   from pathlib import Path

   # Load consciousness.json to understand current data
   CONSCIOUSNESS_FILE = Path.home() / ".cynic" / "consciousness.json"

   def audit_data_sources():
       """Inventory data sources for each reality."""
       sources = {
           "CODE": {
               "input_type": "Code snippet",
               "sources": ["User submission", "Git repo", "CI/CD logs"],
               "status": "UNKNOWN"
           },
           "MARKET": {
               "input_type": "Token data",
               "sources": ["Price feed", "Volume data", "Sentiment API"],
               "status": "UNKNOWN"
           },
           "GOVERNANCE": {
               "input_type": "Proposal text",
               "sources": ["DAO snapshot", "Forum posts", "Discord"],
               "status": "UNKNOWN"
           },
           "SOLANA": {
               "input_type": "Blockchain data",
               "sources": ["RPC endpoint", "Token program", "Account state"],
               "status": "UNKNOWN"
           }
       }
       return sources

   def audit_data_schemas():
       """Check data schemas in storage."""
       schemas = {
           "judgments": {
               "fields": ["judgment_id", "state_key", "q_score", "verdict", "dog_votes", "confidence", "timestamp"],
               "status": "UNKNOWN"
           },
           "decisions": {
               "fields": ["decision_id", "judgment_id", "decision", "timestamp"],
               "status": "UNKNOWN"
           },
           "embeddings": {
               "fields": ["cell_id", "vector", "metadata"],
               "status": "UNKNOWN"
           }
       }
       return schemas

   def audit_learning_signals():
       """Check learning signal pipelines."""
       signals = {
           "user_feedback": {
               "source": "User rates verdict",
               "format": "UNKNOWN",
               "destination": "Q-Learning loop",
               "status": "UNKNOWN"
           },
           "outcome_signal": {
               "source": "Post-judgment verification",
               "format": "UNKNOWN",
               "destination": "E-Score update",
               "status": "UNKNOWN"
           },
           "residual_signal": {
               "source": "Unexplained variance",
               "format": "UNKNOWN",
               "destination": "Axiom discovery",
               "status": "UNKNOWN"
           }
       }
       return signals

   if __name__ == "__main__":
       print("CYNIC Data Pipeline Audit")
       print("=" * 50)

       print("\n1. Data Sources:")
       sources = audit_data_sources()
       for reality, source in sources.items():
           print(f"  {reality}: {source['sources']}")

       print("\n2. Data Schemas:")
       schemas = audit_data_schemas()
       for table, schema in schemas.items():
           print(f"  {table}: {schema['fields']}")

       print("\n3. Learning Signals:")
       signals = audit_learning_signals()
       for signal, config in signals.items():
           print(f"  {signal}: {config['source']} → {config['destination']}")

       print("\nAll pipelines marked as UNKNOWN — need investigation")
       print("See: docs/audit/data-pipeline-map.md for details")
   EOF

   chmod +x scripts/audit/audit_data_pipelines.py
   ```

2. Run the script to see current status:
   ```bash
   python scripts/audit/audit_data_pipelines.py
   ```
   Expected: Shows UNKNOWN status for all pipelines

3. Commit:
   ```bash
   git add scripts/audit/audit_data_pipelines.py
   git commit -m "feat(audit): create data pipeline audit script"
   ```

---

### Task 2.2: Audit CODE reality data pipeline

**Files:**
- Create: `docs/audit/pipeline-code-reality.md`

**Steps:**

1. Trace CODE data flow through codebase:
   ```bash
   grep -r "CODE.*reality\|code.*judge\|code.*analyze" cynic/ --include="*.py" | head -20
   ```

2. Document CODE pipeline:
   ```bash
   cat > docs/audit/pipeline-code-reality.md << 'EOF'
   # CODE Reality Data Pipeline

   ## Pipeline: CODE Review Judgment

   ```
   INPUT: Code snippet
       ↓
   SOURCE: User submits via [API/Discord/Web]
       ↓
   STORAGE: Stored in [table: judgments, field: code_input]
       ↓
   PROCESSING:
       - ANALYST: Z3 SMT analysis
       - ARCHITECT: TreeSitter patterns
       - JANITOR: Ruff code quality
       - GUARDIAN: Security scan
       ↓
   OUTPUT: Verdict (HOWL/WAG/GROWL/BARK) + Q-Score
       ↓
   LEARNING SIGNAL: User feedback (did verdict help?)
       ↓
   STORAGE: Update Q-table with signal
   ```

   ## Data Schema (ASSUMED)

   ```sql
   -- Assumed judgment table
   CREATE TABLE judgments (
       judgment_id UUID,
       state_key TEXT,           -- "CODE:JUDGE:PRESENT:1"
       code_input TEXT,          -- The code snippet
       q_score FLOAT,            -- Quality score [0, 100]
       verdict TEXT,             -- HOWL/WAG/GROWL/BARK
       dog_votes JSON,           -- {ANALYST: 85, ARCHITECT: 72, ...}
       confidence FLOAT,         -- φ-bounded [0, 0.618]
       timestamp TIMESTAMP
   );
   ```

   ## Status: UNKNOWN

   **Questions to answer:**
   - [ ] Where does user CODE input come from? (API? Discord?)
   - [ ] How is code stored? (Raw text? AST?)
   - [ ] Which Dogs actually analyze CODE?
   - [ ] How is verdict computed from dog votes?
   - [ ] Where does feedback signal come from?
   - [ ] How is feedback connected to Q-Learning?

   ## Data Flow: UNVERIFIED

   - ❓ Input pipeline working?
   - ❓ Dog voting integrated?
   - ❓ Verdict stored correctly?
   - ❓ Learning signal reaching Q-Learning?
   EOF
   ```

3. Commit:
   ```bash
   git add docs/audit/pipeline-code-reality.md
   git commit -m "docs(audit): document CODE reality pipeline (unverified)"
   ```

---

### Task 2.3: Audit MARKET reality data pipeline

**Files:**
- Create: `docs/audit/pipeline-market-reality.md`

**Steps:**

1. Look for MARKET data handling:
   ```bash
   grep -r "MARKET\|token\|price\|volume" cynic/ --include="*.py" | head -20
   ```

2. Document MARKET pipeline:
   ```bash
   cat > docs/audit/pipeline-market-reality.md << 'EOF'
   # MARKET Reality Data Pipeline

   ## Pipeline: Token Analysis Judgment

   ```
   INPUT: Token data (price, volume, sentiment, contract)
       ↓
   SOURCES: [Price feed?, Volume API?, Sentiment scraper?]
       ↓
   STORAGE: Stored in [table: judgments, field: market_input]
       ↓
   PROCESSING:
       - ORACLE: Bayesian prediction
       - SAGE: Historical pattern matching
       - SCHOLAR: Community signal analysis
       - ANALYST: Smart contract review
       ↓
   OUTPUT: Verdict (HOWL/WAG/GROWL/BARK) + Investment signal
       ↓
   LEARNING SIGNAL: Price movement (did prediction match reality?)
       ↓
   STORAGE: Update Q-table with signal
   ```

   ## Data Schema (ASSUMED)

   ```sql
   -- Assumed market judgment
   CREATE TABLE judgments (
       judgment_id UUID,
       state_key TEXT,              -- "MARKET:JUDGE:PRESENT:1"
       market_input JSON,           -- {price, volume, sentiment, ...}
       q_score FLOAT,
       verdict TEXT,
       dog_votes JSON,
       confidence FLOAT,
       timestamp TIMESTAMP
   );
   ```

   ## Status: UNKNOWN

   **Critical questions:**
   - [ ] Where does price/volume data come from? (No data source found!)
   - [ ] Is there a price feed integration?
   - [ ] Is sentiment analysis implemented?
   - [ ] How accurate are market predictions?
   - [ ] Does learning loop use price data as ground truth?

   ## SUSPECTED GAP: Missing Data Source

   MARKET reality seems to be active (1 judgment in consciousness.json)
   but data source is UNKNOWN. Need to find where market data comes from.
   EOF
   ```

3. Commit:
   ```bash
   git add docs/audit/pipeline-market-reality.md
   git commit -m "docs(audit): document MARKET reality pipeline (data source unknown!)"
   ```

---

### Task 2.4: Create data gaps summary

**Files:**
- Create: `docs/audit/data-gaps-critical.md`

**Steps:**

1. Create critical gaps document:
   ```bash
   cat > docs/audit/data-gaps-critical.md << 'EOF'
   # CRITICAL DATA GAPS

   ## Blocking Issues

   ### 1. NO STANDARDIZED DATA INGESTION (CRITICAL)

   Status: NO data ingestion framework found

   - [ ] Where does judgment input come from?
   - [ ] Is there a unified input schema?
   - [ ] How are different realities' data validated?
   - [ ] Is there input sanitization?

   **Impact:** Can't run test pipelines without this

   ### 2. NO FEEDBACK SIGNAL PIPELINE (CRITICAL)

   Status: User feedback mechanism NOT FOUND

   - [ ] How does user provide feedback on verdict?
   - [ ] Where is feedback stored?
   - [ ] How does feedback reach Q-Learning loop?
   - [ ] Is there feedback validation?

   **Impact:** Learning loops can't work without feedback

   ### 3. NO DATA SOURCE INTEGRATIONS (CRITICAL)

   Status: External data sources not integrated

   - [ ] No price feed for MARKET reality
   - [ ] No code analysis tools for CODE reality
   - [ ] No blockchain data for SOLANA reality
   - [ ] No DAO data for GOVERNANCE reality

   **Impact:** Realities are isolated, can't judge real data

   ### 4. UNKNOWN DATA SCHEMAS (HIGH)

   Status: Storage schema not documented

   - [ ] What fields does judgment table have?
   - [ ] What format for dog votes?
   - [ ] What format for learning signals?
   - [ ] Are schemas consistent across realities?

   **Impact:** Can't build test harness without knowing schema

   ### 5. NO DATA VALIDATION (HIGH)

   Status: Input validation logic not found

   - [ ] Are inputs sanitized?
   - [ ] Are null values handled?
   - [ ] Are size limits enforced?
   - [ ] Are schema violations logged?

   **Impact:** Bad data can corrupt learning loops

   ### 6. NO DATA QUALITY METRICS (HIGH)

   Status: No way to measure data quality

   - [ ] No error rate tracking
   - [ ] No missing data tracking
   - [ ] No stale data alerts
   - [ ] No schema violation logs

   **Impact:** Can't diagnose if low accuracy is data or logic issue

   ## Recommendations

   **FTM Priority: Build data ingestion framework**

   This blocks everything else:
   1. Data ingestion standardization (1 week)
   2. Input validation (3 days)
   3. Schema documentation (3 days)
   4. Feedback signal pipeline (1 week)
   5. Data quality monitoring (1 week)

   Then can proceed to:
   - Phase 3: Measurement infrastructure
   - Phase 4: Test pipelines
   - Phase 5: Disease diagnosis
   EOF
   ```

2. Commit:
   ```bash
   git add docs/audit/data-gaps-critical.md
   git commit -m "docs(audit): identify critical data pipeline gaps"
   ```

---

### Task 2.5: Create data pipeline map (high-level)

**Files:**
- Create: `docs/audit/data-pipeline-map.md`

**Steps:**

1. Create overall data pipeline map:
   ```bash
   cat > docs/audit/data-pipeline-map.md << 'EOF'
   # CYNIC Data Pipeline Map

   ## Overall Architecture

   ```
   ┌─────────────────────────────────────────────────────────────┐
   │                    DATA PIPELINE MAP                         │
   └─────────────────────────────────────────────────────────────┘

   4 REALITIES × 4-STAGE PIPELINE:

   1. INGESTION (Data Input)
      ├─ CODE: [UNKNOWN SOURCE]
      ├─ MARKET: [UNKNOWN SOURCE]
      ├─ GOVERNANCE: [UNKNOWN SOURCE]
      └─ SOLANA: [UNKNOWN SOURCE]
           ↓ (data_validation ?)

   2. STORAGE (Persistence)
      └─ PostgreSQL / SurrealDB
           ├─ Table: judgments (schema UNKNOWN)
           ├─ Table: decisions (schema UNKNOWN)
           └─ Table: embeddings (schema UNKNOWN)
           ↓ (fetch_for_processing ?)

   3. PROCESSING (Judge + Learn)
      ├─ Judge Orchestrator
      │  └─ 11 Dogs vote (integration status: UNKNOWN)
      ├─ Q-Learning loop (trigger: UNKNOWN)
      ├─ Thompson Sampling (trigger: UNKNOWN)
      ├─ E-Score update (trigger: UNKNOWN)
      └─ Feedback ingestion (source: UNKNOWN)
           ↓ (store_results ?)

   4. OUTPUT (Results)
      ├─ Verdict (HOWL/WAG/GROWL/BARK)
      ├─ Q-Score [0, 100]
      ├─ Confidence [0, 0.618]
      └─ Dog votes {dog: score, ...}
           ↓ (feedback_collection ?)

   5. LEARNING (Feedback Loop)
      ├─ User feedback (source: UNKNOWN)
      ├─ Outcome signal (source: UNKNOWN)
      └─ → Q-Learning (integration: UNKNOWN)
   ```

   ## Status Summary

   | Stage | Status | Confidence |
   |-------|--------|-----------|
   | Ingestion | UNKNOWN | LOW |
   | Storage | PARTIAL | MEDIUM |
   | Processing | UNKNOWN | LOW |
   | Output | UNKNOWN | LOW |
   | Learning | UNKNOWN | CRITICAL |

   ## Critical Path Blockers

   - ❌ Ingestion framework (blocks everything)
   - ❌ Feedback pipeline (blocks learning)
   - ❌ Integration verification (can't test without this)

   ## Next Phase

   Phase 3: Build measurement infrastructure to verify pipelines
   EOF
   ```

2. Commit:
   ```bash
   git add docs/audit/data-pipeline-map.md
   git commit -m "docs(audit): create overall data pipeline map"
   ```

---

### Task 2.6: Create data quality baseline

**Files:**
- Create: `docs/audit/data-quality-baseline.json`

**Steps:**

1. Create baseline metrics from consciousness.json:
   ```bash
   cat > docs/audit/data-quality-baseline.json << 'EOF'
   {
     "timestamp": "2026-02-27",
     "data_quality_baseline": {
       "total_judgments": 851,
       "judgments_by_reality": {
         "CYNIC": 2,
         "CODE": 2,
         "MARKET": 1,
         "SOLANA": 1,
         "GOVERNANCE": 0,
         "OTHER": 845
       },
       "data_coverage": {
         "judgments_with_feedback": "UNKNOWN",
         "judgments_with_outcome_signal": "UNKNOWN",
         "judgments_with_learning_signal": "UNKNOWN"
       },
       "data_quality": {
         "schema_violations": "UNKNOWN",
         "null_values": "UNKNOWN",
         "missing_fields": "UNKNOWN",
         "stale_data_ratio": "UNKNOWN"
       },
       "q_learning_metrics": {
         "q_table_states": 6,
         "q_table_entries": 10,
         "total_updates": 966,
         "total_visits": 3703,
         "coverage_pct": 1.7
       },
       "learning_signal_quality": {
         "signal_rate": "UNKNOWN",
         "signal_timeliness": "UNKNOWN",
         "signal_correctness": "UNKNOWN"
       },
       "notes": "Most metrics UNKNOWN — need instrumentation in Phase 3"
     }
   }
   EOF
   ```

2. Commit:
   ```bash
   git add docs/audit/data-quality-baseline.json
   git commit -m "docs(audit): create data quality baseline"
   ```

---

### Task 2.7: Create data gaps CSV

**Files:**
- Create: `docs/audit/data-gaps.csv`

**Steps:**

1. Create comprehensive gaps list:
   ```bash
   cat > docs/audit/data-gaps.csv << 'EOF'
   category,gap_name,severity,impact,blocker,notes
   ingestion,CODE_data_source,CRITICAL,Can't judge code,TRUE,Where does code come from?
   ingestion,MARKET_data_source,CRITICAL,Can't judge tokens,TRUE,No price feed found
   ingestion,GOVERNANCE_data_source,CRITICAL,Can't judge proposals,TRUE,No DAO integration
   ingestion,Input_validation,HIGH,Bad data corrupts learning,TRUE,No sanitization found
   ingestion,Input_schema,HIGH,Can't standardize,FALSE,Schema not documented
   storage,Judgment_schema,HIGH,Can't build test,TRUE,Fields unknown
   storage,Decision_schema,MEDIUM,Missing audit trail,FALSE,May not exist
   storage,Embedding_schema,MEDIUM,Can't retrieve semantic,FALSE,May not exist
   processing,Judge_integration,CRITICAL,Dogs may not work together,TRUE,Integration untested
   processing,Dog_voting_mechanism,CRITICAL,Consensus unknown,TRUE,PBFT integration unverified
   processing,Q_Learning_trigger,CRITICAL,Learning can't start,TRUE,Feedback path broken
   processing,Thompson_trigger,HIGH,Bayesian inference blocked,TRUE,Trigger mechanism unknown
   processing,E_Score_update,HIGH,Reputation system broken,FALSE,Update mechanism unknown
   learning,Feedback_collection,CRITICAL,Learning impossible,TRUE,User feedback path unknown
   learning,Outcome_signal,CRITICAL,Learning validation broken,TRUE,Ground truth not connected
   learning,Cross_reality_transfer,HIGH,No cross-domain learning,FALSE,Not implemented
   monitoring,Data_quality_metrics,HIGH,Can't diagnose issues,FALSE,No telemetry
   monitoring,Pipeline_health,MEDIUM,No alerts on failure,FALSE,No monitoring
   EOF
   ```

2. Commit:
   ```bash
   git add docs/audit/data-gaps.csv
   git commit -m "docs(audit): comprehensive data gaps inventory"
   ```

---

### Task 2.8: Commit Phase 2 work

**Steps:**

1. Verify all Phase 2 files:
   ```bash
   ls -la docs/audit/pipeline-*.md docs/audit/data-*.* scripts/audit/
   ```

2. Final commit:
   ```bash
   git add docs/audit/ scripts/audit/
   git commit -m "docs(audit): complete phase 2 data pipeline audit"
   ```

---

## PHASE 3-6: Measurement, Testing, Diagnosis, Completeness

(Remaining phases follow similar structure — 60+ tasks total)

---

## EXECUTION PATH

This plan is designed for **subagent-driven execution**:

1. **Each task** = 2-5 minutes work
2. **Each commit** = Incremental progress
3. **After each phase** = Review findings before proceeding

---

**Total Effort:** 5.5 weeks (~60 tasks)
**Blocking Issues:** Data pipelines (Phase 2 finding)
**Critical Path:** Phases 1-2 must complete before Phases 3-6 can proceed

