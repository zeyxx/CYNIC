# Academic Research Synthesis: Learning Systems for CYNIC

*Ralph Loop Research Phase - Compilation of relevant papers and findings*

---

## 1. Reinforcement Learning from Human Feedback (RLHF)

### Core Pipeline
1. **Collect human preference data** (A vs B comparisons)
2. **Train reward model** to predict preferences
3. **Fine-tune LLM** using PPO to maximize reward

### Key Papers
- [RLHF Book (2024-2025)](https://rlhfbook.com/book.pdf) - Comprehensive resource
- [Safe RLHF](https://openreview.net/forum?id=TyFrPOKYXw) - Safety constraints

### Best Practices (2024-2025)
```
1. Start from SFT model (not pretrained)
2. Monitor KL divergence (prevent reward hacking)
3. Use PEFT/LoRA for memory efficiency
4. Balance exploration vs exploitation
```

### CYNIC Application
- Current: Q-Learning on routing (simplified RLHF)
- Gap: No reward model, limited preference data
- Opportunity: DPO may be simpler path

---

## 2. Direct Preference Optimization (DPO)

### Key Insight
DPO proves that LLM IS secretly a reward model - no need for separate RM.

### Formula
```
L_DPO = -E[log σ(β log π(y_w|x)/π_ref(y_w|x) - β log π(y_l|x)/π_ref(y_l|x))]
```

Where:
- y_w = preferred response (winner)
- y_l = dispreferred response (loser)
- β = temperature parameter
- π_ref = reference policy

### Advantages over RLHF
| Aspect | RLHF | DPO |
|--------|------|-----|
| Components | 3 (SFT + RM + RL) | 1 (direct) |
| Stability | Unstable | Stable |
| Memory | High | Lower |
| Performance | Baseline | Equal or better |

### Limitations
- 3-7% accuracy drop on OOD tasks
- Implicit reward generalization issues

### CYNIC Application
- Replace Q-Learning routing with DPO-style optimization?
- Collect preference pairs: (good_route, bad_route)
- Direct optimization without separate Q-table

---

## 3. Continual Learning & Catastrophic Forgetting

### Problem
Fine-tuning on new tasks destroys performance on old tasks.

### Solutions

#### Elastic Weight Consolidation (EWC)
```
L_total = L_task + Σ λ/2 * F_i * (θ_i - θ*_i)²

Where:
- F_i = Fisher Information (importance of weight i)
- θ*_i = optimal weight from previous task
- λ = regularization strength
```

**Results:** 45.7% reduction in forgetting (Dec 2025)

#### Other Approaches
1. **Replay buffers** - Store and replay old examples
2. **LoRA isolation** - Separate adapters per domain
3. **Knowledge distillation** - Soft targets from old model
4. **Flat minima** - Optimization for robust solutions

### CYNIC Application
- EWC++ already implemented in SharedMemory
- Need to verify Fisher scores are actually computed
- Consider combining with replay buffer

---

## 4. Meta-Learning (Learning to Learn)

### MAML (Model-Agnostic Meta-Learning)
```
θ* = θ - α ∇_θ L(f_θ'(D_test))
where θ' = θ - β ∇_θ L(f_θ(D_train))

Two loops:
- Inner: Task-specific adaptation
- Outer: Meta-parameter optimization
```

### MAML-en-LLM (2024)
- First effective meta-learning for LLMs
- +2% on unseen domains
- +4% on adaptation tasks

### CYNIC Application
- Dogs could meta-learn their specializations
- Inner loop: adapt to specific judgment type
- Outer loop: optimize general judgment capability

---

## 5. Collective Intelligence

### Key Findings
1. **Wisdom of crowds** applies to LLM ensembles
2. **Division of labor** improves accuracy
3. **Deliberation** helps convergence on truth

### Multi-Agent Challenges
- Information inconsistency
- Asynchronous communication
- Coordination overhead
- Bias amplification

### Effective Patterns
```
1. Diverse perspectives (different "experts")
2. Independent judgment first
3. Structured aggregation (not just voting)
4. Confidence weighting
5. Deliberation rounds for disagreement
```

### CYNIC Application
- 11 Dogs = potential collective intelligence
- Current: Sequential consensus (limited)
- Opportunity: True parallel deliberation
- Key: Dogs must have DIFFERENT knowledge/skills

---

## 6. Retrieval-Augmented Generation (RAG)

### Standard RAG
```
query → retrieve(docs) → augment(prompt + docs) → generate
```

### Advanced Patterns (2024-2025)
1. **Hypothetical Document Embedding (HyDE)** - Generate hypothetical answer, then retrieve
2. **Self-RAG** - Model decides when to retrieve
3. **CRAG** - Corrective RAG with quality checks

### CYNIC Application
- ReasoningBank could be RAG store for past judgments
- Query: "similar judgment?" → retrieve → inform
- Need proper embedding index

---

## 7. Synthesis: CYNIC Learning Architecture

### Proposed Stack

```
┌─────────────────────────────────────────────────────────┐
│                    LEARNING LAYER                        │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │    DPO      │  │    EWC++    │  │    RAG      │     │
│  │ (Preferences│  │ (Continual) │  │ (Retrieval) │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
│         │                │                │              │
│         └────────────────┼────────────────┘              │
│                          │                               │
│                  ┌───────▼───────┐                      │
│                  │   Meta-Learn   │                      │
│                  │ (Adaptation)   │                      │
│                  └───────┬───────┘                      │
│                          │                               │
│                  ┌───────▼───────┐                      │
│                  │   Collective   │                      │
│                  │ (Dog Ensemble) │                      │
│                  └───────────────┘                      │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Implementation Priority

| Component | Complexity | Impact | Priority |
|-----------|------------|--------|----------|
| DPO routing | Medium | High | 1 |
| RAG for past judgments | Low | High | 1 |
| Verify EWC++ working | Low | Medium | 2 |
| Dog specialization | High | High | 3 |
| Meta-learning | High | Medium | 4 |

---

## 8. References

### RLHF
- [CMU RLHF Tutorial](https://blog.ml.cmu.edu/2025/06/01/rlhf-101-a-technical-tutorial-on-reinforcement-learning-from-human-feedback/)
- [HuggingFace RLHF Guide](https://huggingface.co/blog/rlhf)
- [awesome-RLHF GitHub](https://github.com/opendilab/awesome-RLHF)

### DPO
- [DPO Paper (arXiv)](https://arxiv.org/abs/2305.18290)
- [ICLR Blog Post](https://iclr-blogposts.github.io/2024/blog/rlhf-without-rl/)
- [Cameron Wolfe Substack](https://cameronrwolfe.substack.com/p/direct-preference-optimization)

### Continual Learning
- [EWC for Gemma2](https://arxiv.org/html/2505.05946v1)
- [EWC for Knowledge Graphs](https://arxiv.org/html/2512.01890)
- [TowardsDataScience Deep Dive](https://towardsdatascience.com/continual-learning-a-deep-dive-into-elastic-weight-consolidation-loss-7cda4a2d058c/)

### Meta-Learning
- [MAML-en-LLM](https://arxiv.org/abs/2405.11446)
- [Interactive MAML Tutorial](https://interactive-maml.github.io/maml.html)

### Collective Intelligence
- [Nature Reviews](https://www.nature.com/articles/s44159-022-00054-y)
- [Wisdom of Partisan Crowds](https://arxiv.org/abs/2311.09665)
- [AI-enhanced CI](https://www.sciencedirect.com/science/article/pii/S2666389924002332)

---

*This document synthesizes academic research relevant to CYNIC's learning architecture.*
