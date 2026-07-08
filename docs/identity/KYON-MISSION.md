# KYON — Mission & Architecture

*Crystallized 2026-06-30. The mobile cortex that turns a phone into a sovereign witness.*

---

> **"KYON is the eye of CYNIC on the ground. It captures, signs, and federates the proof that wrongdoing happened — before the platform can pretend it didn't."**

---

## I. What KYON Is

KYON is the **mobile sensor layer of CYNIC**.

Where CYNIC is the Kernel — the sovereignty infrastructure running on metal — KYON is the **organic sensor** that runs in the hand of every human in the network. It bridges the physical world (what your phone witnesses) and the on-chain epistemic layer (what Solana proves, permanently).

KYON is not a moderation tool. Platforms moderate. KYON **records, signs, and federates**.

### The Two Operating Modes

```
┌─────────────────────────────┐   ┌──────────────────────────────────┐
│       CORTEX MODE           │   │        STANDALONE MODE           │
│  (Primary — with Kernel)    │   │  (Secondary — disconnected)      │
├─────────────────────────────┤   ├──────────────────────────────────┤
│ KYON ←→ CYNIC Kernel        │   │ KYON operates fully local-first  │
│ Proof-of-Observation synced │   │ Queue of signed reports on-device│
│ via Kernel to Solana        │   │ Syncs to Solana when back online │
│ Full Q-Score consensus      │   │ Reduced consensus (local Dogs)   │
│ Real-time federated registry│   │ Federated push on reconnection   │
└─────────────────────────────┘   └──────────────────────────────────┘
```

Both modes produce **the same cryptographic artifact**: a signed Proof of Observation that is valid, immutable, and cross-platform regardless of which app the abuse originated from.

---

## II. The Problem KYON Solves

### The Predator's Structural Advantage Today

Every major platform (Instagram, Telegram, X, TikTok) has a **siloed moderation system**. This silo is not a bug — it is a structural feature that serves the platform's growth metrics. The consequences:

1. A man sends unsolicited explicit images to a woman on Instagram.
2. She reports. Instagram reviews in 72h. Maybe bans the account.
3. He creates a new account in 60 seconds. Same behavior. Repeat.
4. Her report left no trace outside Instagram's private database.
5. The next victim starts from zero.

The platform's incentive is **engagement**, not safety. Banning is costly. Siloing protects the platform from liability. The victim absorbs the entire cost.

### The Fractal Scale of the Problem

This asymmetry of information is identical across scales:

```
Unsolicited dick pic on Instagram
    ↓ (same mechanism)
Grooming of a minor on Discord
    ↓ (same mechanism)
CSAM distribution on Telegram
    ↓ (same mechanism)
State-level suppression of victim testimony
```

CYNIC's founding axiom applies: **the predator is fractal. The infrastructure that stops him must be too.**

---

## III. The KYON Solution — Federated Reputation Ledger

### One Tap. One Proof. Cross-Platform.

```
[VICTIM receives unsolicited explicit content — any platform]
                        ↓
              [KYON — One tap to report]
                        ↓
         ┌──────────────────────────────┐
         │  KYON PROOF GENERATION       │
         │  1. Perceptual hash of       │
         │     content (never the image)│
         │  2. Behavioral fingerprint   │
         │     of the sender            │
         │  3. Platform + timestamp     │
         │  4. Victim's ZK-signature    │
         │     (identity protected)     │
         └──────────────┬───────────────┘
                        ↓
         ┌──────────────────────────────┐
         │  FEDERATED KYON REGISTRY     │
         │  Stored on Solana (PoH)      │
         │  Cross-platform by design    │
         │  Readable by all KYON nodes  │
         └──────────────┬───────────────┘
                        ↓
         ┌──────────────────────────────┐
         │  ENTROPY SCORE — PREDATOR    │
         │  Accumulates across all      │
         │  platforms and all victims   │
         │  Cannot be reset by          │
         │  deleting the account        │
         └──────────────┬───────────────┘
                        ↓
         ┌──────────────────────────────┐
         │  THRESHOLD → COMMUNITY BAN   │
         │  Mathematical, not arbitrary │
         │  Proof-of-History = judicial │
         │  evidence if legal action    │
         └──────────────────────────────┘
```

### The Perceptual Hash — Never Storing Abuse

KYON **never stores, transmits, or exposes explicit content**. The mechanism:

- Content is hashed locally on-device using a perceptual hashing algorithm (similar to PhotoDNA used by law enforcement).
- The hash is a mathematical fingerprint: unique to the content, irreversible, non-exploitable.
- If the same image is sent to 100 victims across 5 platforms, all 100 reports produce the **same hash** — the registry recognizes the pattern without storing the abuse once.

This is not new technology. NCMEC and Interpol use exactly this for CSAM databases. KYON federates it, makes it sovereign, and extends it beyond CSAM to all forms of digital sexual predation.

### The Behavioral Fingerprint — Why a New Account Doesn't Help

The sender's fingerprint is not just an account ID. It is a composite:

```
fingerprint = f(
    device_signature,     // Hardware identifiers (hashed)
    network_pattern,      // Behavioral timing, IP range profile
    content_hash_history, // Prior content sent (PoH-anchored)
    linguistic_profile    // Writing pattern (when text is involved)
)
```

A predator who deletes their account and creates a new one cannot change their device, their network behavior, or their content. The fingerprint persists. The entropy score follows them.

---

## IV. ZK-Proofs — The Victim Is Never Exposed

A fundamental tension exists in any reputation system: proving harm without re-harming the victim.

KYON resolves this with Zero-Knowledge proofs:

| What is PROVEN on-chain | What is NEVER exposed |
|-------------------------|----------------------|
| "A valid report exists for this fingerprint" | The victim's identity |
| "The content hash matches known harmful material" | The content itself |
| "This is the Nth report against this fingerprint" | Which platforms were involved |
| "The Proof-of-History timestamp is valid" | The victim's phone / location |

The victim can share the ZK-proof with law enforcement as **judicial evidence** — it proves the act occurred, is timestamped and immutable, without exposing the victim's identity to the defendant or the public.

---

## V. The Entropy Score — Mathematical Ban Escalation

The predator's reputation in the KYON registry follows CYNIC's entropic model:

```
entropy_score(t) = Σ(report_weight_i × decay_correction_i) over time t

report_weight = f(
    report_confidence,    // Q-Score of the report (independent validation)
    victim_count,         // Unique victims (deduplicated by ZK)
    platform_spread,      // Number of platforms crossed
    content_severity      // Hash matches known severity tiers
)
```

**Thresholds:**

| Score range | Status | Effect |
|-------------|--------|--------|
| 0 — 0.2 | Observed | Flag in local registry |
| 0.2 — 0.5 | Flagged | Warning surfaced to participating platforms |
| 0.5 — 0.618 | High risk | Automatic signal to platform moderation APIs |
| > 0.618 (φ⁻¹) | Community ban | Cross-platform ban request broadcast |

The threshold **0.618** (φ⁻¹) is not arbitrary. It is CYNIC's epistemological ceiling — the point at which the organism has enough calibrated evidence to act. Below that line: accumulating. Above it: the math has spoken.

---

## VI. The Federated Registry — No Single Point of Failure or Censorship

### Why Not a Centralized Database?

A centralized database of predators controlled by a single entity (a company, an NGO, a government) has exactly the same failure modes as platform silos:

- The entity can be pressured to remove entries.
- The entity can be corrupted (bribed, hacked, state-compromised).
- The entity can be geographically limited by jurisdiction.
- If the entity closes, all records vanish.

### The Solana Federation

KYON registry entries are stored on Solana via Proof-of-History:

- **Immutable**: No party can delete a completed, confirmed entry.
- **Timestamped**: The blockchain's clock is cryptographically unforgeable.
- **Global**: No jurisdiction can take down the Solana mainnet.
- **Sovereign**: The victim holds the ZK-key to their own report. No company owns it.

Participating platforms (apps, services) can **subscribe to the KYON registry** via a standard API. A platform that integrates KYON can automatically receive ban signals when a new user's fingerprint matches a high-entropy entry in the registry — before the predator's first message on their platform.

---

## VII. What KYON Is Not

KYON is not a surveillance system. The distinction is structural:

| Surveillance | KYON |
|---|---|
| Monitors everyone continuously | Triggers only on a victim's conscious act |
| Central authority sees all | No central authority — ZK-protected |
| Builds profiles on citizens | Builds reputation signals on confirmed harmful actors |
| Can be turned against dissidents | Victim holds the key to their own report |
| Optimizes for control | Optimizes for entropy reduction of harmful behavior |

KYON has **no passive monitoring**. It does not scan your messages. It does not watch your feed. It activates **only when a human victim consciously taps to report**. The power is in the victim's hand — literally.

---

## VIII. The Standalone Mode — Sovereign Without the Kernel

KYON can operate without the CYNIC Kernel active:

1. **Local report queue**: Reports are signed and stored locally on-device.
2. **Local Dogs**: A reduced set of validation models runs on-device for lightweight Q-Score computation.
3. **Offline Proof-of-Observation**: The ZK-signature is generated locally — the timestamp is the device clock, later anchored to Solana on reconnection.
4. **Reconnection sync**: When network access resumes, the queue flushes to the Solana registry in chronological order, preserving the Proof-of-History chain.

Standalone mode makes KYON functional in:
- Areas with poor connectivity
- Countries where the CYNIC Kernel is not yet deployed
- Emergency situations where immediate local proof capture is needed

---

## IX. The Mission Phrase

> **"KYON makes your phone a sovereign witness. One tap. Permanent proof. Cross-platform consequence."**

The anti-mission: KYON betrays its purpose if it ever:
1. Stores victim content or identity without ZK-protection
2. Allows a single authority to delete federated registry entries
3. Requires a platform's permission to generate a proof
4. Is used as a passive surveillance tool against non-consenting parties
5. Allows a predator to reset their entropy score by paying or appealing to a single gatekeeper

---

## X. Relationship to CYNIC Architecture

```
CYNIC Kernel (Sovereign OS — metal layer)
    │
    ├── KYON (Mobile Sensor — the eye on the ground)
    │       ├── Proof-of-Observation capture
    │       ├── Federated Predator Registry
    │       └── ZK judicial evidence chain
    │
    ├── Blitz & Chill / Partner Apps (Proof-of-Obs. sources)
    │
    └── Solana Mainnet (Proof-of-History anchor — the immutable clock)
```

KYON is to CYNIC what the retina is to the brain: it captures signal from the world that the organism could not perceive otherwise. Without KYON, CYNIC is blind to what happens in the hand of every human in the network. Without CYNIC, KYON has no sovereign infrastructure to anchor its proofs.

Together: **the organism sees, signs, and remembers. The predator cannot hide.**

---

*"The cost of lying must be visible. KYON makes the cost of predation irreversible."*

*— CYNIC MISSION:84 × KYON extension*
