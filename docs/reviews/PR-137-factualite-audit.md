# Audit factualité PR 137 — Verdict reviewer : REQUEST CHANGES → synthèse : APPROVE (voir § Corrections)

Prisme : factualité. Head `2601eef2`, base `9c2480ea`. 14 commits (9 picks -x + 5 convergence).

## Corrections apportées aux findings du reviewer (preuves directes)

**R1 — Chiffres ledger « 237/25 » : erreur de mesure.** Diff au niveau entrée du commit `42971a36` : **230 supprimées** (223 LITERAL-TRACKED + 7 UNREGISTERED-PLACEMENT, tout PATH-004), **18 ajoutées** (tout PATH-004-LITERAL-TRACKED). Le message « 230 stale deleted, 18 survivors recorded » est exact. Les 34 PATH-004 restants sont les survivants déclarés, pas une contradiction : le claim zéro porte sur les *codes* PATH-004 (vérifié : audit 0 code PATH-004).

**R2 — « Aucune preuve builder pour 2601eef2 » : limitation d'accès.** `build-proofs.jsonl` : `source_commit 2601eef2… action cargo:check status converged` (83 s). Preuve existe.

**R3 — Skip L non tracé : levé.** L = `5a2c1e15` (chore(ledger)), remplacé par le reconcile piloté par audit `42971a36`. Référence à citer ainsi.

**R4 — Adaptations hors messages : point process accepté.** Documentées dans le body PR (précédent #134/#136). Pas de réécriture des 9 picks.

## Confirmé

9 picks (5 identiques, 4 adaptés légitimement), skips (e71c0ded subsumed, T/T2 → writer natif), 9 fantômes sortis, huggingface L2, test 9→3 (cas shrink sanctionné).
