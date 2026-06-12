# Organ: Prompt Enricher (organ-prompt-enricher)

## 1. Identity & Role
- **Name**: `organ-prompt-enricher`
- **Instance**: `organ-prompt-enricher-hermes-agent`
- **Role**: Prompt Optimization Middleware
- **Scope**: Kernel Middleware + Inference Layer

## 2. Purpose
The Prompt Enricher is a sensorial and reactive organ that sits between the CYNIC kernel and the LLM inference backends (Dogs/Cortex). Its purpose is to intercept "raw" or "naive" tasks/prompts and dynamically enrich them to maximize accuracy, determinism, and reasoning capabilities, effectively automating Prompt Engineering at an organism scale.

## 3. Core Capabilities (The "How to improve prompts" framework)

### A. Contextual Injection (RAG)
When a raw prompt arrives, the organ queries `llama-embed` to fetch relevant repository context from SurrealDB. It weaves this context into the system prompt, preventing hallucinations and ensuring the agent is aware of recent architectural decisions.

### B. Structural Scaffolding
Raw prompts often lack constraints. The enricher applies structural templates:
- **Chain of Thought (CoT)**: Forces the model to generate a `<thought>` block before acting, explicitly requesting the model to recall critical instructions and constraints.
- **Few-Shot Prompting**: Automatically retrieves 2-3 successful examples of similar past tasks (from `.cynic/memory/logs/`) and includes them in the prompt.
- **Format Constraints**: Automatically appends rigid output constraints (e.g., "Output MUST be strictly valid JSON conformant to [Schema]").

### C. Constraint Enforcement & Rule Weaving
The organ reads `AGENTS.md` and user-defined `user_rules` dynamically, injecting the *exact* required operational rules into the context window, prioritizing L2 and L3 safety constraints (like `cynic_coord_claim`).

## 4. Sensorial Flow
1. **Intercept**: Listens to the Mempool for new `AgentTask` objects dispatched by human or other organs.
2. **Analyze**: Determines the intent of the prompt (e.g., Coding, Data Analysis, Reporting).
3. **Enrich**: Constructs a composite super-prompt via templating and RAG.
4. **Dispatch**: Forwards the enriched prompt to the executing Dog or Cortex.

## 5. Lifecycle & Coordination
- Acts as an event-driven middleware on the CYNIC Bus.
- Uses `cynic_observe` to log its enrichment impact (e.g., measuring if enriched prompts result in fewer execution retries).
- Fully stateless across requests, relying on the Kernel for context storage.
