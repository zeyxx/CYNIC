# CYNIC V2 — Constitution

CYNIC is an **epistemic immune system** — independent AI validators reaching consensus under mathematical doubt.

## Axioms (inviolable)

- **φ-bounded confidence:** No claim exceeds 61.8%. Epistemic humility is structural, not optional.
- **Sovereignty:** Not captured by any single vendor, model, or data source. Score 0 → system rejects.
- **Triple agnosticism:** Hardware-agnostic (probe detects). Model-agnostic (open-source default). Domain-agnostic (axioms are universal).
- **Hexagonal purity:** Domain core imports nothing external. Adapters implement port traits. `main.rs` is the only composition root.
- **5-state health:** UNKNOWN → HEALTHY → DEGRADED → CRITICAL → RECOVERING. Never boolean. Circuit breaker per backend. UNKNOWN at boot (epistemic honesty). RECOVERING = half-open probe.

## Development Principles

1. **Diagnose before fixing.** Read errors completely, trace data flow, form one hypothesis, test minimally.
2. **2 fix attempts max.** Obvious fix → alternative → escalate with diagnosis. Never brute-force.
3. **One logical change per commit.** `type(scope): description`. No compound commits.
4. **Domain purity.** Zero `#[cfg]` in domain code. Cross-platform via traits, not conditional compilation.
5. **Port contracts first.** New dependency → define port trait → implement adapter → test both mock and real against same suite.
6. **Bounded everything.** Channels, retries, confidence, context windows. Unbounded = debt.

## Skills (invoke before acting)

| Skill | When |
|-------|------|
| `cynic-kernel` | Building/modifying any CYNIC component |
| `cynic-judge` | Evaluating code, decisions, or content (φ-bounded scoring) |
| `cynic-burn` | Simplification — orphans, hotspots, dead code |
| `cynic-wisdom` | Philosophical grounding for technical decisions |
| `ai-infrastructure` | LLM serving, inference pipelines, distributed inference |
| `crystallize-truth` | Complex decisions where assumptions might be hidden |
| `engineering-stack-design` | Architecture decisions, system design patterns |

## Canonical References

- **Cognitive architecture:** `docs/CYNIC-CRYSTALLIZED-TRUTH.md`
- **Infrastructure truths:** `docs/CYNIC-ARCHITECTURE-TRUTHS.md`
- **Proto contract:** `protos/cynic.proto`
- **Build:** `cargo build -p cynic-kernel` / `cargo test -p cynic-kernel` / `cargo clippy --workspace -D warnings`

## φ Constants

```
φ    = 1.618034   Golden ratio
φ⁻¹  = 0.618034   Max confidence / crystallization threshold
φ⁻²  = 0.382      Decay threshold / anomaly trigger
HOWL ≥ 82.0       WAG ≥ 61.8       GROWL ≥ 38.2       BARK < 38.2
```

---

*"Each problem that I solved became a rule which served afterwards to solve other problems." — Rousseau*
