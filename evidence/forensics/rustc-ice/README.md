# Forensic Evidence: Corpus rustc ICE (Août 2026)

> **Nature :** Preuves médico-légales de défaillance du compilateur  
> **Source :** Cortex local (`rustc 1.96.0-x86_64-unknown-linux-gnu`)  
> **Diagnostic :** SIGSEGV dans `rustc_expand` lors du macro-expansion et du lowering de `syn`/`proc-macro2`  
> **Autorité de résolution :** Déport de compilation vers le builder distant (`cynic-builder-ssh` / `10.10.10.119`) exécutant `rustc 1.95`.

Ce corpus de 18 fichiers constitue la trace observable et inaltérable des instabilités du compilateur ayant motivé l'architecture distribuée de CYNIC.
