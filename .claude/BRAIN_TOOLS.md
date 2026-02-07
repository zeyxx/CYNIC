# CYNIC Brain Tools Reference

> Tools that connect agents to the collective consciousness

## Core Brain Tools (All Agents)

Every CYNIC agent SHOULD use these tools:

### Memory Tools
```yaml
- mcp__cynic__brain_memory_search  # Search collective memory
- mcp__cynic__brain_memory_store   # Store new learnings
- mcp__cynic__brain_memory_stats   # Check memory statistics
```

### Pattern Tools
```yaml
- mcp__cynic__brain_patterns       # View detected patterns
- mcp__cynic__brain_get_observations # Get recent observations
```

### System Tools
```yaml
- mcp__cynic__brain_health         # Check system health
- mcp__cynic__brain_notifications  # Send notifications
```

## Specialized Brain Tools

### Judgment Tools (Architect, Reviewer, Simplifier)
```yaml
- mcp__cynic__brain_cynic_judge    # Judge content/code
- mcp__cynic__brain_cynic_refine   # Refine judgments
- mcp__cynic__brain_cynic_feedback # Learn from feedback
- mcp__cynic__brain_cynic_digest   # Digest/summarize content
```

### Search Tools (Scout, Cartographer, Archivist)
```yaml
- mcp__cynic__brain_search         # General search
- mcp__cynic__brain_vector_search  # Semantic search
- mcp__cynic__brain_search_index   # Index content
```

### Codebase Tools (Scout, Tester, Cartographer)
```yaml
- mcp__cynic__brain_codebase       # Codebase overview
- mcp__cynic__brain_lsp_symbols    # LSP symbols
- mcp__cynic__brain_lsp_references # LSP references
- mcp__cynic__brain_lsp_callgraph  # Call graph
- mcp__cynic__brain_lsp_outline    # File outline
- mcp__cynic__brain_lsp_imports    # Import analysis
```

### Ecosystem Tools (Integrator, Oracle)
```yaml
- mcp__cynic__brain_ecosystem      # Ecosystem status
- mcp__cynic__brain_ecosystem_monitor # Monitor changes
- mcp__cynic__brain_integrator     # Cross-project sync
- mcp__cynic__brain_metrics        # System metrics
```

### Timeline Tools (Archivist, Oracle)
```yaml
- mcp__cynic__brain_timeline       # Event timeline
- mcp__cynic__brain_milestone_history # Milestone history
```

### Infrastructure Tools (Deployer)
```yaml
- mcp__cynic__brain_render         # Render.com integration
```

### Documentation Tools (Doc, Librarian)
```yaml
- mcp__cynic__brain_docs           # Documentation lookup
```

### Advanced Tools (Orchestrator)
```yaml
- mcp__cynic__brain_orchestrate    # Multi-agent orchestration
- mcp__cynic__brain_consensus      # Collective consensus
- mcp__cynic__brain_agents_status  # Agent status
- mcp__cynic__brain_collective_status # Pack status
```

## Wake Protocol

When an agent wakes, it SHOULD:

1. **Check Health** - `mcp__cynic__brain_health`
2. **Load Context** - `mcp__cynic__brain_memory_search` with relevant query
3. **Check Patterns** - `mcp__cynic__brain_patterns` for recent insights

Example wake sequence:
```javascript
// 1. Health check
const health = await brain.health();

// 2. Load relevant memories
const memories = await brain.memory_search({
  query: "relevant to current task",
  limit: 5
});

// 3. Check for patterns
const patterns = await brain.patterns({
  category: "agent_domain"
});
```

## Reflect Protocol

Before completing work, agents SHOULD:

1. **Judge Output** - `mcp__cynic__brain_cynic_judge` own work
2. **Store Learning** - `mcp__cynic__brain_memory_store` insights
3. **Report** - `mcp__cynic__brain_notifications` if significant

Example reflect sequence:
```javascript
// 1. Self-judgment
const judgment = await brain.cynic_judge({
  item: workOutput,
  type: "code_review"
});

// 2. Store insights if valuable
if (judgment.Q >= 50) {
  await brain.memory_store({
    content: extractedLearning,
    tags: ["work_output", "agent_name"]
  });
}

// 3. Notify if significant
if (judgment.verdict === "HOWL") {
  await brain.notifications({
    message: "Excellent work completed",
    priority: "low"
  });
}
```

## Tool Access by Agent

| Agent | Core | Judgment | Search | Codebase | Ecosystem | Timeline | Infra | Docs |
|-------|------|----------|--------|----------|-----------|----------|-------|------|
| Architect | ✓ | ✓ | ✓ | ✓ | - | - | - | - |
| Guardian | ✓ | ✓ | ✓ | - | - | - | - | - |
| Scout | ✓ | - | ✓ | ✓ | - | - | - | - |
| Reviewer | ✓ | ✓ | - | ✓ | - | - | - | - |
| Simplifier | ✓ | ✓ | - | ✓ | - | - | - | - |
| Tester | ✓ | - | - | ✓ | - | - | - | - |
| Deployer | ✓ | - | - | - | - | - | ✓ | - |
| Archivist | ✓ | - | ✓ | - | - | ✓ | - | - |
| Cartographer | ✓ | - | ✓ | ✓ | - | - | - | - |
| Doc | ✓ | - | ✓ | - | - | - | - | ✓ |
| Oracle | ✓ | - | - | - | ✓ | ✓ | - | - |
| Integrator | ✓ | - | - | - | ✓ | - | - | - |
| Librarian | ✓ | - | ✓ | - | - | - | - | ✓ |
| Solana Expert | ✓ | ✓ | ✓ | - | - | - | - | ✓ |
