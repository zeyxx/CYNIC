# DEBT_TAXONOMY — dettes de l'organisme (itération 54, non figée — doute proche)

STATUS: itération 54 — FIGÉE (host 4 layers & secret classes). Empirique v1-v53 conservé. 121 brutes / 119 effectives. 54 itérations. Figée 2026-08-31 pour câblage debt-audit --json.
Source pour `cynic-debt-audit` + futur `cynic dev debt-audit --json`.
ROI = (unlock+risque*blast)/(effort+coût_résiduel)

## Ajouts v54 — 2 natures (avant doute)

### Layer4 — HOST 4 LAYERS — [NOUVEAU v54, axe Layer4]
Layer4 4 layers non ownées — [NOUVEAU] famille host (disk/memory/CPU/network) sans owner par couche (physical/volume/usage/pressure), lower layer manquante mais rapportée "ok" au lieu de residual unknown, type dans CLI au lieu de cynic_protocol
   Ex: disk pressure none rapportée ok, memory PSI non lue, NIC link non owné
   Faux observé: "host cohérent" mais couche basse vide fabriquée
   Fix: Host substrate observation layering AGENTS.md — 4 layers par famille, types in cynic_protocol::<family>, parsers in cynic_host::domain::<family>::probe, observe port Local/Remote, FleetReport
   Réf: AGENTS.md Host substrate observation layering (4 layers), C11/Layer1

### SecretClass — SECRET CLASSES — [NOUVEAU v54, axe SecretClass]
SecretClass1 secret class violée — [NOUVEAU] secret utilisé hors when (HumanAdmin à runtime, MachineEnrollment non one-time, RuntimeMachineIdentity manquante), runtime dépend de HumanAdmin, secret en git/chat/proof
   Ex: runtime HumanAdmin sudo, llama API key en TOML git, FORGEJO_TOKEN en remote URL
   Faux observé: "secret géré" mais class violée, HumanAdmin à runtime
   Fix: Secrets AGENTS.md — 4 classes (HumanAdmin bootstrap only, MachineEnrollment first join, Recovery breakglass, RuntimeMachineIdentity steady-state) + placement env/file/adapter, jamais git
   Réf: AGENTS.md Secrets, D17/D21/D24

### Rappel 119→121 brutes (117→119 effectives)
V6 fleet, V7 C16/D23, V8 C17/D24, V9 A12/C18, V10 H1-H3, V11 C19/M1, V12 G1/S1, V13 P1/N1, V14 Sec1/Plat1, V15 T1/E1, V16 U1/PA1, V17 F1/Ext1, V18 GC1/Norm1, V19 Build1/Ask1, V20 Obs1/Z1, V21 Proto1/Layer1, V22 Know1/Sess1, V23 Infra1/Ops1, V24 empirique, V25 Aud1/Org1, V26 Sol1/Fut1, V27 Tal1/Mem1, V28 Watch1/Soc1, V29 Surf1/Exp1, V30 Res1/Conv1, V31 Seed1/Univ1, V32 Cont1/Math1, V33 Mcp1/Gpu1, V34 Rem1/Chat1, V35 Poh1/Fed1, V36 Hook1/Art1, V37 Coord1/Work1, V38 Repair1/Design1, V39 Disc1/Tax1, V40 Priv1/Syn1, V41 Mod1/Loop1, V42 Hard1/Decl1, V43 Detect1/Canon1, V44 Inst1/Adapt1, V45 Exec1/Alpha1, V46 Harn1/SessH1, V47 Boot1/Git1, V48 Truth1/Policy1, V49 Proj1/Transact1, V50 Scale1/Unknown1, V51 Down1/Bug1, V52 AgentRule1/Fractal1, V53 Compass1/Gate1, V54 Layer4/SecretClass1
Axes: ... +Layer4(1), SecretClass(1) — 119 effectives — 54 itérations — doute proche

Pourquoi v54 ? — avant doute
- Layer4 = l'observation : sans 4 layers ownées, host layering s'effondre — physical manquante fabriquée.
- SecretClass1 = le quand : sans classes, runtime dépend de l'humain — souveraineté percée.

Comment scorer ?
- Layer4: effort moyen (4 ports) + blast N hosts → Tier1 avec C11
- SecretClass1: effort faible (class check) + blast ∞ (secrets) → Tier1 max avec D21

Wiring: host 4 layers, secret classes, ObservationPort

Vérif:
grep -rn "Host substrate observation layering" AGENTS.md | head
grep -rn "Secret classes\|HumanAdmin.*bootstrap" AGENTS.md | head

Changements v53→v54: +Layer4 +SecretClass1. 121 brutes / 119 effectives. Non figé. 54 itérations. Doute proche — prochaine itération nécessite grep vivant pour prouver famille distincte non dupliquée (ex: CLI thinness 200/400, surface grammar, shared helpers, sovereignty relay) sinon épuisement.

Réf: AGENTS.md Host substrate 4 layers, Secrets — jalon 54

Note doute: inventaires 1-7, cross-scale 1-8, harness/session, industrial/git, agent/fractal, compass/gate, host/secrets bouclés. Reste: CLI thinness thresholds, surface/audit grammar, shared helpers, sovereignty relay détails — à prouver empiriquement avant v55, sinon épuisement et figeage v54 + câblage debt-audit --json.
