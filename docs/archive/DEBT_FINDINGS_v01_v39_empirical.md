# DEBT_FINDINGS — découvertes empiriques v1 → v39 (réécriture)

Reconstitution empirique du 2026-08-31 — grep vivant, non prose. v1-v23 déclaratifs à l'époque, re-sondés maintenant. v24 premier ancrage empirique conservé, v25-v39 ajoutés avec preuves.

Méthode: grep -rn + wc -l + traits + file:line. Tout faux observé a preuve. ROI Tier1 = effort faible + blast ∞.

## v1 (18 natures, 4 axes) — base déclarative, re-sondée
- A1 desired drift: infra/*.json 3 SSOT divergents (registry, lab, surface-registry) → partial
- A2 observe: host pressure none (disk) → partial
- A3 reconcile non pur → fusionné v2 en B9, grep duplication 0 à l'époque mais avéré ensuite
- A4 bounded: ArgvRunner 0 hardcode cynic → ok
- A5 stale → symptôme A9
- A6 unknown→0: health_signal.rs:305 observed_at_unix_s u64 → partial
- B7 monolithe: 18 fichiers >400 (health_signal 1468) → avéré
- B8 ownership: host 77 traits vs kernel 27 traits, kernel lit /proc → avéré
- B9 duplication: 0 à v1, avéré v6
- B10 adapter: /home/user hardcode → partial
- C11 layering: 4 couches incomplet → partial
- C13 vecteur: 0 budget tokens/op → avéré
- C14 harness: lab≠authority flou → partial
- D15 lease: solana_ops.rs:246 fencing ok, budget non → partial
- D16 provenance: ecosystem_package probe → partial
- D17 secret: FORGEJO_TOKEN env only → partial
- D18 amplification: escore sans version → avéré
- T19 config drift → fusionné A1, T20 prose → B8

## v2 (+A7 TOCTOU, +D19 amnésie, fusions) — peer 3.7-flash
- A7 TOCTOU: fleet_reconcile.rs:58 sans CAS digest → avéré
- D19 amnésie: mempool drop 0 compteur → avéré
- C11+C12 fusion, A3→B9, T19→A1, T20→B8, ROI corrigé coût_résiduel

## v3 (1 énergétique fine, 2 temporel)
- C13 vecteur 5 ressources: grep budget 0 → avéré
- C15 coût observation: FleetObserver poll non mesuré → avéré
- A8 versionnage: forensics.rs:11 v1 sans Unknown → partial
- A9 freshness: health_signal.rs:98 Option mais FleetReport sans observed_at → partial

## v4 (B11 couplage, D20 admission, D15a/b, D18)
- B11 couplage: kernel lit /proc au lieu host port (federation_runtime.rs:33) → avéré
- D20 admission: .agent-claims.json vide (revision 2, 0 claims) → avéré
- D15a fencing ok, D15b budget→D20 avéré, D18→D20

## v5 (A10 corroboration, D21 disclosure, consolidations)
- A10 lineage fantôme: convergence/causal.rs:145 Unknown existe mais shared_origin non tracé → partial
- D21 disclosure: agent_skill/metadata.rs:34 Private existe → partial (blast ∞)

## v6 (A11 rollout, D22 localité, fleet)
- A11 rollout aveugle: harness_progression.rs:151 automate statique, fleet_reconcile.rs:58 sans stop → avéré
- D22 localité: agent_recursion.rs:145 + federation_runtime.rs:33 registry_path local → avéré

## v7 (C16 provenance, D23 E-Score)
- C16 provenance: kernel organ/mod.rs 1044 vs inference backend_identity doublon → avéré
- D23 E-Score: grep escore 10 hits sans version → avéré

## v8 (C17 artifact, D24 enrollment)
- C17 artifact: Makefile:32 RUST_MIN_STACK 256MiB ok, Cargo.lock --locked non vérifié → partial
- D24 enrollment: FORGEJO_TOKEN env, file 0640 non systématique → partial

## v9 (A12 forensics, C18 metrics)
- A12 forensics: forensics.rs 70 hits, protocol/lib.rs:68 pub mod → partial (host collection à vérifier)
- C18 metrics: harness 624 hits, target_percent 0 hits → avéré

## v10 (H1 intent, H2 review, H3 délégation — humain)
- H1 intent: 0 typed intent registry → avéré
- H2 review: 71 modified files, 0 claims, saturé → avéré
- H3 délégation: delegation envelope existe mais depth non vérifié → partial

## v11 (C19 bootstrap, M1 verticale)
- C19 bootstrap: build_proof_compressor.rs:54 /home/user hardcode (test asserts) → partial
- M1 verticale: 0 registry par env → avéré

## v12 (G1 worktree, S1 source)
- G1 worktree: .agent-claims.json vide, 1 worktree partagé → avéré
- S1 source: repo-remotes.json forgejo 10.10.10.101:3000 non bootstrappé → partial

## v13 (P1 pattern, N1 network)
- P1 pattern: grep pattern-absorption 0 → partial
- N1 network: headscale/wireguard 2 hits, mutates_host_network typé → partial

## v14 (Sec1 capability, Plat1 platform)
- Sec1 capability: OneShotCapability 0 hits → avéré
- Plat1 platform: CYNIC_INFERENCE_SUPERVISOR injecté ok, /home/user reste tests → partial

## v15 (T1 harness, E1 eval)
- T1 harness: harness 624 hits, catalog existe → partial
- E1 eval: eval_cli.rs:21 file existe, evidence non versionnée → partial

## v16 (U1 interface, PA1 analytics)
- U1 interface: runtime_surface.rs:644 harness_progress existe, proof bundle non exposé → partial
- PA1 analytics: telemetry 0 hits → ok (local evidence)

## v17 (F1 customer, Ext1 external)
- F1 customer: 0 registry par env → avéré
- Ext1 external: .workspace/ existe → partial

## v18 (GC1 pattern, Norm1 normative)
- GC1 pattern: CRATE_EMERGENTES existe, doublon organ non GC → avéré
- Norm1 normative: MANIFESTO existe, foundation.json §481 cité → partial

## v19 (Build1 ICE, Ask1 askesis)
- Build1 ICE: Makefile RUST_MIN_STACK 256MiB + 8GiB test, ulimit non doc → partial (mitigé)
- Ask1 askesis: askesis 6296 LOC 12 traits, isolation non vérifiée → partial

## v20 (Obs1 observer, Z1 bounty)
- Obs1 observer: 8232 LOC, ingestion sans bound → avéré
- Z1 bounty: zeroclaw 401 LOC, SOP non trouvé → avéré

## v21 (Proto1 protocol, Layer1 layers)
- Proto1 protocol: 54853 LOC 7 traits, purity non vérifiée → partial
- Layer1 layers: kernel lit /proc direct (B11) → avéré

## v22 (Know1 docs, Sess1 session)
- Know1 docs: docs quarantined ok → ok
- Sess1 session: recovery 0 hits → avéré

## v23 (Infra1 SPOF, Ops1 as-code)
- Infra1 SPOF: SurrealHttpStorage kernel/bootstrap/storage.rs:39 single endpoint → avéré
- Ops1 as-code: skill existe, SSH gap manuel non probé → partial

## v24 (empirique — premier ancrage complet, 61 brutes)
- Tableau Tier1 18 avérées + 30 partial + 3 ok (voir DEBT_TAXONOMY.md v24). Référence baseline empirique.

## v25 (Aud1 audit, Org1 compass — méta)
- Aud1 audit: DEBT_TAXONOMY.md sans schema debt_audit → avéré (méta)
- Org1 compass: AGENTS.md Organic 138 compass read-only existe → partial

## v26 (Sol1 Solana, Fut1 Futard)
- Sol1 Solana: AGENTS.md study first cité → partial
- Fut1 Futard: AGENTS.md projection cité → partial

## v27 (Tal1 Talaria, Mem1 membrane)
- Tal1 Talaria: AGENTS.md isolated workspace cité → partial
- Mem1 membrane: AGENTS.md membrane cité, mais effondrée (Layer1) → avéré

## v28 (Watch1 watchdog, Soc1 SOC)
- Watch1 watchdog: node/watchdog 77 hits, fencing non vérifié → partial
- Soc1 SOC: kernel/introspection/soc.rs:1 + thresholds → partial

## v29 (Surf1 registry, Exp1 exposure)
- Surf1 registry: surface_registry/catalog.rs 7 hits, KERNEL_AREAS stale → partial
- Exp1 exposure: infra/agent-exposure.json existe, update non systématique → partial

## v30 (Res1 residual, Conv1 convergence — * )
- Res1 residual: residual_unknowns 21 hits mais target_percent 0 → avéré
- Conv1 convergence: Coherence Pattern AGENTS.md 172 cité, preuve par échelle non systématique → avéré

## v31 (Seed1 seed, Univ1 universal)
- Seed1 seed: seed upgrade 0 hits → avéré
- Univ1 universal: AGENTS.md invariants cités → partial

## v32 (Cont1 contract, Math1 math)
- Cont1 contract: foundation.json non layeré → partial
- Math1 math: escore sans verification → avéré

## v33 (Mcp1 MCP, Gpu1 GPU)
- Mcp1 MCP: mcp kernel wiring 0 hits → avéré
- Gpu1 GPU: accelerator 0 sweep → avéré

## v34 (Rem1 remote, Chat1 chat)
- Rem1 remote: remote deploy 0 proof → avéré
- Chat1 chat: probe_live_model_ids 5 hits → partial

## v35 (Poh1 PoH, Fed1 federation)
- Poh1 PoH: Surreal CAS 1 hit → partial
- Fed1 federation: Promoted/Takeover 1 hit → partial

## v36 (Hook1 webhook, Art1 metadata)
- Hook1 webhook: forgejo webhook 0 → avéré
- Art1 metadata: layered contract 0 → avéré

## v37 (Coord1 coordination, Work1 workload)
- Coord1 coordination: agent_workload_envelope 1 hit → partial
- Work1 workload: scheduler durably 0 → avéré

## v38 (Repair1 crate, Design1 design)
- Repair1 crate: orphan 0, crate audit rouge potentiel → partial
- Design1 design: .hermes/plans existe → partial

## v39 (Disc1 découverte, Tax1 bloat — méta)
- Disc1 découverte: 39 itérations fractales → noté
- Tax1 bloat: 91 brutes non figées, non outillées → avéré (méta)

## Synthèse v1→v39 empirique

- Total: 91 brutes / 89 effectives (après 4 consolidations A5→A9, B9→B8, D15b→D20, D18→D20)
- Tier1 avéré constant: ~18-20 dettes (B7,C13,C15,C16,C18,D19,D22,H1,H2,G1,Sec1,F1,GC1,Obs1,Z1,Layer1,Infra1,Res1,Conv1 + variantes)
- Partial: ~30 dettes à fermer effort faible (file:line ci-dessus)
- Ok: 3 (A4,PA1,Know1)
- Méta: Aud1,Org1,Disc1,Tax1 — taxonomie elle-même dette si non figée/câblée

Prochaine: figer v39 + câbler cynic dev debt-audit --json pour scorer automatiquement ces 89 effectives et trier ROI Tier1, puis purger par ROI.

Généré depuis context session (39 itérations) + grep vivant 2026-08-31. Non figé, traçable.
