# CYNIC Architecture

## Overview

CYNIC is an autonomous AI agentic system.


## Philosophy: φ-Bounded Reasoning

Every architectural decision in CYNIC is grounded in φ-based mathematics:

- Confidence cap: 61.8% (φ⁻¹)
- Q-Score: Geometric mean of 5 axiom scores, capped at 100
- Verdict thresholds: HOWL ≥82, WAG ≥61.8, GROWL ≥38.2, BARK <38.2
- Timing: Fibonacci sequences determine cycle intervals


## Four Consciousness Levels

| Level | Name | Latency | Dogs Active | LLM Usage |
|-------|------|---------|-------------|-----------|
| L3 | REFLEX | <10ms | GUARDIAN, ANALYST, JANITOR, CYNIC | No |
| L2 | MICRO | ~500ms | + SCHOLAR, ORACLE | Fast models |
| L1 | MACRO | ~2.85s | All 11 Dogs | Full reasoning |
| L4 | META | ~4h daily | All 11 Dogs | Evolution |

L3 events can interrupt L2/L1. L4 consolidates discoveries.


## Module Layout

```
cynic/
├── cognition/          # Judgment pipeline (BRAIN)
│   ├── cortex/         # Orchestrator, handlers
│   └── neurons/        # 11 Dogs (Sefirot)
├── organism/           # State management (BODY)
├── learning/           # Q-Learning + EWC
├── llm/               # Multi-provider LLM routing
├── core/              # Shared infrastructure
├── metabolism/        # Execution & scheduling
├── immune/            # Safety guardrails
├── nervous/           # Observability
├── senses/           # Perception
└── api/              # FastAPI endpoints
```


## Core Interfaces

### StorageInterface

Abstract interface for all storage backends (PostgreSQL, SurrealDB).

### LLMProvider

Abstract adapter: OllamaAdapter, ClaudeAdapter, GeminiAdapter, LlamaCppAdapter.

### EventHandler

Three event buses: CORE (Judgment, Learning), AUTOMATION (Triggers), AGENT (Dogs).

## Data Flow

### PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE

1. **PERCEIVE**: Watchers gather input (Git, Market, Solana, Social)
2. **JUDGE**: 11 Dogs evaluate with PBFT consensus (7/11 quorum)
3. **DECIDE**: Action selection via Q-Table or Thompson Sampling
4. **ACT**: Execution with guardrails (PowerLimiter, AlignmentChecker, HumanGate)
5. **LEARN**: Q-Table update with TD(0) + EWC
6. **ACCOUNT**: Cost tracking and EScore update
7. **EMERGE**: Self-improvement and meta-cycle

## Key Data Models

### Cell

A point in the ∞^N hypercube: reality, analysis, time_dim, content, lod, consciousness.

### Judgment

Judgment output: q_score, verdict (HOWL/WAG/GROWL/BARK), confidence (φ-bounded), dog_votes.

## Consensus Protocol: PBFT

- Total Dogs: 11 (Lucas(5))
- Byzantine threshold: f = 3
- Quorum: 7 (2f+1)


## φ-Constants Reference

| Constant | Value | Derivation |
|----------|-------|------------|
| PHI | 1.618033988749895 | Golden Ratio |
| PHI_INV | 0.618033988749895 | φ⁻¹ |
| MAX_CONFIDENCE | 0.618 | φ⁻¹ — cap |
| HOWL_MIN | 82.0 | φ² × φ⁻¹ × 100 |
| WAG_MIN | 61.8 | φ⁻¹ × 100 |
| GROWL_MIN | 38.2 | φ⁻² × 100 |
| DOGS_TOTAL | 11 | Lucas(5) |
| DOGS_QUORUM | 7 | 2f+1 |
| AXIOMS_CORE | 5 | Fibonacci(5) |
| AXIOMS_FACETS | 7 | Lucas(4) |
| META_CYCLE_JUDGMENTS | 233 | Fibonacci(13) |

## Getting Started

```bash
python -m cynic.cli start
python -m cynic.cli chat
python -m cynic.cli status
```
