<!-- RECONSTRUCTED 2026-05-29 — Original lost in home directory reset. Content approximate. -->

# Futardio Pitch — Draft Initial (2026-05-28)

**Statut :** Brouillon — remplacé par les versions spécialisées (CYNIC v3, B&C v3-v5, unified v6)

---

## Structure du Pitch (5 min)

### 1. Hook (30 sec)

"50 000 nouveaux tokens par jour sur Solana. 98% sont des arnaques. Les outils existants te montrent des chiffres. Personne ne te dit ce qu'ils veulent dire."

### 2. Solution (2 min)

CYNIC = moteur de jugement. Pas un dashboard, un juge.

- 3 modèles IA indépendants
- Consensus phi-bounded (max 61.8%)
- Verdict : HOWL (fiable) → BARK (danger) → EPOCHÉ (indécidable)
- Zéro cloud, hardware propre

### 3. Demo (1 min)

```bash
curl -X POST cynic:3030/judge -d '{"content":"<mint>","domain":"token-analysis"}'
```

### 4. Équipe (1 min)

- T. : CYNIC kernel (Rust), HolDex
- S. : B&C (TypeScript), business
- G. : ASDelegate, HolDex, CultScreener
- Heritage : 3000+ commits

### 5. Ask (30 sec)

- Partenaires techniques
- Beta testeurs
- Connexions investisseurs

---

## Notes d'itération

- Trop long pour 5 min → couper
- Intégrer B&C plus tôt
- Ajouter angle "agents IA"
- Préparer version simplifiée pour non-techniques

---

*Remplacé par versions v3+ — garder pour historique.*
