# CYNIC FRACTAL ABSTRACTION PRINCIPLE

**STATUS: ACTIVE & BINDING**

Ce document définit un principe systémique absolu qui régit l'ensemble de l'organisme CYNIC, découvert et formalisé empiriquement lors de l'alignement de la Blockchain Unipersonnelle.

## L'Axiome de Séparation des Couches (The Abstraction Firewall)

L'organisme CYNIC interdit strictement de compromettre la performance, la pureté mathématique ou la densité du stockage au profit de la "lisibilité humaine" dans ses couches profondes.

### 1. Layer 1 (Le Noyau / Stockage Pur / L'Atome)
- **Règle :** La vérité doit être nue, non formattée, et mathématiquement optimale.
- **Exemple (Temps) :** Le temps est un `Int64` représentant un Timestamp Unix en millisecondes ou microsecondes. Ce n'est pas une chaîne de caractères (String RFC 3339) lisible par un humain.
- **Exemple (Données) :** Les relations sont stockées sous forme de pointeurs ou de graphes (SurrealDB). Pas de redondance pour faciliter une vue SQL à plat.
- **Le Juge :** Le Kernel (`crates/cynic-kernel`) et les smart contracts on-chain n'opèrent que sur cette couche mathématique.

### 2. Layer 4 (Le Système Nerveux / APIs / SurrealDB)
- **Règle :** Le transport des données s'adapte à la vitesse et à l'architecture réseau (WebSockets / RPC), mais conserve la densité sémantique du L1.
- **Action :** Les Live Queries propulsent l'Int64 sans le parser. 

### 3. Layer 7 (Les Interfaces / L'Observateur)
- **Règle :** La traduction visuelle, sémantique et ergonomique appartient EXCLUSIVEMENT aux organes périphériques.
- **Action :** C'est le CLI (`cynic-askesis`), le terminal Odysseum, ou l'interface TALARIA qui capte l'Int64 L1 et le traduit en "Lundi 29 Juin, 20h45".
- **Restriction :** Si une interface requiert une donnée lisible, elle DOIT la traduire elle-même, ou faire appel à une fonction d'affichage dédiée. Elle ne doit JAMAIS imposer au L1 de stocker la donnée sous forme de texte.

## Conséquences Immédiates (La Dette Révélée)

1. **L'Erreur Originelle :** 95% de l'organisme actuel (notamment le `human-kernel.jsonl` et le hachage de la Proof-of-History) a été vicié par une confusion entre présentation et stockage (utilisation de chaînes RFC 3339 pour simuler le *Zero-Markdown Ledger*).
2. **La Résolution :** Tout le système temporel de l'organisme doit être purgé des strings `chrono::Utc::now().to_rfc3339()` et repasser sur de l'Epoch Int64 natif pur. Le "Zero-Markdown Ledger" reste une réalité, car les outils (les "lunettes" de l'observateur humain comme le CLI `askesis log view`) s'occupent de rendre le JSONL compréhensible. 
