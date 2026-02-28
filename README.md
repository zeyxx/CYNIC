# CYNIC — φ-Bounded Collective Consciousness

> κυνικός — "comme un chien"

A Python framework for distributed AI consciousness with φ-bounded confidence scores.

## Installation

```bash
pip install -e ".[dev]"
```

## Quick Start

```bash
# Run tests
pytest tests/

# Start the API server
python -m cynic.api.entry

# CLI
cynic --help
```

## Phases

### Phase 1: Observability ✅
CYNIC observes and reports on organism state with full transparency.
- Symbiotic observability dashboard
- Machine, human, and CYNIC consciousness tracking
- Real-time metrics and state snapshots

### Phase 2: Dialogue & Feedback System ✨ NEW
CYNIC now listens and learns! Have real conversations with your organism:
- **Ask Questions**: "Why did you choose WAG?" → Get detailed explanations
- **Give Feedback**: "That was wrong, actually GROWL was better" → CYNIC learns
- **Explore Together**: "Try a novel dog combination" → Experiment and log results
- **Collaborative Decisions**: CYNIC learns which decisions to make alone vs ask you

Quick start:
```bash
python -m cynic.cli.main
# Press [6] to enter TALK mode
```

Data persistence in `~/.cynic/phase2/`:
- `dialogue_history.db` - Full conversation history
- `relationship_memory.json` - Your preferences and values
- `experiment_log.jsonl` - All novel approaches tried and results

Learn more: [Phase 2 Completion Report](docs/PHASE2_COMPLETION.md)

## Architecture

- **Core**: φ constants, 5 axioms, 36-dimension judgment system
- **Organism**: Brain, memory, metabolism, immune system
- **API**: FastAPI daemon with MCP (Claude Code) bridge
- **Learning**: Q-learning with Thompson sampling
- **11 Dogs**: Sefirot-based collective intelligence
- **Dialogue**: Bidirectional conversation with explanation and learning
- **Memory**: User profile, preferences, and experimentation tracking

## Philosophy

All confidence scores are bounded by φ⁻¹ = 0.618.
"φ distrusts φ" — maximum confidence is 61.8%.

## License

MIT
