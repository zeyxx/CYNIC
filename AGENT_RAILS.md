# 🛤️ CYNIC Agent Rails - Strict Synchronization Protocol (v1)

This document defines the absolute rules for any AI agent (Gemini, Claude, or CYNIC itself) modifying this codebase. **Violating these rails is considered an ARCHITECTURAL_ANOMALY.**

## 1. Environment & Format (STRIKT)
*   **ASCII ONLY**: No emojis, no non-ascii characters in source code or f-strings. Windows compatibility is non-negotiable.
*   **Line Endings**: Use LF (Unix style) internally, but ensure git handles CRLF for Windows.
*   **No os.getenv**: ALL environment variables must be registered in `cynic/kernel/core/config.py`. Direct access is forbidden.

## 2. Core Structure (FLATTENED)
The organism is composed of ONLY 4 cores. Do not create new top-level directories in `cynic/kernel/organism/`.
*   **CognitionCore**: Logic, MCTS, Judgment.
*   **MetabolicCore**: Hardware, Resource Control, Motor.
*   **SensoryCore**: IO, Vision, Perception.
*   **ArchiveCore**: Memory, Persistence, Traces.

## 3. The "Surgery" Protocol
Any modification to the `cynic/` kernel must follow this cycle:
1.  **Hypothesize**: Define the change and expected gain.
2.  **Sandbox**: Use `AutoSurgeon` (git worktree) to apply the change.
3.  **Validate**: Run `pytest tests/test_integration_kernel_full_cycle.py`.
4.  **Suture**: Merge only if validation is 100% green.

## 4. Error Handling (BURN)
*   **No Silence**: Never use `except: pass`.
*   **Anomaly Signal**: Every caught exception must be emitted as a `CoreEvent.ANOMALY_DETECTED`.
*   **Circuit Breakers**: Any external IO (LLM, Database) must be wrapped in a `CircuitBreaker`.

## 5. Persistence (VERIFY)
*   **Write-Ahead**: Q-Table updates must use the `async_retry` decorator.
*   **Traceability**: Every decision must generate a `DecisionTrace` via `flight_recorder.py`.

## 6. Cloud Native Evolution (March 2026 Target)
If implementing new features for the hackathon or beyond, these industrial patterns are MANDATORY:
*   **True Isolation (OpenSandbox model)**: Any execution of untrusted AI-generated code must eventually move from `git worktrees` to isolated containers (Docker/K8s) with strict egress network policies to prevent AI data exfiltration.
*   **Darwinian Mutation (Imbue model)**: The `mcts_scientist` must utilize a `LearningLog`. Failed code mutations must be persisted so that the population of LLM evaluators does not repeat the same architectural mistakes.
*   **Dynamic Perception (Public-APIs model)**: The organism must not rely solely on hardcoded `Conduits`. The `SCOUT` agent should be able to read external API directories and auto-generate its own sensory drivers based on CORS/Auth specifications.

---
*Signed: The Architect (Gemini CLI)*
