# Spécification d'Ingénierie Système : Simulation d'Intégration E2E Multi-Agents

**Status :** Normatif & Contractuel (Layer 1-3 Multi-Agent Validation — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système de test ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. SCÉNARIO D'INTÉGRATION E2E (crates/cynic-host/tests/)                    │
│    • multi_agent_e2e_simulation.rs : Test complet multi-acteurs simultanés │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. COMPOSANTS COLLABORATIFS EXERCÉS                                         │
│    • cynic-host::coordination_lease_store (Verrouillage avec Fencing)       │
│    • cynic-host::audit_engine (Audit AST v2 du workspace)                  │
│    • cynic-intel::futarchy::cpmm (Matching CPMM & Résolution de Décision)   │
│    • cynic-intel::futarchy::vault (Coffre de Liquidité Odysseum)            │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que prouve cette simulation ? Scénario & Assertions)

### Scénario Émulé :
1. **Agent-Codex** réserve `crates/cynic-inference` $\to$ Reçoit `LeaseGrant` (Token 1001).
2. **Agent-Claude** tente d'acquérir `crates/cynic-inference` $\to$ Reçoit `TargetClaimOutcome::Conflict`.
3. **Agent-Claude** se replie sur `crates/cynic-intel` $\to$ Reçoit `LeaseGrant` (Token 1002).
4. **Agent-Pi** réserve `docs/specs` $\to$ Reçoit `LeaseGrant` (Token 1003).
5. Les agents exécutent l'**Audit AST v2** en mémoire $\to$ 0 violation critique.
6. **Agent-Codex** crée un marché de gouvernance financé par le coffre Odysseum : *"Should CYNIC adopt Solana Anchor 0.30+?"*.
7. **Agent-Claude** et **Agent-Codex** placent des paris $\to$ Le prix monte à $\ge 60\%$.
8. Clôture du marché $\to$ `MarketDecision::Pass`.
9. Les 3 agents finalisent leurs baux avec des hashs de preuve $\to$ Le registre de baux redevient vierge.

---

## III. HOW (Comment exécuter ce test d'intégration ?)

```bash
# Exécution ciblée du test multi-agents
cargo test -p cynic-host --test multi_agent_e2e_simulation
```

---

## IV. WHY (Pourquoi ce test est capital ?)

1. **Preuve d'Antifragilité Concurrente** : Démontre mathématiquement que plusieurs agents autonomes ne peuvent pas corrompre l'état du code ni entrer en conflit destructeur.
2. **Couplage Réussi des Verticales** : Valide l'interconnexion fluide entre la coordination (L1), l'audit de code (L0) et la futarchie économique (L3).
