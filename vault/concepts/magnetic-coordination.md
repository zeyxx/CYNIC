# Magnetic Agent Coordination & Thermodynamics

## Intuition & Emergence

Agents operating in shared worktrees face two recurring pathologies:
1. **Collisions**: Overlapping claims and accidental overwrite during high-velocity parallel work.
2. **Context Stuffing**: Dumping conversational histories to child agents, creating high thermal dissipation and cognitive fatigue (the Curie Point).

## The Physical Model

- **Alignment Field $\vec{B}$**: The parent process does not converse; it induces an orientation field (intent, invariants, output schema, monotonic budget).
- **Repulsion Potential $U(d)$**: Distance in the code tree generates topological repulsion. At $d \le 1$ (same file/module), repulsion is infinite for mutating agents.
- **Thermodynamic Phases**:
  - *High-$T$ Exploration*: Free movement, shared reading, zero mutation rights.
  - *Zero-$T$ Crystallization*: Spin-locked along $\vec{B}$, exclusion locks engaged, atomic commit and read-back proof.
- **Passive Demagnetization**: Expired TTL claims naturally decay without requiring human administrative intervention.

## Associated Formal RFC

- See `specs/RFC-001-MAGNETIC-COORDINATION.md` for the normative specification and mathematical bounds.
