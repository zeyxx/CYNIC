# Demo UI — rendre φ⁻¹ visible et culturel — 2026-06-02

## Principe

Ne pas expliquer φ⁻¹ — le montrer en action dans les données réelles.
Le tooltip `?` reste pour ceux qui veulent le fond. L'UI montre le comportement.

## Changements concrets

### 1. Plafond visuel sur les barres d'axiomes

Ajouter un marqueur vertical à la position 100% de la barre (= φ⁻¹ = 0.618).
La barre ne peut jamais le dépasser — c'est visible mécaniquement.
Label minimal : `φ⁻¹` au bout de chaque barre.

Avant : barre qui remplit jusqu'à un % de la largeur totale.
Après : barre dans un container borné, avec un repère vertical "φ⁻¹" à droite.

### 2. Raw score vs bounded score — quand ils diffèrent

L'API retourne `raw_fidelity` etc. en plus de `fidelity`.
Si raw > 0.618 (le Dog voulait scorer plus haut, le kernel a capé) :
Afficher sous le score : `↓ from {raw:.3} (capped)`

Ce cas est rare mais puissant quand il arrive — montre l'enforcement mécanique en direct.

### 3. EPOCHÉ — reformulation

Actuellement : "Suspendu — Dogs en désaccord"
Nouveau : header fort + explication en 1 ligne :

```
EPOCHÉ
The Dogs disagree past the threshold.
No verdict forced. Judgment suspended.
```

Ajouter l'axiome du désaccord : "Disagreement detected on: phi (Δ=0.43)"

### 4. Header "About this score" discret

Juste avant les barres d'axiomes, une ligne :
`Scores are bounded at φ⁻¹ = 0.618 — the golden ratio inverse.
No verdict can exceed this threshold, regardless of input.`

Petit, muted, toujours là. Pas un tooltip — une déclaration permanente.

## Ce qu'on NE fait pas

- Pas d'animation, pas d'explications longues
- Pas de changer le tooltip existant
- La phrase ne doit pas être plus longue que 2 lignes
