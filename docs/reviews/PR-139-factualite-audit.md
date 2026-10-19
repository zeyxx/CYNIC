# Audit factualité PR 139 — Verdict reviewer : REQUEST CHANGES → synthèse : APPROVE (voir § Corrections)

Prisme : factualité. Head `8f1854b3`, base `cd30e668`. 11 picks -x + 2 skips.

## Corrections (preuves directes)

**F1 — Pick #11 « divergent, 2 fichiers perdus » : FAUX POSITIF.** Le replay contient 2 fichiers au lieu de 4 par déduplication upstream correcte, vérifiée au contenu :
- `main.rs` (`&config_path` ×2) : déjà apporté par #137 (`2601eef2`) présent dans la base — présent au tip.
- `forgejo_webhook_e2e.rs` (`KeyInit`) : déjà sur `main` au pick — présent au tip.
Le patch-id diffère légitimement après rebase (contexte + upstream changés). Contenu final vérifié présent.

**F2 — « Aucune preuve builder » : limitation d'accès.** `build-proofs.jsonl` : `source_commit 8f1854b3… status converged` (118 s). Preuve existe.

## Confirmé

10/10 patch-ids identiques (hors #11 expliqué), 2 skips subsumed (réserves déjà sur main), empilement #137 (ancêtre + H1 présent).
