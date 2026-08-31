# DEBT_TAXONOMY — dettes de l'organisme (itération 35, non figée)

STATUS: itération 35 — PoH atomicity & federation (CAS, no SPOF). Non figée. Empirique v24 conservé.
Source pour `cynic-debt-audit` + futur `cynic dev debt-audit --json`.
ROI = (unlock+risque*blast)/(effort+coût_résiduel)

## Ajouts v35 — 2 natures

### Poh — POH — [NOUVEAU v35, axe Poh]
Poh1 PoH non atomique — [NOUVEAU] PoH insert sans CAS retry loop sur SurrealDB HTTP, non atomique, sans fencing
   Ex: PoH insert sans compare_and_swap retry, race condition
   Faux observé: "anchored" mais non atomique, fork silencieux
   Fix: PoH = CAS retry loop atomique (cynic-poh-atomicity)
   Réf: cynic-poh-atomicity, D15 fencing

### Fed — FEDERATION — [NOUVEAU v35, axe Fed]
Fed1 federation avec SPOF — [NOUVEAU] federation avec SPOF kernel, non kernel-optional, pas de node autonomy, pas de federation membrane
   Ex: fleet sans Promoted/Takeover, kernel single point, pas de local recursion
   Faux observé: "federated" mais SPOF, non autonome
   Fix: federation pattern = no SPOF, kernel-optional, node-local agent tree (AGENTS.md Fractal)
   Réf: cynic-federation-pattern, D22, Mem1

### Rappel 81→83 brutes (79→81 effectives)
V6 fleet, V7 C16/D23, V8 C17/D24, V9 A12/C18, V10 H1-H3, V11 C19/M1, V12 G1/S1, V13 P1/N1, V14 Sec1/Plat1, V15 T1/E1, V16 U1/PA1, V17 F1/Ext1, V18 GC1/Norm1, V19 Build1/Ask1, V20 Obs1/Z1, V21 Proto1/Layer1, V22 Know1/Sess1, V23 Infra1/Ops1, V24 empirique, V25 Aud1/Org1, V26 Sol1/Fut1, V27 Tal1/Mem1, V28 Watch1/Soc1, V29 Surf1/Exp1, V30 Res1/Conv1, V31 Seed1/Univ1, V32 Cont1/Math1, V33 Mcp1/Gpu1, V34 Rem1/Chat1, V35 Poh1/Fed1
Axes: A(6+), B(5), C(7+), D(9), H(3), M(1), G(1), S(1), P(1), N(1), Sec(1), Plat(1), T(1), E(1), U(1), PA(1), F(1), Ext(1), GC(1), Norm(1), Build(1), Ask(1), Obs(1), Z(1), Proto(1), Layer(1), Know(1), Sess(1), Infra(1), Ops(1), Aud(1), Org(1), Sol(1), Fut(1), Tal(1), Mem(1), Watch(1), Soc(1), Surf(1), Exp(1), Res(1), Conv(1), Seed(1), Univ(1), Cont(1), Math(1), Mcp(1), Gpu(1), Rem(1), Chat(1), Poh(1), Fed(1) — 81 effectives

Pourquoi v35 ?
- Poh1 = l'ancrage : sans atomicité, PoH n'ancre rien — settlement (Sol1) sur sable.
- Fed1 = la fédération : sans no-SPOF, fleet n'est pas fédérée — D22/Mem1 restent monolithe.

Comment scorer ?
- Poh1: effort faible (CAS loop) + blast ∞ (tout PoH fork) → Tier1 max avec D15
- Fed1: effort élevé (membrane) + blast ∞ (tout fleet) → Tier1 max avec Mem1/Layer1

Wiring: PoH CAS, federation pattern, kernel-optional, Promoted/Takeover

Vérif:
grep -rn "PoH.*CAS\|Surreal.*CAS\|compare_and_swap" crates/cynic-kernel/src --include="*.rs" | head
grep -rn "federation.*pattern\|Promoted.*Takeover\|kernel.*optional" crates/*/src --include="*.rs" | head

Changements v34→v35: +Poh1 +Fed1. 83 brutes / 81 effectives. Non figé. 35 itérations.

Réf: PoH atomicity, federation pattern
