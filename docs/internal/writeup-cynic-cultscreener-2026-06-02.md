# CYNIC × CultScreener — Writeup B2B

> À envoyer à GCR après le thread chess/raffle. Sujet distinct.
> Format : message Telegram direct, pas un doc formel.

---

## Message à envoyer

```
Separate topic — the CYNIC layer for CultScreener.

Two things CYNIC can add to every community listed on CultScreener:

---

**1. Proposal judgment**

Before a community vote goes live, run it through CYNIC.
6 axioms scored independently: is it sound? verifiable? does it concentrate power?
Verdict: BARK / GROWL / WAG / HOWL / EPOCHÉ — recorded on Solana.

Practical use: a community proposes "burn 10% of supply and redirect fees to treasury."
CYNIC judges it. Dogs flag "no quorum mechanism defined" (GROWL).
CultScreener shows the verdict alongside the proposal.
Voters see an independent second opinion before they decide.

---

**2. Wallet behavioral analysis**

This is already running in CYNIC for every token we score.
We measure:

- Diamond hands: are long-term holders actually holding or bag-holding?
- Accumulator ratio: is the buy pressure organic growth or insider accumulation before a dump?
- Organic growth: is the holder distribution healthy?
- K-Score: composite longevity × growth signal

For CultScreener: instead of showing "100K holders", show what those holders
actually DO. A community with 1M holders and 0.8 accumulator ratio
is worse than one with 50K holders and 0.3 accumulator ratio.

We built this from on-chain data (Helius). It works now.

---

**Revenue model**

Simple: CYNIC charges per judgment call.
Pay in USDC or $TALARIA (20% discount with TALARIA, burns on use).
You take a cut as the distribution layer.

Your communities get signal they can't get elsewhere.
We get distribution into the ecosystem we're trying to serve.

---

No pitch deck. The code is at github.com/zeyxx/CYNIC.
Want to test it on a real community this week?
```

---

## Notes internes (ne pas envoyer)

**Ce qui est construit et fonctionne :**
- Token analysis domain : K-Score, diamond hands, accumulator ratio, organic growth → `cynic-kernel/src/dogs/deterministic/mod.rs`
- Governance domain : jugement de propositions sur 6 axiomes → `domains/governance.md` (déployé aujourd'hui)
- Verdict on-chain : programme Solana devnet `AKjCbxzdjXHcTmTqN37K7eZM2RUsCYTmaXUriTd6csBH`
- Demo live : `https://demo.talaria.build`

**Ce qui n'est pas encore construit :**
- x402 payment (identifié, doc en mémoire)
- Dashboard CultScreener-specific
- API contract formel

**Argument honnête pour GCR :**
Le wallet behavioral analysis est différenciateur — CultScreener fait du diamond hands tracking,
CYNIC fait le même travail mais avec 6 dimensions dont accumulator ratio (que CultScreener
ne montre pas et qui est le signal le plus prédictif selon notre étude n=30 tokens, rho=-0.622).
