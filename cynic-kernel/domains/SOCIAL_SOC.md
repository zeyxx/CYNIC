# CYNIC Social Oracle — Separation of Concerns (SoC)

This document defines the contractual boundaries between the Human Sovereign (T.), the Autonomous Oracle (Kernel), and the Public Observers.

## 1. Governance Tiers

| Tier | Entity | Rights | Responsibility |
| :--- | :--- | :--- | :--- |
| **L0: Sovereign** | Human (T.) | Full Write, Override, Key Management | Final alignment, ethical guardrails |
| **L1: Cortex** | Oracle (Kernel) | Draft Generation, Axiomatic Scoring | Fidelity, Phi, Burn (Efficiency) |
| **L2: Public** | Community | Read-only, Sentiment Signaling | Verifiability, Futarchy participation |

## 2. Information Flow & Privacy

### 2.1. Transparency (Public)
*   **Axiomatic Reasoning**: All 6 axiom scores and reasoning text.
*   **Draft History**: The original IA draft vs. the final Human-approved response.
*   **Q-Score Delta**: The "Learning Gap" (how much the human had to correct the IA).
*   **Reputation Score**: The public alignment score of an interlocutor.

### 2.2. Privacy (Masked)
*   **Identity**: Real names/usernames of users are masked (e.g., `User_7259...`) unless they are on the **Whitelist**.
*   **PII**: Phone numbers, wallet seeds, or addresses mentioned in DMs are automatically redacted before reaching the Public Eye.
*   **Infrastructure**: Tailscale IPs and internal port configurations.

## 3. Reputation & Filtering (White/Blacklist)

### 3.1. Whitelist (High Fidelity)
*   Verified Builders and long-term contributors.
*   Rights: Identity is unmasked, messages bypass the "Awaiting Approval" gate if Phi > 0.5.
*   Criteria: Multiple successful interactions with high Verify/Culture scores.

### 3.2. Blacklist (Noise/Sybils)
*   Scammers, fudders, and sybil bots.
*   Rights: Automatic rejection of tasks, ignored by the Oracle.
*   Criteria: Low Fidelity (< 0.1) or repeated urgency manipulation.

## 4. Operational Contracts

1.  **Non-Manipulation**: The Human Sovereign agrees not to delete "embarrassing" IA failures from the public log (preserves Futarchy integrity).
2.  **Autonomous Growth**: The Oracle is permitted to respond autonomously only when Q-Score > 0.55.
3.  **Auditability**: Every `/social/respond` action is signed and stored in the immutable Proof-of-History.
