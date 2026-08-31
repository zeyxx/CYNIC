# DEBT_TAXONOMY — dettes de l'organisme (itération 36, non figée)

STATUS: itération 36 — webhook & metadata layers (fail-closed, layered contracts). Non figée. Empirique v24 conservé.
Source pour `cynic-debt-audit` + futur `cynic dev debt-audit --json`.
ROI = (unlock+risque*blast)/(effort+coût_résiduel)

## Ajouts v36 — 2 natures

### Hook — WEBHOOK — [NOUVEAU v36, axe Hook]
Hook1 webhook non fail-closed — [NOUVEAU] Forgejo push webhook vers cynic build triggers non fail-closed, sans trigger, sans proof, webhook silencieux
   Ex: push sans webhook, build trigger sans lease, fail-open
   Faux observé: "CI triggered" mais non, build non lancé, preuve manquante
   Fix: forgejo webhook automation = push webhook → cynic build triggers, fail-closed (cynic-forgejo-webhook-automation)
   Réf: forgejo webhook, builder runtime

### Art — ARTIFACT METADATA — [NOUVEAU v36, axe Art]
Art1 metadata non layerée — [NOUVEAU] CYNIC metadata non via layered contracts, non content-addressed, non versionnée
   Ex: artifact metadata en prose, non layered, non injectée
   Faux observé: "metadata ok" mais non portable, non reproductible
   Fix: metadata layering via contracts (cynic-artifact-metadata-layering)
   Réf: artifact integrity, C17

### Rappel 83→85 brutes (81→83 effectives)
V6 fleet, V7 C16/D23, V8 C17/D24, V9 A12/C18, V10 H1-H3, V11 C19/M1, V12 G1/S1, V13 P1/N1, V14 Sec1/Plat1, V15 T1/E1, V16 U1/PA1, V17 F1/Ext1, V18 GC1/Norm1, V19 Build1/Ask1, V20 Obs1/Z1, V21 Proto1/Layer1, V22 Know1/Sess1, V23 Infra1/Ops1, V24 empirique, V25 Aud1/Org1, V26 Sol1/Fut1, V27 Tal1/Mem1, V28 Watch1/Soc1, V29 Surf1/Exp1, V30 Res1/Conv1, V31 Seed1/Univ1, V32 Cont1/Math1, V33 Mcp1/Gpu1, V34 Rem1/Chat1, V35 Poh1/Fed1, V36 Hook1/Art1
Axes: A(6+), B(5), C(7+), D(9), H(3), M(1), G(1), S(1), P(1), N(1), Sec(1), Plat(1), T(1), E(1), U(1), PA(1), F(1), Ext(1), GC(1), Norm(1), Build(1), Ask(1), Obs(1), Z(1), Proto(1), Layer(1), Know(1), Sess(1), Infra(1), Ops(1), Aud(1), Org(1), Sol(1), Fut(1), Tal(1), Mem(1), Watch(1), Soc(1), Surf(1), Exp(1), Res(1), Conv(1), Seed(1), Univ(1), Cont(1), Math(1), Mcp(1), Gpu(1), Rem(1), Chat(1), Poh(1), Fed(1), Hook(1), Art(1) — 83 effectives

Pourquoi v36 ?
- Hook1 = le déclencheur : sans webhook fail-closed, le build ne se lance pas — toute la chaîne artifact (C17/Art1) dort.
- Art1 = la description : sans metadata layerée, l'artefact n'est pas reproductible — C17 reste faux vert.

Comment scorer ?
- Hook1: effort faible (webhook) + blast N (tous builds) → Tier1 avec Build1
- Art1: effort faible (layer) + blast N (tous artefacts) → Tier1 avec C17

Wiring: forgejo webhook, artifact metadata layers, builder runtime

Vérif:
grep -rn "forgejo.*webhook\|build.*trigger" infra/* crates/*/src --include="*.rs" --include="*.json" | head
grep -rn "artifact.*metadata\|layered.*contract" crates/*/src --include="*.rs" | head

Changements v35→v36: +Hook1 +Art1. 85 brutes / 83 effectives. Non figé. 36 itérations.

Réf: forgejo webhook, artifact metadata
