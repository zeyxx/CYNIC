# Historique d'évolution — Titouan Corneloup (zeyxx)

> Document de travail : rapport d'historique pour formalisation CV.
> État : brouillon de cartographie, non finalisé. À enrichir avec la branche du stage privacy-ethereum-framework.
> Date : 2026-09-17
> Auteur : Titouan Corneloup (zeyxx) / Hermes Agent

Ce document cartographie l'évolution technique et intellectuelle de zeyxx, depuis l'initiation au développement assisté par IA jusqu'à la construction d'un runtime Rust souverain. Il vise à servir de base factuelle pour un CV "corporate in the front."

---

## 1. Position actuelle (2026-09)

**Profil** : Ingénieur Rust / Solana / sécurité, en recherche d'alternance (B3 Cybersécurité, Paris Ynov Campus, promo 2027).

**Projet maitre** : CYNIC — runtime Rust souverain pour systèmes multi-agents, ~318k LOC, 26 crates, DAG acyclique, audit de boundary exécutable. R&D de fondation avec dimension monétaire/futarchique.

**Rôle dans CYNIC** : Fondateur, architecte, opérateur souverain (constituent power). Auteur des RFCs, des contrats, des audits d'architecture, des rapports de recherche concurrentielle. Intégration Hermes Agent comme co-pilote de développement.

**Souveraineté** : Forgejo = SSOT (10.10.10.101), GitHub = miroir public, builder (10.10.10.119) = exécution de build, Synapse DNS pour résolution dynamique. Pas de hardcode, registres dynamiques.

---

## 2. CYNIC — projet de R&D et d'apprentissage

### 2.1 Qu'est-ce que CYNIC ?

CYNIC est un runtime Rust de bas niveau pour systèmes multi-agents souverains. Il ne "contrôle" pas les agents : il coordonne, observe, prouve. Une mutation sans read-back proof est un échec, pas un succès.

**Philosophie** : un système se définit par ce qu'il produit (preuves, observations, transitions), pas par ce qu'il prétend être. Human = pouvoir constituant au-dessus de tout système, score, modèle, agent.

**Architecture** : 5 pouvoirs (constituent, judicial, executive, legislative, control). Chaque node = binaire + rôle, host obligatoire. Pensée en modèles structurels/gouvernementaux, docs formels avant implémentation, itérations multiples.

**Couches** :
- `cynic-protocol` : types purs, ports, prédicats. Pas d'I/O, pas de policy.
- `cynic-kernel` : bus de coordination, observations, santé, read-back.
- `cynic-host` : primitives portables d'OS, substrate, observation hardware.
- `cynic-node` : supervision locale, probes, admission.
- `cynic-cli` : surface opérateur mince (parse → inject → call → render).
- `cynic-mcp` : projection agent-facing, remplaçable. Jamais un bootstrap prérequis.

### 2.2 Ce qui est fait (démontrable)

**RFCs et specs** :
- RFC-001 Magnetic Agent Coordination & Thermodynamic Phase Transitions (implémenté, vérifié 2026-09-11). Modèle physique d'exclusion topologique entre agents, champ d'orientation B sans contexte, phases haute/basse T, point de Curie.
- SPEC-CYNIC-STORAGE-V1 : chaîne causale cryptographique SHA3-512, 2-stage commit binding, WAL local avec PoH anchor, settlement Solana hors-bande et batché.
- RFC-002 Five Layer Architecture.

**Contrats** :
- `contracts/foundation.json` : contrat constitutionnel et évolutif.
- `contracts/framework.json` : spécification du framework.
- `infra/runtime-authority.json` : carte d'autorité opérationnelle.
- `infra/registry.json` : registre de placement, lu jamais 하드코드.
- `infra/agent-source-protocol.json` : gouvernance de source machine-readable, forgejo SSOT, branche agent/<id>/<scope>, PR via merge bot.

**Audits exécutables** :
- `cynic dev kernel-audit --json` : 284 fichiers, 151 couverts, 133 non couverts, 2 hotspots (status: blocked).
- `cynic dev ownership-audit --json` : 11 surfaces, status ok avec 2 migrations déclarées.
- `cynic dev crate-audit --json` : 4 crates bloquées (>40k LOC : host 85k, kernel 58k, node 52k, protocol 51k).
- `cynic dev agent-exposure --json`, `cynic dev framework --json`.

**Architecture boundary tests** : 6 crates couvertes, 5 manquantes (inference, intel, observer, market, zeroclaw-solana).

**博报告** :
- Competitive Landscape Report 2026-09-05 : cartographie souverain agent frameworks, futarchy, P2P governance, AI runtime. ~460k LOC Rust, whitepaper statuses, pas de mainnet, déploiement lab Cortex, pas de token public, pas de futarchy live.
- Architecture Geometry & Pattern Audit 2026-09-02 : topologie mesurée, 26 arêtes Cargo, classification DDD.
- Stage 2 Primitive Closure Audit 2026-09-03 : diagnostic stalls sur citoyenneté, mandat, admission, ledger local, consensus, finalité.
- Monetary Systems Research 2026-09-03 : séparation UnitOfAccount ≠ Currency ≠ EnergyBudget ≠ Stake ≠ Reputation ≠ Authority.

### 2.3 Ce qui est en cours / bloqué

**Évolution recente (branche actuelle)** :
- Refactor network : alignement génération mesh plan avec derivation peer canonique et plans registre dynamiques.
- Harden zero-trust nftables firewall policy déclarable et audit.
- Éradication stubs telemetry faux, live probes, reconcile registre dynamique.
- Clean-room extract cynic-forensics de cynic-host, inversion dépendances kernel/CLI.
- Suppression stubs organism-audit, telemetry fake.

**Travaux de split** : kernel (god object ~58k LOC, 14 familles de responsabilités) → extraction incrementale par domaine vers crates dédiées. Organ/inference → cynic-inference (résolu). Lifecycle, fleet, dogs, wallet_judgment, solana backends à extraire.

**Verticales (19 déclarées, statuts contrastés)** :
- Fonctionnel partiel : substrate observation, runtime authority, inference.
- Proto/orphan : futarchy market, agent coordination, companion askesis, public settlement.
- Blocked : talaria_code_evolution, resource_metabolism, node_recursion, fleet_coordination, runtime_federation, solana_public_settlement, one_person_futarchy, contestable_judgment.

**Vision v2 (docs/v2-architecture-canon/)** : ensemble canonique de docs — cosmologie, tokenomique, mécanique deflationnaire, ICO séquence, futarchie, score, organisme topology, roadmap, red team, hérmétique 7 lois. Projets, pas runtime authority.

### 2.4 Ce que CYNIC enseigne (apprentissage)

- Supply chain 100% Rust, éradication wrappers shell.
- Démolition agressive du kernel god object → crates dédiées.
- Dynamic registries > hardcode. Network surfaces lisent infra/registry.json, jamais IPs en dur.
- Blue-green restart pour services runtime : start replacement temp port → health-check → switch config → verify → kill old. Jamais tuer avant healthy.
- Every command variant must reach real executor with live read-back, never canned SUCCESS.
- Decision/policy logic ne vit pas dans cynic-protocol (types/ports seulement).
- 100% English dans code/test/commits/docstrings. Dialogue opérateur 100% Français. Pas de franglais.
- Design gate bottom-up : desired state + authority, read-only observation + port, pure selection rule, exact argv/privilege/consent/rollback/stop, post-action read-back, owning boundary.
- Architectural tests automatisés sur chaque crate boundary.
- Git : worktree isolé, git add -A interdit, one commit per slice, agent/<name>/<scope>, jamais push main direct.
- Dette : suppression destructive pour révéler faiblesses structurelles > avancée chirurgicale lente.

---

## 3. Exploration / curiosité — les axes de recherche

### 3.1 Souveraineté des agents et futarchy

Cartographie concurrentielle poussée : Freysa AI ($30M Series A, TEE-isolated agents), ElizaOS (50k+ agents, plugin ecosystem), Olas/Autonolas (PoAA staking, Gnosis prediction markets), MetaDAO (premier futarchy Solana production, $2.2M seed + $2.5M + $1.5M, META ~$73M), Futarchy Labs (integration widget), GnosisDAO GIP-145, Polymarket, ERC-8004 (21,562+ agents registered), x402 (payments), Allora (MCN), Bittensor (128 subnets, $2.5-3B), 0G ($290M), Ritual ($25M Series A), Akash, Render.

Positionnement CYNIC : membrane souveraine opérateur-unique, pas multi-org DAO. Causal dependency chain (Constituent → Identity → Capability → Resource → Workload → Action → Proof → State Transition → View). Host substrate observation 4 couches (physical→volume→usage→pressure). Mandate-account governance. Read-back proof. Fractal agent recursion. Rust-native industrial bar.

### 3.2 Solana

- Bounty Veilo (privacy_pool, mainnet) — audit parallèle 6 agents, Swap §1 relayer skim full spread (HIGH extractable), PoC `swap_s1_skim.rs` exécutable, backup V-01 overcredit phoenix_ember_unwrap (déjà public).
- `cynic-zeroclaw-solana` : crate adapters Solana, 399 LOC, plugin externe composant port intel.
- PoH atomic chain_hash + CAS retry sur SurrealDB HTTP.
- cynic-anchor program Solana, PoH checkpointing, Anchor client, build proof compressor.
- `cynic-intel` / `solana_safety_dog` : évaluation SolanaPay requests, Helius enrichment, dexscreener backend.
- Settlement Solana hors-bande et batché : WAL checkpoints vérifiés → Merkle root → anchor sur Solana selon operator publication policy.

### 3.3 Fouille / exploration non linéaire

- Vault d'index conceptuel : Magnetic Coordination, Fractal Agent Recursion, Internal Prediction Markets, Solana Settlement & Public Futard.io.
- Epistemic Harmonic Scoring, Competitive Landscape, monetary research, geometry audit, primitive closure audit, edge context map.
- Scripts de bench (qwen38, matrix), runtime evidence bundle, disk audit, isolated runner.
- Proxmox, Synapse DNS, SSH fallback publisher.

---

## 4. Stage privacy-ethereum-framework — en attente d'accès au repo privé

**Contexte fourni** : stage de 180h avec CAPLOGY sur le projet privacy-ethereum-framework (branche `titouan`).

**Blocage technique vérifié** (2026-09-17, état observé) :
- `GET api.github.com/repos/Ihceen/privacy-ethereum-framework` → **404 Not Found** (repo invisible aux requêtes non authentifiées → privé).
- `GET codeload.github.com/Ihceen/.../tar.gz/refs/heads/titouan` → **404 Not Found**.
- `GET github.com/.../tree/titouan` → page "Sign in to GitHub" → page de login exigée → **accès refusé** (vault vide, l'opérateur a décliné `browser_vault_save_login`).
- Recherche repo "privacy-ethereum-framework" user:Ihceen → 0 résultat (le repo n'est pas dans les repos publics de l'user ; il est soit privé soit sous un autre owner).
- `git clone` → bloqué par le mécanisme de consentement de session.

**Conclusion** : le repo est privé et se trouve probablement sous `Ihceen` ou `capology`, mais est inaccessible sans credentials GitHub appartenant à l'opérateur. Deux solutions viables : (a) l'opérateur télécharge le ZIP via l'interface GitHub authentifiée et le dépose sur un chemin local ; (b) l'opérateur fournit un PAT (scope `repo`), utilisé une fois en mémoire non persistante.

**Ce qu'il faudra cartographier dès accès fourni** (checklist) :
- Commits de la branche `titouan` (messages, sha, dates, granularity).
- Fichiers créés / modifiés / supprimés.
- Artéfacts produits (rapports, specs, proof-of-concepts, scripts).
- Responsabilités assumées (ce qui a été conçu vs implémenté vs testé vs documenté).
- Résultats / livrables (validé, testé, déployé ?).

**Action restante** : l'opérateur fournit l'export du repo privé (zip GitHub authentifié ou chemin local cloné), je cartographie le travail effectif et je complète cette section.

**Ce qui est déjà observable dans CYNIC** (liens transitifs, à vérifier) :
- Commit `0720890f` (2026-06-04) : `docs(internal): co-founder framework T.↔S. — cadrage CIIA + agreement draft (#288)`. Mentionne Talaria SAS, co-founder agreement T.↔S., recognition PoH concept. CIIA = cadre légal lié à Talaria, potentiellement lié à la période du stage mais pas confirmé comme livrable du stage PEF.
- La notion de "stage" apparaît dans `docs/v2-architecture-canon/` et `reports/monetary-systems-research.md` comme une notion de maturité du runtime (stage 1 = unité de compte interne). C'est un vocabulaire CYNIC, pas forcément celui du stage PEF.
- Branche distante CYNIC `origin/agent/hermes/ansemhack-plan` (empty sur le remote observé) — nom similaire à "ansemhack" (évoque Ansem, cofondateur d'ElizaOS). À ne pas confondre avec le stage CAPLOGY/PEF tant que l'export n'est pas fourni.

---

## 5. Parcours d'initiation aux IA et développement assisté

### 5.1 Comment zeyxx est arrivé au développement avec IA

**Point de départ** : étudiant B3 Cybersécurité (Paris Ynov, promo 2027), orientation Rust/Solana/sécurité, recherche d'alternance.

**Évolution observable** :
- Passage d'un usage IA comme outil ponctuel à un protocol agentique structuré (claims, budgets, scopes, read-back).
- Émergence d'une épistémologie : un système se définit par ce qu'il produit/fait (faits/preuves observables), pas par ce qu'il prétend être. Amplification par les faits et le PoW plutôt que réponses génériques.
- Formalisation de la souveraineté comme problème architectural : agent deve 하에 human constituent power, pas de contrôle central, preuves lisibles, registres dynamiques.
- Adoption d'outils : Hermes Agent (Claude Code-like local), cynic-mcp, cynic-cli pour définition de surfaces, cynic dev audits pour read-back.

### 5.2 Pratiques de développement assistant

- Claim/heartbeat/release sur worktree isolé.
- BUDGET EXPLICIT, scoped, time-bounded.
- Vector B (intent, invariants, output schema, budget) sans contexte conversationnel dumpé.
- Phase exploration (T>Tc, lecture seule) → phase cristallisation (T→0, exclusion locks, commit atomique + read-back).
- PoC exécutable avant soumission (ex: swap_s1_skim.rs pour Veilo).
- Conventional Commits, branche agent/<id>/<scope>, PR via merge bot, forgejo SSOT.

### 5.3 Études / références

- Phoenix, Swap, Perps, Positions, ZK/Groth16 audits (Veilo bounty).
- Futarchy mécaniques (MetaDAO conditional tokens, TWAP, AMM), ERC-8004 registries, x402 payments.
- Host substrate observation (disk/mem/cpu/net 4-layer probing).
- Solana program security (overcredit, PDA uniqueness, arithmetic checked, signer strict, relayer trust assumptions).

---

## 6. Synthèse pour CV "corporate in the front"

### 6.1 Ce qui est valorisable immédiatement

- **Rust industrial-scale** : 26 crates, 318k+ LOC, DAG acyclique, lints industriels (Axum/Embark baseline), edition 2024, codegen-units/ice isolation documenté.
- **Architecture DDD + boundary tests** : modularité, ports/adaptateurs, 6 crates avec architecture boundary tests exécutables, CRATE_OWNERSHIP.md formalisé.
- **Gouvernance machine-readable** : contrats JSON, spécifications RFCs, audits exécutables (kernel/ownership/crate/agent-exposure/framework), registre de placement dynamique.
- **Souveraineté par conception** : registres dynamiques, pas de hardcode, blue-green restart, SSH fallback, remote execution port, publication chain avec read-back.
- **Recherche concurrentielle** : rapport 122 pages cartographiant souverain agents, futarchy, P2P governance, AI runtime. Positionnement CYNIC structuré.
- **Sécurité / bounty** : Veilo Swap §1 HIGH extractable identifié avec PoC exécutable.
- **Solana** : PoH anchor, anchor client, build proof compressor, safety dog, adapters, settlement batché.
- **Agent protocol** : RFC-001 implémenté, claims/leases/topology, demagnetization passive, Curie point fatigue detection.

### 6.2 Angle "corporate in the front"

- **Faiseur de systèmes** pas faiseur de promesses. Chaque capacité a un owner, un port, une preuve, un statut observé (observed/decile/proposed/blocked/unknown).
- **Industrial bar** : lints deny, tests d'architecture, read-back proof, pas de stubs, pas de succès implicite.
- **Sovereign engineering** : l'opérateur humain est la source de vérité souveraine ; le système observe, propose, prouve, n'admettent jamais sans admission.
- **Communication** : specs douloureusement explicites, documentation aux frontières, minimale dans les implémentations, glossaire à construire.
- **Orientation** : Rust/Solana/sécurité, agents souverains, futarchy, machine-readable governance, R&D de fondation avec dimension monétaire.

### 6.3 Ce qu'il faut compléter

- [ ] Contenu exact du stage privacy-ethereum-framework (branche à fournir).
- [ ] Dates de début/fin de chaque phase (CYNIC, stage, autres projets).
- [ ] Références professionnelles / contacts (Si laissé pour plus tard).
- [ ] Metrics RH-friendly (équivalent équipe, impact business, liens vers projets publics).
- [ ] Autres projets Solana personnels (préciser si existants).
- [ ] Levier vers GitHub public (vitrine) et forgejo (SSOT) pour preuve live.

---

## 7. Prochaines étapes

1. **Fournir la branche du stage privacy-ethereum-framework** → je cartographie travail effectif et je complète §4.
2. **Préciser dates et autres projets Solana** → je structure le temps.
3. **Valider la forme** : ce rapport est fait pour Gemini formaliser un CV. Je peux fournir une version exportable (markdown/JSON) avec sections cleanly séparées.
4. **CV draft** : une fois branche stage intégrée, je peux produire un premier jet de CV avec structure "corporate in the front / R&D in the back."

---

*Document vivant. Doit être lu comme carte, pas comme autorité. Preuves dans le code et les contrats.*
