# Organ Prompt Enricher

A dedicated CYNIC microservice (Organ) to dynamically enrich tasks and prompts before they are executed by Cortex agents.

## Directory Structure
- `src/`: Core logic (enrichment, RAG injection, mempool listener).
- `templates/`: Jinja2 or structured prompt templates (Few-Shot, CoT).
- `config/`: Environment and routing configurations.

## Architecture & The "Flawless Loop"

1. **Task Submission (Mempool)**
   The human (or another system) dispatches a task to the CYNIC Kernel using `cynic_dispatch_agent_task`.
2. **Interception**
   This organ listens to the Kernel's `/agent-tasks` mempool for tasks with state `raw`.
3. **Enrichment**
   The organ loads templates from `templates/`, injects relevant RAG context via `llama-embed`, and enforces structural rules.
4. **Re-dispatch**
   The organ updates the task in the Kernel to `enriched`.
5. **Execution**
   A Cortex (like Antigravity or Claude) claims the `enriched` task, entirely abstracted from the complexity of the initial prompt.
