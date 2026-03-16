# CYNIC Empirical — Research and Investigation Protocol

Use when researching practices, investigating problems, or exploring external solutions.
Grounds decisions in evidence before acting.

**Announce at start:** "I'm using cynic-empirical to research this before acting."

## When to Use

- Investigating unfamiliar libraries, tools, or practices
- Exploring how others solved a problem (industrial practices, open-source patterns)
- Auditing current state before making changes
- Gathering evidence to support or falsify a hypothesis

## Protocol

### 1. State the question precisely
Write down exactly what you're trying to find out. Vague questions produce vague research.

### 2. Form a hypothesis
What do you EXPECT to find? State it explicitly. Research without a hypothesis is browsing.

### 3. Research
Use appropriate tools:
- `context7` MCP: up-to-date library documentation
- `ts_exec`: probe live infrastructure directly
- `cynic_audit_query`: query CYNIC's own verdict history
- Web search: external practices and standards

### 4. Challenge your hypothesis
What would falsify it? Apply the strongest counterargument.
If you cannot find a counterargument, you haven't researched hard enough.

### 5. Crystallize findings
Apply `crystallize-truth` for complex findings with multiple competing claims.
Confidence cap: φ⁻¹ = 61.8% — never claim certainty from research alone.

### 6. Act on evidence
Only proceed once you have evidence, not assumptions. If evidence is inconclusive:
document the uncertainty, state your best estimate with confidence, and proceed with
the lowest-risk option.

## Anti-patterns

- Researching after deciding (post-hoc rationalization)
- Stopping at the first confirming source
- Treating documentation as ground truth (always probe the live system)
- Research loops without a conclusion (set a time bound: 2-3 search rounds max)
