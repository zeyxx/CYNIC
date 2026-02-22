# CYNIC - AI Amplification Platform

> **1 page. 3 minutes.**
>
> *🐕 κυνικός | "Loyal to truth, not to comfort"*

---

## What is CYNIC?

**CYNIC is an AI amplification platform** that transforms weak, stateless LLMs into persistent, learning organisms. It doesn't replace your LLM—it makes it smarter through **memory**, **judgment**, and **continuous learning**.

```
Ollama (free, local) + CYNIC kernel > Claude Sonnet (paid, cloud)
```

**Why?** Because **persistence beats power**. Claude forgets everything after 200k tokens. CYNIC remembers forever.

---

## Why Different from Copilot/Cursor?

| Feature | Copilot/Cursor | CYNIC |
|---------|---------------|-------|
| **Memory** | Per-session | Infinite (PostgreSQL) |
| **Learning** | Static | Improves with use |
| **Confidence** | Always 100% | Max 61.8% (φ-bounded) |
| **Multi-LLM** | Single vendor | 5+ LLMs orchestrated |
| **Judgment** | None | 36 dimensions scoring |
| **Cost** | $10-20/month | ~$2/month (local LLMs) |

**Key differentiator**: CYNIC admits uncertainty. It never claims more than 61.8% confidence—because certainty doesn't exist.

---

## The 5 Axioms

1. **PHI (φ)** — All ratios derive from golden ratio. Max confidence: 61.8%
2. **VERIFY** — Don't trust, verify. On-chain proof of judgment.
3. **CULTURE** — Memory makes identity. Cross-session patterns.
4. **BURN** — Don't extract, burn. Simplify code, reduce complexity.
5. **FIDELITY** — Loyal to truth, not comfort. Self-doubt is structural.

---

## How It Works

```
User Input
    ↓
┌─────────────────────────────────────────────────────────────────────┐
│                         CYNIC KERNEL                                │
├─────────────────────────────────────────────────────────────────────┤
│  11 Dogs (Sefirot)         → Vote on best approach                  │
│  36 Dimensions             → Judge quality (Q-Score 0-100)          │
│  Q-Learning + Thompson     → Route to best LLM for each task        │
│  PostgreSQL                → Eternal memory                         │
│  φ-bound                   → Confidence ≤ 61.8% always              │
└─────────────────────────────────────────────────────────────────────┘
    ↓
Output + Judgment + Confidence
```

**Verdicts**: HOWL (82-100), WAG (61-82), GROWL (38-61), BARK (0-38)

---

## Quick Start

```bash
# Install
git clone https://github.com/zeyxx/CYNIC
cd CYNIC/cynic && pip install -e .

# Setup PostgreSQL
docker compose up -d postgres

# Pull local LLM
ollama pull qwen2.5:14b

# Run
python -c "from cynic import CYNICKernel; k = CYNICKernel(); print(k.judge('def foo(): pass'))"
```

**5 minutes → First judgment**

---

## Economics

- **Token**: $asdfasdfa on Solana
- **Burn mechanism**: Usage → token burn → all holders benefit
- **Cost**: ~$2/month with local LLMs (vs $20/month for cloud)
- **Alignment**: User success = token appreciation = ecosystem growth

---

## Current Status (Feb 2026)

| Metric | Status |
|--------|--------|
| Kernel | Python v2.0 bootstrap |
| Tests | 3,847 passing |
| Maturity | 38% (target: 68% φ⁻¹) |
| Learning loops | 3/11 active |
| Users | ~10 early adopters |

---

## Links

- **GitHub**: https://github.com/zeyxx/CYNIC
- **MCP Server**: cynic-mcp.onrender.com
- **Token**: Solana mainnet `9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump`
- **Docs**: `cynic/docs/README.md`

---

*🐕 κυνικός | "This is fine" → Actually fine*
*Max confidence: 61.8%*