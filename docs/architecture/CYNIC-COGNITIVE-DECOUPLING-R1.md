# Architecture Master Plan: Cognitive Decoupling (R1)

## 1. Le Prisme Fondamental : Séparation des Fonctions (SoC)
L'organisme CYNIC a atteint une taille critique. Le monolithe `cynic-kernel` étouffe sous le poids de fonctions cognitives qui ne lui appartiennent pas. Pour survivre et évoluer (règle du "Bottom-Up" et "Organic Always Wins"), l'architecture doit être formellement scindée selon les couches topologiques (L10 -> L80) et les Constellations.

### La Trinité de la Conscience Organique
L'organisme s'articule désormais autour de trois pôles strictement découplés :

1. **La Moelle Épinière (CYNIC-KERNEL)** : *Mémoire et Coordination (L20)*
   - **Rôle** : Le Kernel n'est plus intelligent. Il est le garant de la mémoire à court/moyen terme (Mempool des tâches) et le bus de routage (MCP). 
   - **Ce qu'il perd** : La gestion des LLMs (`Dogs`), le calcul de consensus (`Judge`), et les budgets API.
   - **Prisme** : Gouvernance et Ordre (Gevurah/Tiferet).

2. **Le Cerveau Perceptif (JARVIS & INFERENCE)** : *Temps Réel et Ontologie (L60/L70)*
   - **Rôle** : Ingérer le chaos du monde physique (clics, voix, fenêtres) via la Constellation de Capteurs, et le transformer en objets sémantiques (Ontologie). La nouvelle caisse `cynic-inference` (extraite du Kernel) lui fournit la puissance de calcul LLM asynchrone.
   - **Ce qu'il gagne** : Le monopole total sur l'I/O de l'utilisateur (Laptop, Kyon).
   - **Prisme** : Perception et Connexion (Hod/Yesod).

3. **Le Miroir (ASKESIS)** : *Discipline et Réflexion (L70)*
   - **Rôle** : Askesis ne lit plus les logs bruts. Il s'abreuve de l'Ontologie produite par le capteur cognitif pour générer des audits comportementaux (HOWL, WAG). Il est la boucle de feedback asynchrone (Psychanalyste de l'organisme).
   - **Prisme** : Intuition et Sagesse (Chokhmah).

---

## 2. État Ciblé (Target State) par Constellation

### A. Constellation: Core Kernel
- **Composant** : `crates/cynic-kernel`
- **Responsabilité** : API REST `/agent-tasks`, base de données SurrealDB locale, proxy MCP.
- **Dépendances** : Zéro LLM, zéro pynput. Ultra-léger.

### B. Constellation: Inference
- **Composant** : `crates/cynic-inference` (Nouveau démon)
- **Responsabilité** : Héberge le code extrait de `judge/` et `dogs/`. Écoute le Kernel (via socket ou poll), reçoit des tâches de "Scoring" ou de "Génération", gère les sémaphores GPU locaux et les API (OpenAI/Anthropic), puis renvoie le résultat au Kernel.

### C. Constellation: Cognitive Sensor
- **Composant** : `crates/cynic-cognitive-sensor` (Windows/Linux)
- **Responsabilité** : Capte la biométrie comportementale (WPM, CPS, Focus, Voix), la transforme en `cognitive-signal-v1` et l'envoie vers l'ingestion cognitive locale.

---

## 3. Phasage de la Migration (L'Amputation)

*On ne construit pas sur du sable. Les étapes suivantes doivent être exécutées séquentiellement :*

- [ ] **Phase 1 : Extraction de l'Inférence**
  - Créer le nouveau crate `crates/cynic-inference`.
  - Y déplacer `src/judge/` et `src/dogs/` depuis le Kernel.
  - Câbler `cynic-inference` comme un client autonome du Kernel.

- [ ] **Phase 2 : Épuration du Kernel**
  - Nettoyer `cynic-kernel/src/main.rs` et `api/rest/` des endpoints liés au jugement.
  - Simplifier le modèle de données du Kernel (Focus strict sur les tâches et observations nues).

- [ ] **Phase 3 : Déploiement Cognitive Sensor (Ontologie 0.1)**
  - Laisser tourner `crates/cynic-cognitive-sensor` pour accumuler du signal brut.
  - Écrire le premier moteur d'ontologie qui traduit ces signaux bruts en "State of Focus".

- [ ] **Phase 4 : Re-câblage Askesis**
  - Mettre à jour `crates/cynic-askesis` pour qu'il consomme l'ontologie cognitive au lieu des vieux logs de Twitter/Hermes.
