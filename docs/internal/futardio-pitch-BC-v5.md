<!-- RECONSTRUCTED 2026-05-29 — Original lost in home directory reset. Content approximate. -->

# Pitch B&C — v5

**Itération sur v4 : fusion des angles, preparation pour unified v6**

---

## Changements par rapport à v4

1. Fusion explicite CYNIC (tech) + B&C (business) — plus de séparation
2. Ajout section compétitive détaillée
3. Simplification du modèle éco
4. Réponses aux objections anticipées

---

## Positionnement unifié

**B&C n'est pas un wrapper autour de CYNIC.** C'est la même infrastructure vue sous deux angles :

- **CYNIC** = le nom du moteur de jugement (open source, Rust)
- **B&C** = l'entité commerciale qui opère, vend, et supporte

Analogie : Linux (kernel open source) → Red Hat (entreprise qui vend le support)

---

## Réponses aux objections

### "C'est open source, on peut copier"

Le moat n'est pas le code, c'est :
1. Les cristaux (mémoire accumulée, 20K+ verdicts)
2. L'équipe qui sait opérer le système
3. La philosophie (phi-bounded, EPOCHÉ) qu'il faut comprendre pour modifier
4. Le dataset Hermes (intelligence sociale unique)

### "Pas d'utilisateurs"

Vrai. Mais :
- 20K verdicts en production = le système fonctionne
- La cible initiale est B2B (API), pas B2C
- Premier client = nous-mêmes (dogfooding)

### "Pourquoi Rust ?"

- Performance : jugement en <100ms
- Fiabilité : type system empêche les bugs critiques
- Souveraineté : pas de runtime, pas de GC, contrôle total
- Signal : le choix de Rust = sérieux technique

### "Pourquoi pas cloud ?"

- Souveraineté = valeur fondamentale du projet
- Coût : ~200€/mois vs 2000€+/mois sur AWS
- Contrôle : les modèles IA tournent localement
- Crypto-natif : la communauté valorise l'auto-hébergement

---

*Avant-dernière itération — voir unified v6 pour la version finale.*
