# Behavior ML Design — Learning Organic Movement from 533K Events

## Dataset Characteristics

| Metric | Value | Status |
|--------|-------|--------|
| Total events | 534,789 | ✓ High-volume |
| Sample period | 2026-04-26 — 2026-05-01 | ✓ 5 days of natural browsing |
| Event types | key, mouse_move, scroll, click | ✓ Rich signal |
| Pause distribution | mean=284ms, median=200ms, p95=817ms | ✓ Human-like |
| Event type ratio | keys:47.8%, mouse:47.2%, scrolls:12.2%, clicks:2.8% | ✓ Realistic (typing + browsing) |

## Core Insight

**Humans browse predictably.** Pause duration, scroll distance, click targeting, and keystroke patterns follow stable distributions. A model trained on 533K real events can generate browsing behavior indistinguishable from human input.

**Why this matters for Organ X:** An agent using behavior-learned sampling instead of heuristics will not trigger X.com bot detection because it IS human — learned from actual data, not hand-tuned rules.

---

## Architecture

### Phase 1: Event Tokenization

Convert raw events to fixed-size feature vectors:

```
Input event:
{
  "type": "mouse_move",
  "x": 2771,
  "y": 197,
  "ts": "2026-04-26T13:36:11.506560+00:00",
  "window_id": "0x2a0000a"
}

Tokenized (48-dim vector):
[
  type_embedding(5),        # key=0, mouse=1, scroll=2, click=3, unknown=4 (1-hot)
  x_bucket(10),             # x ∈ [0, 1920] → 10 buckets (1-hot)
  y_bucket(8),              # y ∈ [0, 1080] → 8 buckets (1-hot)
  pause_ms_bucket(8),       # Δt ∈ [0, 10000] → 8 log-buckets (1-hot)
  region_type(4),           # nav, feed, detail, search (1-hot from window_name heuristic)
  window_context(1)         # "home" vs "detail" vs "search" (embedding)
]
```

### Phase 2: Sequence Model (LSTM)

**Architecture:**
- Input: sequence of N=20 previous events (tokenized)
- Hidden: 128 units, 1 layer
- Output: probability distribution over next event type + continuous params

**Loss function:**
```
L = α * cross_entropy(P(type_next), ground_truth_type)
  + β * mse(pause_next, ground_truth_pause)
  + γ * mse(xy_next, ground_truth_xy)
```

Weights: α=1.0, β=0.5, γ=0.3 (type is most important)

**Training:**
- Split: 80% train, 10% val, 10% test
- Batch size: 256
- Epochs: 20 (early stop on val loss)
- Optimizer: Adam(lr=1e-3)
- Device: GPU (4060 Ti, ~12GB VRAM available)

### Phase 3: Inference (Agent Integration)

Once trained, the model becomes the agent's "typing simulator":

```python
# Agent wants to search for "solana defi"
next_event = behavior_model.sample(
    context=[last_20_events],
    intent="type_search_query"
)
# Returns: { type: "key", char: 's', pause_before: 145ms, ... }
# Agent executes this in the browser, pause, then generates next event

# Repeat until search bar is filled
search_text = ""
for _ in range(len("solana defi")):
    event = behavior_model.sample(context, "typing")
    browser.execute(event)
    search_text += event["char"]
```

**Result:** Search bar populated in human-realistic timing (no mechanical 50ms keypresses, no uniform pauses).

---

## Success Metrics

### Quantitative (Before/After)

| Metric | Baseline (Heuristic) | Target (LSTM) | Falsification |
|--------|----------------------|---------------|---------------|
| Prediction accuracy (next event type) | 30% (random) | **>75%** | <70% = dataset quality issue |
| Perplexity (pause timing) | ∞ (heuristic) | **<1.5** | >2.0 = data too uniform |
| X.com bot detection rate | TBD | **<5%** | >10% = model doesn't work |

### Qualitative (Visual Check)

Watch the agent move through X.com:
- Scrolls should have variable speed (slow reads, fast skimming)
- Clicks should cluster on feed items, not random
- Pauses before clicks (deliberation)
- Typing should have realistic rhythm (mistakes, corrections, thinking pauses)

---

## Implementation Plan

### Step 1: Data Preparation (1h)
- Load 534K events from behavior_log.jsonl
- Tokenize to feature vectors (48-dim)
- Create train/val/test splits
- Normalize pause times (log scale)

### Step 2: Model Training (2-3h, includes GPU wait)
- Instantiate LSTM (128-dim, 1 layer)
- Train on 534K sequences of length 20
- Monitor: train loss, val loss, perplexity
- Save checkpoint every epoch

### Step 3: Evaluation (30min)
- Test set prediction accuracy
- Perplexity on unseen sequences
- Qualitative: sample 10 sequences, inspect for human-likeness

### Step 4: Integration (1h)
- Wrap trained model in `BehaviorSimulator` class
- Add `sample(context, intent)` method
- Package as MCP tool for agent

### Step 5: Validation (1h)
- Agent executes behavior-guided sequence in browser
- Capture mitmproxy logs: does it look human?
- Compare pause distributions: generated vs. original

**Total:** ~6 hours (including GPU training time)

---

## Why This Unblocks May 10

Current Organ X gap: Agent browsing is hard to orchestrate without triggering bot detection.

With behavior ML:
1. Agent reads domain task ("explore Solana token security papers")
2. Model generates realistic clicking/scrolling/typing
3. Hermes browser executes the sequence
4. Passive capture (mitmproxy) observes results
5. Observations feed kernel → kernel judges → loop closes

**By May 10:** "Organism reflected on observations guided by learned human behavior" becomes possible.

---

## Data Integrity Check (Empirical)

**Hypothesis:** If behavior data is human, pause distribution should be bimodal:
- Fast pauses (<100ms): mechanical (mouse-to-click, keystroke-to-keystroke)
- Slow pauses (100-2000ms): deliberation (reading, deciding, searching)

**Test:** Histogram of 10K pause deltas.

**Expected falsification:** If unimodal or uniform → data is bot-generated or corrupted.

---

## Post-Hackathon Extensions

1. **Multi-site training:** Mix X.com + GitHub + Google search behavior → domain-agnostic model
2. **Intent conditioning:** Model takes (context, intent, domain) → more focused generation
3. **Adversarial validation:** Feed generated sequences to bot detector (Cloudflare, X.com ML)
4. **Distillation:** Compress 128-dim LSTM → 32-dim for edge inference (agent on lower-power machines)

---

*Design: 2026-05-01 18:50 CEST, ready for implementation phase May 2-6.*
