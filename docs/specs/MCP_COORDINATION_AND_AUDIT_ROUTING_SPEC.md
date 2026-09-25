# Spécification d'Ingénierie Système : Raccordement MCP & Routage d'Audit v2

**Status :** Normatif & Contractuel (Layer 1 Agent-MCP Interface — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. CATALOGUE D'EXPOSITION (infra/agent-exposure.json)                       │
│    • Déclaration normative de tous les outils MCP autorisés.                │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. ROUTEURS MCP (crates/cynic-mcp/src/routers/)                             │
│    • coord.rs : Routage des baux `cynic_coord_*` (claim, renew, release).   │
│    • local.rs : Outils d'exécution locale pure (`cynic_audit_v2`,           │
│                 `cynic_detection_report`, `cynic_fleet_reconcile_*`).       │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. COUCHE MOTEUR SOUS-JACENTE (crates/cynic-host/)                          │
│    • coordination_lease_store/ : Moteur de baux thread-safe avec fencing.  │
│    • audit_engine/             : Exécution de l'audit AST v2.               │
│    • detection_engine/         : Sondes D-Bus / Process / Hardware.         │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que font ces outils ? Invariants & Schémas)

### 1. Nouveaux Outils Déclarés
* **`cynic_audit_v2`** :
  * *Entrée* : `{}` (aucun paramètre requis, cible le workspace courant).
  * *Sortie* : JSON conforme au schéma `cynic.audit_report.v2`.
  * *Garantie* : Zéro faux positif sur les commentaires, diagnostic instantané (< 500 ms).
* **`cynic_detection_report`** :
  * *Entrée* : `{}`.
  * *Sortie* : JSON conforme au schéma `cynic.detection_report.v1` contenant le superviseur de processus actif (`systemd` vs `process`) et les capacités matérielles sans invocation de shell.

---

## III. HOW (Comment les agents invoquent ces outils ?)

### Exemple d'Invocation JSON-RPC MCP :
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "cynic_audit_v2",
    "arguments": {}
  }
}
```

### Réponse Structurée Émise :
```json
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"schema\": \"cynic.audit_report.v2\",\n  \"summary\": {\n    \"status\": \"ok\",\n    \"critical_count\": 0\n  }\n}"
    }
  ]
}
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi exposer `cynic_audit_v2` via MCP ?**
   * Permet aux agents autonomes (Codex CLI, Claude Code) de s'auto-évaluer avant de valider un commit, sans dépendre d'un terminal externe.
2. **Pourquoi enregistrer chaque outil dans `infra/agent-exposure.json` ?**
   * Évite la prolifération d'outils "fantômes" non audités et garantit qu'aucun chemin machine local ou secret ne fuit dans les prompts système des agents.
