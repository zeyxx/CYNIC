# CYNIC — Dualité des Registres & Distribution des Artéfacts

## 1. Principe Fondamental : Déclaré vs Observé

Dans la doctrine épistémologique de CYNIC, un système ne se définit pas par ce qu'il prétend être, mais par ses faits observables et mesurables.
Pour éliminer les illusions d'état et les déclarations fantômes, le registre de la flotte est formellement découplé en deux couches :

```
┌──────────────────────────────────────┐
│  REGISTRE DÉCLARÉ (Intention)        │ <── Modification humaine / PR / Architecture
│  infra/registry.json                 │     Définit les rôles prescrits et la topologie cible
└──────────────────┬───────────────────┘
                   │
                   ▼  Comparaison épistémique (Audit de divergence)
                   ▲  cynic node audit / scripts/probe-fleet-observed-registry.py
                   │
┌──────────────────┴───────────────────┐
│  REGISTRE OBSERVÉ (Preuve Réelle)    │ <── Sondes réseau en temps réel / Binary attestation
│  infra/registry-observed.json        │     Mesure RTT, joignabilité, ports ouverts, SHA-256
└──────────────────────────────────────┘
```

---

## 2. Les Contrats Formels

### 2.1. Registre Déclaré (`infra/registry.json`)
- **Rôle** : Single Source of Truth (SSOT) de la configuration attendue.
- **Règle d'or** : Jamais muté automatiquement par les sondes pour éviter toute dérive d'autorité silencieuse.

### 2.2. Registre Observé (`infra/registry-observed.json`)
- **Schéma** : `cynic.registry_observed.v1`
- **Champs clés** :
  - `status` : `reachable` | `unreachable` | `filtered`
  - `rtt_ms` : Latence aller-retour mesurée
  - `services` : Ports et protocoles confirmés
  - `binary_attestation` : SHA-256 calculé du binaire `cynic` sur l'hôte
  - `divergences` : Liste des écarts détectés par rapport à `infra/registry.json`

---

## 3. Runtime Artifact Store (`10.10.10.103:8443`)

- **Contrat** : `infra/runtime-artifact-store.json`
- **Endpoint HTTP** : `http://10.10.10.103:8443`
- **Chemin système** : `/data/runtime-releases`
- **Mécanisme de publication** :
  - Exécuté par le builder (`10.10.10.119`) via `scripts/publish-release-to-store.sh`.
  - Dépôt atomique sous `/data/runtime-releases/v<version>/` avec génération de `SHA256SUMS`.
  - Mise à jour atomique du symlink `latest`.
  - Téléchargement libre et non-authentifié pour tous les conteneurs et agents de la flotte.
