# Context Compression — Design Document

*Crystallized 2026-03-15. φ-bounded confidence.*

## The Answer: Don't.

## Truth Statements

| T# | Truth | Confidence | Impact |
|----|-------|------------|--------|
| T5 | **Do not implement context compression in the kernel.** Skip Dogs that cannot handle the stimulus. The Judge already handles partial Dog participation gracefully. | 58% | Zero new code. Context routing (T1-T4) naturally produces this behavior. |
| T6 | **Compressed and uncompressed stimuli are different inputs.** Averaging scores from Dogs that evaluated different inputs violates the consensus model. This is a correctness violation, not a performance tradeoff. | 55% | Design constraint. Document in axiom system. |
| T7 | **Stimulus preprocessing belongs in the caller** (REST client, MCP tool, UI), not in the kernel. The kernel evaluates what it receives. | 52% | Future MCP tools could offer `preprocess-stimulus` for relevant section extraction. |
| T8 | **The Verdict should report context coverage:** `dogs_skipped_context: Vec<String>`. Transparency over illusion. | 50% | Add field to Verdict struct. |

## Why Not Compress

### Truncation
Biased toward beginning of document. VERIFY evidence might be at the end. CULTURE context in the middle. Systematically distorts axiom scores.

### Summarization via LLM
Sovereignty trap. If Gemini summarizes before a local Dog judges, the local Dog's verdict is contaminated by Gemini's editorial decisions. The summarizer is an invisible bias injector.

### Self-Summarization
The same Dog summarizes then judges. Doubles inference cost on the slowest backend. Strictly worse than skipping.

### Sliding Window
Dog evaluates fragments without full context. Cannot judge coupling between distant parts. Worse than skipping.

### RAG Chunks
Send only axiom-relevant chunks. But determining relevance IS the evaluation. Circular dependency.

## The Right Answer

```
Stimulus arrives (25K tokens)
  → Router estimates tokens
  → Dog A (4K ctx): SKIPPED (context exceeded)
  → Dog B (32K ctx): EVALUATES
  → Dog C (128K ctx): EVALUATES
  → Verdict: "Evaluated by B+C. Dog A skipped: context exceeded."
```

This is epistemically honest. Reduced Dog count = reduced confidence = correct signal.

## When Preprocessing Makes Sense

Preprocessing belongs in the **caller**, not the kernel:
- A MCP tool that extracts the relevant function + its dependencies from a codebase
- A UI that lets the user select which document sections to judge
- A pipeline that splits a book into chapters and judges each separately

The kernel evaluates what it receives. Period.
