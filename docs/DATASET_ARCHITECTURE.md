# CYNIC Dataset Architecture for Specialized LLM Training

To build a "Specialized LLM" (Cortex-Sovereign), we must move beyond raw logs and structure our data into a **Refinement Pipeline**.

## 1. Hierarchy of Storage

| Tier | Path | Purpose | Format |
| :--- | :--- | :--- | :--- |
| **L0: RAW** | `~/.cynic/organs/telegram/messages.db` | Immutable source of truth (SQLite). | Raw SQL |
| **L1: REFINED** | `cynic-python/datasets/telegram/signals_v1.jsonl` | LLM-extracted signals, cleaned. | JSONL |
| **L2: TRAINING** | `cynic-python/datasets/training/instruct_v1.jsonl` | Human-verified QA or reasoning chains. | JSONL (ChatML) |

## 2. Proposed JSONL Format for Training (ChatML Compatible)

For the specialized LLM to learn the "CYNIC style" (aphoristic, cynical, axiomatic), the dataset must follow this structure:

```json
{
  "id": "tg_block_12345",
  "source": "telegram_dm",
  "domain": "community",
  "messages": [
    {"role": "user", "content": "How do you define Fidelity in this protocol?"},
    {"role": "assistant", "content": "Fidelity is not about blind obedience to data, but the structural integrity between a signal and its empirical root. φ⁻¹ (61.8%) is the boundary where certainty ends and dogma begins."}
  ],
  "metadata": {
    "axioms": {"fidelity": 0.618, "phi": 0.5},
    "verdict": "WAG",
    "learned_weights_version": "2026-05-04"
  }
}
```

## 3. Specialized Datasets per Use-Case

1.  **Axiomatic reasoning (`cynic-logic`):**
    *   *Input:* A controversial tweet or signal.
    *   *Target:* A multi-axiom breakdown with scoring.
2.  **Community Engagement (`hermes-support`):**
    *   *Input:* User DMs or Telegram questions.
    *   *Target:* Helpful but "Dog-like" (faithful, protective) responses.
3.  **Alpha Extraction (`alpha-miner`):**
    *   *Input:* Raw Telegram/X feed noise.
    *   *Target:* Clean JSON signal extraction (Ticker, Direction, Context).

## 4. Maintenance (Ouroboros Loop)

The LLM specialized for CYNIC will be trained via **Self-Correction (DPO/RLHF)**:
- Hermes generates a response.
- T. (Human) provides a "GROWL" (bad) or "HOWL" (good).
- The interaction is added to `training/instruct_v1.jsonl`.
