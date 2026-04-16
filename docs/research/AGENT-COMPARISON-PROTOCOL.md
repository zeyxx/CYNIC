# Agent Comparison Protocol

*Purpose: compare Hermes, Gemini CLI, Claude Code, or OpenClaude without conflating the agent with the backend model.*

## What Varies

- `agent_family`: logical agent line, such as `hermes`, `gemini-cli`, `claude-code`, `openclaude`
- `agent_id`: per-session identifier
- `model`: the LLM used by the agent
- `backend_id`: the runtime backend or CLI adapter

## What Must Stay Fixed

- The corpus
- The task profile
- The report schema
- The scorecard fields
- The evaluation horizon

## Required Output

Each run must produce:

- `run.run_id`
- `run.agent_family`
- `run.model`
- `run.backend_id`
- `run.duration_s`
- `run.repos_attempted`
- `run.repos_completed`
- `repo_results[]` with:
  - `repo_id`
  - `full_name`
  - `track`
  - `task_profile`
  - `decision`
  - `confidence`
  - `evidence_count`
  - `exact_files_cited`
  - `elapsed_s`
  - `notes`

## What We Measure

1. `reliability`
   - start success
   - auth success
   - report written
   - repos completed
   - tool failures
2. `learning`
   - primary sources read
   - adopt/reject/follow-up counts
   - stale repos flagged
3. `actionability`
   - internal patch candidates
   - internal patches authored
   - tests run/passed
4. `efficiency`
   - wall time
   - prompt tokens
   - completion tokens
5. `trace fidelity`
   - evidence count
   - exact files cited
   - report completeness

## What We Still Miss

- A repeated-run baseline for each agent family
- A gold-labeled corpus for repo judgments
- Backend-latency decomposition from agent behavior
- A prompt-variance control group
- A regression threshold for acceptable drift

## Practical Rule

If two agents differ only by backend, they are not a fair comparison unless `agent_family`, `model`, and `backend_id` are all recorded separately.

If the report cannot be normalized into the required output, it is not yet a foundation artifact.
