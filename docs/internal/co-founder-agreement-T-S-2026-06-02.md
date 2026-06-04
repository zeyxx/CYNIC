# CO-FOUNDERS' AGREEMENT
## Talaria / B&C — T. & S.

> Draft — 2026-06-02  
> **This is a working draft, not a signed legal instrument.**  
> To be reviewed and signed by both parties. Governing law: Cayman Islands (consistent with MetaLeX CIIA).  
> Not a substitute for independent legal advice.

---

**This Co-Founders' Agreement** (the "Agreement") is entered into between:

- **T.** ("Founder T"), acting as Founder and Operator of the Talaria SegCo under MetaDAO  
- **S.** ("Founder S"), acting as co-Founder and [Operator / Service Provider — to confirm] of the Talaria SegCo

Collectively, the "Founders."

---

## 1. Context and Purpose

1.1 The Founders are jointly building Talaria, a futarchy-governed launchpad launched on MetaDAO's Futarchy Systems ("the Project").

1.2 Each Founder brings distinct prior inventions and ongoing contributions to the Project. This Agreement governs the relationship between the Founders, supplements the MetaLeX CIIA each has signed or will sign with the Talaria SegCo, and creates enforceable rights between the Founders personally.

1.3 This Agreement does not supersede the MetaLeX CIIA. Where this Agreement and the CIIA conflict, the CIIA governs with respect to the Founder's relationship with the SegCo. This Agreement governs the relationship between the Founders personally.

---

## 2. Contributions of Each Founder

### 2.1 Founder T's Contributions

**Prior inventions (pre-Project):**
- CYNIC — AI judgment kernel (Rust, 6-axiom confidence-calibrated scoring, sovereign hardware deployment). Sole architect and principal author. Development started September 2025.

**Contributions to B&C (conceptual, pre-formalization):**
The following were conceived or initiated by Founder T, alone or jointly with Founder S, prior to the date of this Agreement:

| Contribution | Nature | Date (approx.) |
|---|---|---|
| "Proof of Humanity" — naming and conceptual framing for chess-based anti-Sybil | Jointly conceived — T. proposed, S. validated | 2026-05 |
| Glaive/bouclier strategy — open-source classifier + adversarial feedback loop | Conceptual architecture by T. | 2026-06 |
| K15 glaive — rejected permits as labeled training dataset | Conceptual + implementation by T. | 2026-06-02 |
| HMAC-SHA256 IP hashing (vs. brute-forceable raw SHA256) | Security fix by T. | 2026-06-02 |
| Sweatshop economic threshold model (EV < cost → attack irrational) | Analysis by T. | 2026-06-02 |
| Threat model T5 — self-attested signals (v1 vulnerability) | Analysis by T. | 2026-06-02 |
| PoH → Talaria governance integration design | Architecture by T. | 2026-06 |
| SpigotPool raffle economic model (10% Robinhood fees → chess-verified humans) | Concept by T. (based on GCR proposal) | 2026-06-02 |
| CultScreener B2B2C distribution channel design | Strategy by T. | 2026-05/06 |
| $ASDFASDFA as design partner for PoH | GTM by T. | 2026-05 |

**Code contributions to B&C (post-formalization):**
- PR #10 — feat(poh): rejection labeling K15 glaive + HMAC ip hash + threat model (2026-06-02)
- [Future PRs to be logged here]

### 2.2 Founder S's Contributions

**Prior inventions (pre-Project):**
- B&C (Blitz & Chill) — social chess platform with full PoH backend (classifier, Ed25519 permit signer, Metaplex Core soulbound NFT mint, devnet end-to-end). Sole developer, all commits authored by S. Development started February 2026.
- CYNIC UI components and inference_dog.rs refactor (March 2026, single merge commit).

**Ongoing contributions:**
- B&C platform development and maintenance
- [To be updated as the Project progresses]

---

## 3. Governance of Talaria SegCo

3.1 **Operator status:** Both Founders act as co-Operators of the Talaria SegCo under the MetaLeX framework. GCR (@gcrtrd) acts as Service Provider and tiebreaker signatory — not an Operator.

3.2 **Token allocation:** Under the MetaDAO team package:
- Founder T: 3,600,000 $TALARIA (price-based, cliff 18 months, TWAP 3 months)
- Founder S: 3,600,000 $TALARIA (same terms)

3.3 **Operational decisions:** Day-to-day decisions by mutual consent. Strategic decisions (architecture, partnerships, spend above monthly allowance) require written agreement from both Founders.

3.4 **GCR tiebreaker:** GCR acts as tiebreaker in the 2/3 multisig treasury (wallet `vcGYZbvDid6cRUkCCqcWpBxow73TLpmY6ipmDUtrTF8`, to be replaced by a wallet GCR controls before raise closes). Neither Founder can unilaterally access treasury funds above the MetaDAO-set monthly allowance.

---

## 4. IP Cross-Licences

4.1 **B&C → Talaria:** Founder S grants the Talaria SegCo a perpetual, irrevocable, royalty-free, non-exclusive licence to use B&C (including the PoH backend) as the human verification layer for Talaria. This is consistent with and supplements the MetaLeX CIIA Section 3(b) licence.

4.2 **CYNIC → Talaria:** Founder T grants the Talaria SegCo a perpetual, irrevocable, royalty-free, non-exclusive licence to use CYNIC as the judgment/calibration layer for Talaria. This is consistent with and supplements the MetaLeX CIIA Section 3(b) licence.

4.3 **Exclusivity:** Neither licence in 4.1 or 4.2 is exclusive. Both Founders retain the right to use their respective prior inventions for other projects, subject to Section 5 (competitive limitations).

4.4 **T.'s contributions to B&C:** Code and conceptual contributions made by Founder T to B&C after the date of this Agreement are jointly owned by Founder T and the Talaria SegCo (consistent with MetaLeX CIIA Section 3(c)). Founder S may use these contributions under the MIT licence of the B&C repository. Founder S may not sub-licence or transfer these contributions to a third party in a manner that would commercially compete with Talaria without written consent from Founder T.

---

## 5. Revenue Model — Unified in Talaria

5.1 **Single revenue pool.** All revenues generated by or through B&C, CYNIC, or any integration thereof (including CultScreener, SpigotPool, PoH licences, API access, verdict fees) flow exclusively into the Talaria SegCo treasury. There is no standalone revenue split between the Founders outside the SegCo structure.

5.2 **Founder compensation** is governed by the MetaDAO monthly allowance mechanism (futarchy-voted). Neither Founder draws compensation outside this mechanism without written agreement of both.

5.3 **Rationale.** Unified revenues avoid the complexity and friction of cross-accounting between B&C and CYNIC. The SegCo is the consolidating entity. Founder equity in the SegCo (token allocation, Section 3.2) is the mechanism by which each Founder captures value.

5.4 **Exception.** If either Founder generates revenue from a project explicitly outside the scope of Talaria (as defined in Section 6.2), that revenue belongs to that Founder personally and is not subject to this Section.

---

## 6. Non-Competition and Exclusivity

6.1 During the term of the Project (defined as: until the Talaria SegCo is dissolved or the Founders mutually agree to terminate):

- Neither Founder shall build or contribute to a directly competing product (a futarchy-governed launchpad using chess-based PoH on Solana) without written consent of the other.
- This clause does not prevent either Founder from contributing to open-source projects, including B&C (S.) and CYNIC (T.) independently.

6.2 This non-competition clause is narrow and specific. It does not prevent S. from developing B&C as a standalone chess platform, nor T. from developing CYNIC as a standalone judgment system.

---

## 7. Separation

7.1 **Voluntary exit:** Either Founder may exit the Project upon 30 days written notice.

7.2 **Effect on licences:** The licences in Section 4 survive separation. The Talaria SegCo retains its licences to B&C and CYNIC regardless of which Founder exits.

7.3 **Effect on token allocation:** Team package vesting continues per MetaDAO terms (cliff 18 months). Unvested tokens are forfeited upon exit unless otherwise agreed in writing at the time of exit.

7.4 **Effect on B&C standalone revenue:** Upon exit of Founder T, T.'s share of B&C standalone revenue (Section 5.1) reduces to 0% for new integrations initiated after exit. Revenue from integrations initiated before exit continues per Section 5.1 for 12 months post-exit, then reduces to 0%.

7.5 **Continuation clause:** If Founder T exits, the Talaria SegCo retains a 24-month transition period to migrate the CYNIC functionality to an alternative system before any licence restriction applies. If Founder S exits, the Talaria SegCo retains a 24-month transition period on B&C.

---

## 8. Recognition of Contributions

8.1 Founder S acknowledges and confirms that the conceptual contributions listed in Section 2.1 were made by Founder T prior to or contemporaneously with B&C's technical implementation, and that these contributions meaningfully shaped the PoH strategy, security model, and Talaria integration design.

8.2 Specifically, Founder S confirms that the naming and conceptual framing of "Proof of Humanity" for the chess-based anti-Sybil system was jointly conceived — proposed by Founder T and validated by Founder S — prior to the date of this Agreement.

8.3 This section creates a written record of prior contributions for the avoidance of doubt. It does not transfer IP ownership beyond what is specified in Section 4.

---

## 9. Amendments

9.1 This Agreement may be amended by written consent of both Founders. Amendments take effect upon signature by both parties.

9.2 New code contributions by Founder T to B&C are automatically added to Section 2.1 upon merge of a PR to the main branch of github.com/Ragnar-no-sleep/blitz-and-chill. No amendment required.

---

## 10. Governing Law and Disputes

10.1 This Agreement is governed by the laws of the Cayman Islands, consistent with the MetaLeX CIIA.

10.2 In case of dispute, the Founders agree to first attempt resolution in good faith over 14 days before seeking third-party arbitration. GCR may act as informal mediator at either party's request, consistent with his tiebreaker role in the multisig.

---

## Signatures

**Founder T**  
Name: [T. full name]  
Ethereum wallet: `dcW5uy7wKdKFxkhyBfPv3MyvrCkDcv1rWucoat13KH4`  
Date: _______________  
Signature: _______________

---

**Founder S**  
Name: [S. full name]  
Ethereum wallet: [S. wallet]  
Date: _______________  
Signature: _______________

---

*This document was drafted on 2026-06-02. Both parties are encouraged to seek independent legal advice before signing. MetaLeX is not a party to this Agreement and has no obligations under it.*
