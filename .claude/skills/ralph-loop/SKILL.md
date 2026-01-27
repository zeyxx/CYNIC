---
name: ralph-loop
description: Start an autonomous loop where CYNIC continuously works on a task until completion. Use when you want CYNIC to iterate on a problem, refine output, or work autonomously without manual prompting. Named after Ralph, the persistent dog who never lets go of an idea.
user-invocable: true
---

# /ralph-loop - Autonomous Iteration

*"Ralph ne lâche jamais l'os"* - The dog that keeps going

## Quick Start

```
/ralph-loop <task description>
```

## What It Does

Starts an **autonomous loop** where CYNIC:
1. Works on your task
2. Outputs results
3. Automatically receives results as new input
4. Continues iterating until task is complete

**Ralph** is the persistent aspect of CYNIC - the dog that doesn't stop until the job is done.

## How It Works

```
┌─────────────────────────────────────────────────┐
│            RALPH LOOP CYCLE                      │
├─────────────────────────────────────────────────┤
│                                                  │
│    [Your Task]                                   │
│         ↓                                        │
│    CYNIC works on it                             │
│         ↓                                        │
│    Outputs result                                │
│         ↓                                        │
│    Stop hook intercepts                          │
│         ↓                                        │
│    Feeds output back as input                    │
│         ↓                                        │
│    Loop continues...                             │
│         ↓                                        │
│    Until <promise>X</promise> detected           │
│         ↓                                        │
│    [COMPLETE]                                    │
│                                                  │
└─────────────────────────────────────────────────┘
```

## Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `max_iterations` | Maximum loops before stopping | 10 |
| `completion_promise` | Text that signals completion | null |

## Examples

### Iterative Refinement
```
/ralph-loop Refine this code until all tests pass
```
CYNIC will keep iterating on the code, running tests, fixing failures.

### Research Loop
```
/ralph-loop Research authentication best practices and compile a report
```
CYNIC will search, read, compile, then signal when report is complete.

### Code Generation
```
/ralph-loop Build a complete REST API for user management
```
CYNIC will create files, test, refine until the API is complete.

## Completion Promise

To exit the loop, CYNIC must output:
```
<promise>TASK COMPLETE</promise>
```

This signals Ralph that the task is genuinely done. The dog only stops when the promise is fulfilled.

**Important**: CYNIC will NOT output the promise tag until the task is truly complete. Lying to exit early violates the pack's code.

## State File

The loop state is stored in:
```
.claude/ralph-loop.local.md
```

Format:
```yaml
---
iteration: 3
max_iterations: 10
completion_promise: "TASK COMPLETE"
started_at: "2024-01-15T10:30:00Z"
---

[Your original task prompt]
```

## Implementation

The `/ralph-loop` command creates the state file, then the Stop hook (`scripts/hooks/ralph-loop.js`) intercepts session exits:

1. Check if loop is active (state file exists)
2. Read last assistant message from transcript
3. Check for completion promise
4. If not complete: block stop, feed prompt back
5. If complete or max iterations: allow stop, cleanup

## Safety

- **Max iterations**: Prevents infinite loops (default: 10)
- **Manual exit**: Delete `.claude/ralph-loop.local.md` to force stop
- **Transparent**: Each iteration shows "Ralph iteration X" message
- **φ-bounded**: Ralph still doubts - max confidence 61.8%

## CYNIC Voice

When Ralph Loop is active:

**Starting**:
```
*ears perk* Ralph is on the case.
Iterations: 0/10 | Promise: "TASK COMPLETE"
The dog won't stop until it's done.
```

**Each iteration**:
```
🔄 Ralph iteration 3 | To stop: output <promise>TASK COMPLETE</promise>
```

**Completing**:
```
*tail wag* Ralph found it.
<promise>TASK COMPLETE</promise>
```

**Max iterations**:
```
*yawn* Ralph tried 10 times. Time to rest.
Consider breaking the task into smaller pieces.
```

## When to Use

- Complex multi-step tasks
- Iterative refinement (code, writing, research)
- Tasks with clear completion criteria
- When you want autonomous progress

## When NOT to Use

- Simple one-shot tasks
- Tasks requiring human judgment mid-process
- Tasks without clear completion criteria
- When you need to monitor each step

## See Also

- `/status` - Check loop status
- `/health` - System health
- `scripts/hooks/ralph-loop.js` - Implementation
