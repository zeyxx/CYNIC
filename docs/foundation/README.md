# CYNIC Foundation — Architecture, Doctrine & Authority

Status: Informative Directory Index & Epistemic Authority Guide  
Authority: `contracts/foundation.json §constitutional.authority_order`, `docs/INDEX.md`  

---

## 1. Epistemic Hierarchy & Authority Order

Per `contracts/foundation.json`, truth in CYNIC is anchored bottom-up:

```text
1. Observed runtime state (hardware, raw bytes, exit codes, machine receipts)
2. contracts/foundation.json & infra/runtime-authority.json
3. Typed Rust contracts & canonical workspace crates (crates/)
4. Normative Architectural Prose (CONSTITUTION.md, CRATE_RULES.md)
5. Philosophical Doctrine & Speculative Research (manifesto/, whitepaper/)
```

**Rule**: Prose never overrides executable code or machine contracts. A capability declared in prose without an executing test or service read-back is an explicit hypothesis or non-normative projection.

---

## 2. Directory Structure & Document Status

The `docs/foundation/` directory organizes the philosophical roots, normative laws, and architectural horizon of the CYNIC organism into three distinct tiers:

### Tier I: Normative Governance & System Laws (Active Law)
Directly enforced by compiler boundaries, lint gates, and the `cynic-audit` suite:

- **[`CONSTITUTION.md`](./CONSTITUTION.md)** : The foundational separation of powers (Constituent Protocol L0, Judicial Kernel L1, Executive Node/Agents L2). Establishes the human as the sole constituent root.
- **[`CRATE_RULES.md`](./CRATE_RULES.md)** : Structural invariants for the Rust workspace (Functional Core / Imperative Shell, Zero-I/O in domain crates, downward DAG isolation).
- **[`DESIGN_PROTOCOL.md`](./DESIGN_PROTOCOL.md)** & **[`GOVERNANCE_MAP.md`](./GOVERNANCE_MAP.md)** : Operational standards for protocol design, telemetry, and change admission.

---

### Tier II: The Living Manifesto (`manifesto/`) — Philosophical Doctrine
The human and cultural foundation of CYNIC. It answers the **Why**: exiting zero-sum games (PvP), rejecting digital predation and societal masking, and coordinating as an open swarm (PvE):

- **[`manifesto/README.md`](./manifesto/README.md)** : General index of the 10 foundation books.
- **Key Themes**:
  - `00_PROLOGUE`: The grand theater, waking up to sovereign reality.
  - `03_HOMO_LUDENS`: Fun as a negentropy compass, the memetic origin of $ASDFASDFA.
  - `04_LE_SSOT_BLOCKCHAIN`: Solana as the physical time clock and immutable single source of truth.
  - `05_CYBERNETIQUE_DES_ESSAIMS`: Cybernetics of bounded autonomous agents (Beer, Wiener, Ashby).
  - `07_L_END_GAME_QUANTIQUE`: The pirate oath and the dual membrane (*Corporate in the front, cypherpunk in the back*).

*Status*: Philosophical doctrine. Non-binding on runtime state, but mandatory for strategic orientation.

---

### Tier III: The Protocol Canon (`whitepaper/`) — Mathematical & Architectural Horizon
The formal specifications, metrology, and long-term research roadmap. It explores the **How**:

- **[`whitepaper/README.md`](./whitepaper/README.md)** : Technical summary and mapping to the manifesto.
- **Epistemic Classification**:
  - `principle` : Core enduring design direction (e.g., POSIWID, Sowellian trade-offs, Proof-of-Work primacy).
  - `research` : Theoretical modeling (e.g., Leontief function G calibration, quadratic E-Score, Defcon-Local mesh).
  - `planned` : Architectural ambitions requiring dedicated crates (e.g., formal DAG verifiers, CASCIL distributed compute).

*Warning*: Concepts in this section (e.g., multi-token Ignis deflation, Lean 4 proofs) describe long-term research horizons. They must not be confused with physical production crates currently compiling in `crates/`.

---

## 3. Guiding Invariants

1. **Human Sovereignty**: The human mind is the constituent source of intent. No agent or algorithm may silently expand its own mandate.
2. **Iron Law**: *"No mutation without read-back proof."*
3. **Bottom-Up over Top-Down**: Physical substrate reality (hardware, slots, exit codes) always trumps speculative abstraction.
