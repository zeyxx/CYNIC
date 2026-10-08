# Hermes-Agent Capabilities Reference

> Complete map of Hermes-Agent features, commands, and current usage status.
> Last updated: 2026-10-05
> Purpose: prevent capability loss, guide bottom-up adoption.

---

## 1. MULTI-AGENT IDENTITY

### 1.1 Profiles (isolated agents)

Each profile = fully independent agent with own config, SOUL.md, memory, skills, cron, API keys, sessions.

```bash
# Create
hermes profile create <name>              # empty
hermes profile create <name> --clone      # copy config/.env/SOUL.md/skills
hermes profile create <name> --clone-all  # full copy (no history/channels)
hermes profile create <name> --clone-from <source>
hermes profile create <name> --clone-channels  # also copy bot tokens (careful)

# Use
hermes -p <profile> chat
hermes -p <profile> doctor
hermes -p <profile> gateway start
hermes -p <profile> skills list
```

**Status**: NOT USED (only `default` profile exists)

**Use case for CYNIC**:
- `coder` profile: Rust refactorization, extraction, tests
- `reviewer` profile: code review, architecture audit, debt analysis
- `researcher` profile: web search, documentation, competitive intel
- `ops` profile: monitoring, cron, build health

### 1.2 Sub-agents (delegate_task)

Ephemeral agents spawned mid-task to parallelize work.

```python
# From agent context
delegate_task(tasks=[
    {"goal": "Extract crate X from monolith", "context": "..."},
    {"goal": "Audit crate Y for debt", "context": "..."},
])
```

**Features**:
- Live transcripts (tail -f)
- Control: list, steer, stop
- JSON schema validation
- Cost tracking per delegation
- Max 10 concurrent children, 250 iterations

**Status**: NOT USED

**Use case for CYNIC**: parallel crate extraction, parallel audit, parallel test generation

### 1.3 Personalities

```bash
# Temporary (per-session)
/personality concise
/personality teacher
/personality  # back to SOUL.md default

# Permanent (config.yaml)
# agent:
#   personalities:
#     codereviewer: >
#       You are a meticulous code reviewer. Identify bugs, security issues,
#       performance concerns, and unclear design choices.
```

**Status**: NOT USED (no custom personalities)

**Use case for CYNIC**:
- `architect`: system design, trade-offs, decision records
- `auditor`: debt analysis, contract verification, proof checking
- `mentor`: teaching, explaining, onboarding

### 1.4 Peer (bot-to-bot)

Direct messages between profiles across gateways.

```bash
hermes peer --help
```

**Status**: NOT USED

**Use case for CYNIC**: researcher → coder handoff, reviewer → architect escalation

---

## 2. COORDINATION & PERSISTENCE

### 2.1 Kanban

Durable SQLite-backed task board shared across profiles.

```bash
hermes kanban init
hermes kanban create "Task title" --description "..."
hermes kanban list
hermes kanban claim <task_id>           # atomic claim
hermes kanban complete <task_id>
hermes kanban link <parent> <child>     # dependency
hermes kanban swarm "Goal"              # parallel workers → verifier → synthesizer
hermes kanban assign <task_id> <profile>
hermes kanban set-model <task_id> <model>
hermes kanban dispatch                  # execute claimed tasks
hermes kanban daemon                    # background dispatcher
```

**Status**: NOT USED

**Use case for CYNIC**: task coordination between profiles, dependency tracking, parallel execution

### 2.2 Cron Jobs

Scheduled tasks with persistent memory.

```bash
hermes cron create "job name" --schedule "0 8 * * *" --prompt "..."
hermes cron list
hermes cron run <job_id>
hermes cron notepad <job_id> get <key>
hermes cron notepad <job_id> set <key> <value>
hermes cron continuity true   # carry output between runs
```

**Status**: NOT USED

**Use case for CYNIC**:
- Daily build health check
- Weekly debt audit
- Hourly fleet monitoring
- Nightly backup verification

### 2.3 Checkpoints

Filesystem snapshots before mutations.

```bash
hermes checkpoints status
hermes checkpoints prune
hermes checkpoints clear
```

**Status**: DISABLED (`checkpoints.enabled: false`)

**Use case for CYNIC**: rollback before risky refactors, experiment isolation

### 2.4 Hooks

Shell scripts triggered by events.

```bash
hermes hooks list
hermes hooks test <event>
hermes hooks doctor
```

**Status**: NOT CONFIGURED

**Use case for CYNIC**: pre-commit validation, post-merge audit, notification triggers

---

## 3. ROUTING & INFERENCE

### 3.1 MoA (Mixture of Agents)

Combine multiple models into one response.

```bash
hermes moa list
hermes moa configure    # interactive
/moa <prompt>
```

**Status**: NOT CONFIGURED

**Use case for CYNIC**: architecture decisions (aggregator + references), code review synthesis

### 3.2 Fallback Providers

Chain of backup providers when primary fails.

```bash
hermes fallback list
hermes fallback add <provider>
hermes fallback remove <provider>
```

**Status**: EMPTY (`fallback_providers: []`)

**Use case for CYNIC**: opencode-go → nous → llama-server chain

### 3.3 Per-Profile Model Routing

Each profile can have its own default model.

```bash
hermes -p coder config set model.default opencode-go/deepseek-v4-flash
hermes -p reviewer config set model.default opencode-go/longcat-2.5-preview-free
```

**Status**: NOT USED (single global model)

**Use case for CYNIC**: cheap model for routine tasks, expensive for complex reasoning

---

## 4. TOOLS & INTEGRATION

### 4.1 Gateway

Multi-platform messaging (Telegram, Discord, Slack, WhatsApp).

```bash
hermes gateway start
hermes gateway status
hermes gateway install
```

**Status**: NOT CONFIGURED

**Use case for CYNIC**: Telegram alerts, Discord team coordination, WhatsApp notifications

### 4.2 Browser

Cloud or local browser automation.

```bash
hermes browser --help
```

**Status**: NOT USED

**Use case for CYNIC**: web research, documentation scraping, UI testing

### 4.3 Secrets

External secret sources (Bitwarden, 1Password).

```bash
hermes secrets --help
```

**Status**: NOT USED (env vars only)

**Use case for CYNIC**: API key management, credential rotation

### 4.4 LSP

Language Server Protocol integration.

```bash
hermes lsp --help
```

**Status**: NOT CONFIGURED

**Use case for CYNIC**: Rust analyzer integration, real-time diagnostics

### 4.5 Security

Supply-chain audit (OSV.dev).

```bash
hermes security --help
```

**Status**: NOT USED

**Use case for CYNIC**: dependency vulnerability scanning, plugin audit

### 4.6 Dashboard

Web UI (port 9119).

```bash
hermes dashboard
hermes dashboard --status
hermes dashboard --stop
```

**Status**: NOT RUNNING

**Use case for CYNIC**: visual monitoring, session management, cost tracking

### 4.7 Journey

Timeline of learned skills + memories.

```bash
hermes journey --help
```

**Status**: NOT USED

**Use case for CYNIC**: track agent learning, skill evolution, knowledge graph

### 4.8 Curator

Background skill maintenance.

```bash
hermes curator status
hermes curator run
hermes curator pause
```

**Status**: NOT USED

**Use case for CYNIC**: automatic skill updates, stale skill detection

### 4.9 Approvals

Smart approvals with LLM reviewer.

```bash
hermes approvals --help
# In chat: /deny <reason>
```

**Status**: NOT CONFIGURED

**Use case for CYNIC**: reduce interruptions, automated safe command approval

### 4.10 Worktree

Git worktree isolation per session.

```bash
hermes -w  # start in isolated worktree
hermes worktree  # audit and reclaim
```

**Status**: NOT USED

**Use case for CYNIC**: parallel experiments, safe refactoring isolation

### 4.11 Skill Bundles

Aliases for multiple skills.

```bash
hermes bundles --help
```

**Status**: NOT USED

**Use case for CYNIC**: `cynic-refactor` bundle = [cynic-refactor, cynic-crate-repair, cynic-architectural-ratchet]

### 4.12 External Memory

External memory provider.

```bash
hermes memory --help
```

**Status**: NOT CONFIGURED

**Use case for CYNIC**: Obsidian integration, vector DB, knowledge base

---

## 5. CURRENT CONFIG STATE

```yaml
# ~/.hermes/config.yaml (key sections)
model:
  provider: opencode-go
  default: space-bunny-free
  base_url: https://opencode.ai/zen/go/v1
  api_mode: chat_completions

providers:
  cynic-local:
    api: http://127.0.0.1:8080/v1
    key_env: CYNIC_LLAMA_API_KEY
  antigravity-proxy:
    enabled: false

fallback_providers: []

agent:
  max_turns: 150
  reasoning_effort: medium
  tool_use_enforcement: auto

checkpoints:
  enabled: false

terminal:
  backend: local
  timeout: 180
  persistent_shell: true
```

**Auth providers configured**:
- opencode-go (active)
- nous (oauth)
- antigravity (oauth, rate-limited)
- copilot (gh token)
- openai-codex (oauth)
- gemini-oauth

**Installed skills**: 60+ (mostly CYNIC-related)

**Installed plugins**: browser-browser-use, browser-browserbase, browser-firecrawl

---

## 6. BOTTOM-UP ADOPTION PLAN

### Phase 0: Measure (data-centric baseline)

Before any change, measure current state:
- Token usage per task type
- Cost per task
- Success rate per model
- Time per task
- Cache ratio (if available)

**Tool**: `opencode stats`, `hermes logs`, custom benchmark

### Phase 1: Profiles + Personalities (identity layer)

1. Create `coder` profile (clone from default)
2. Create `reviewer` profile (clone from default)
3. Define personalities: `architect`, `auditor`, `mentor`
4. Test: `hermes -p coder chat`, `hermes -p reviewer chat`

**Proof**: each profile responds with distinct identity

### Phase 2: Sub-agents (parallelization layer)

1. Test `delegate_task` with 2 parallel extractions
2. Measure: time vs sequential
3. Validate: output quality

**Proof**: parallel extraction completes faster with same quality

### Phase 3: Kanban (coordination layer)

1. `hermes kanban init`
2. Create tasks for CYNIC refactor
3. Assign to profiles
4. Test dispatch

**Proof**: tasks complete via kanban workflow

### Phase 4: Cron (automation layer)

1. Daily build health check
2. Weekly debt audit
3. Measure: detection time vs manual

**Proof**: issues detected automatically

### Phase 5: MoA + Routing (optimization layer)

1. Configure MoA for architecture decisions
2. Set up fallback chain
3. Per-profile model routing

**Proof**: cost reduction, quality improvement

---

## 7. REFERENCES

- Official docs: https://hermes-agent.nousresearch.com/docs
- Multi-profile gateways: https://hermes-agent.nousresearch.com/docs/user-guide/multi-profile-gateways
- Kanban: https://hermes-agent.nousresearch.com/docs/user-guide/features/kanban
- Profiles: https://hermes-agent.nousresearch.com/docs/user-guide/profiles
- v0.19.0 release notes: https://github.com/NousResearch/hermes-agent/releases
- v0.21.0 (Pantheon): https://github.com/NousResearch/hermes-agent/releases
