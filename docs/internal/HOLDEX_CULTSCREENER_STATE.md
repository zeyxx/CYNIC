# HolDex & CultScreener : État des Lieux et Historique

Ce document sert de référence interne pour l'équipe technique et les agents autonomes travaillant sur l'écosystème ODYSSE/CYNIC, afin de clarifier la nomenclature entre HolDex et CultScreener.

## Nomenclature et Historique

### 1. CultScreener (Nom du Dépôt Interne)
- **Rôle :** C'est le dépôt GitHub principal et le nom interne du projet actuel.
- **État :** En production / Traction active.
- **Technologie :** Code optimisé pour limiter les appels RPC, conçu pour filtrer et afficher les tokens communautaires de manière efficiente.

### 2. HolDex (Nom Public Actuel)
- **Rôle :** C'est la marque publique (front-end) du projet CultScreener.
- **Raison du changement :** Le mot "Cult" subit des restrictions de visibilité et de shadowban sur X (Twitter). Le projet a donc été rebrandé publiquement en "HolDex" pour contourner ces filtres algorithmiques et maximiser l'adoption.

### 3. L'ancien "HolDex" (Alpha Obsolète)
- **Rôle :** Une ancienne itération/alpha du projet.
- **État :** Obsolète / Dépréciée.
- **Raison de l'abandon :** L'architecture originelle était beaucoup trop coûteuse en requêtes RPC sur Solana, rendant le passage à l'échelle impossible économiquement. Le code actuel de `CultScreener` résout ces problèmes.

## Impact sur les communications (Levée Futard.io)
Pour le pitch et toute communication externe (Futard.io, X), nous utilisons **exclusivement le nom HolDex**.
Pour le développement interne, les commits, et les intégrations CI/CD, le dépôt de référence technique reste **CultScreener**.
