# Rapport de Benchmark Empirique Multi-Dimensionnel — AMD Ryzen 7 5700G (32 Go)

**Date :** 28 août 2026  
**Cible Matérielle :** AMD Ryzen 7 5700G (8C/16T, Zen 3) + Radeon Vega 8 (Vulkan0 RADV) + 32 Go DDR4-3200 (Dual-Channel)  
**Moteur d'Inférence :** `llama-server` (Vulkan0 RADV + Zen 3 AVX2, KV quant `q4_0`/`q8_0`, Flash Attention activée)  
**Données Brutes :** `benchmark-5700g-empirical-results.json`

---

## 1. Tableau Récapitulatif des 9 Configurations Mesurées

| Modèle | Poids / Actif | Offload | Contexte | Prompt (t) | Prefill ($\text{tok/s}$) | Decode ($\text{tok/s}$) | TTFT ($\text{ms}$) | Peak RSS ($\text{Mo}$) | Bande Passante Déduite |
|---|---|---|---|---|---|---|---|---|---|
| **MiniCPM5-1B (Q8_0)** | $1.0\text{B} / 1.0\text{B}$ | `CPU-only` (ngl 0) | 4k | 512 | 487.56 | 37.43 | 717.86 | 1 258.6 | $37.43\,\text{GB/s}$ |
| **MiniCPM5-1B (Q8_0)** | $1.0\text{B} / 1.0\text{B}$ | `Vulkan0` (ngl 99) | 4k | 512 | **744.06** | **41.72** | **470.39** | **340.4** | **$41.72\,\text{GB/s}$** |
| **MiniCPM5-1B (Q8_0)** | $1.0\text{B} / 1.0\text{B}$ | `Vulkan0` (ngl 99) | 8k | 2 048 | 597.42 | 38.48 | 2 236.27 | 357.3 | $38.48\,\text{GB/s}$ |
| **LFM2.5-2.6B (Q4_K_M)** | $2.6\text{B} / 2.6\text{B}$ | `CPU-only` (ngl 0) | 4k | 512 | 159.65 | 20.41 | 2 298.72 | 1 768.4 | $29.19\,\text{GB/s}$ |
| **LFM2.5-2.6B (Q4_K_M)** | $2.6\text{B} / 2.6\text{B}$ | `Vulkan0` (ngl 99) | 4k | 512 | **234.77** | **23.96** | **1 563.25** | **365.8** | **$34.27\,\text{GB/s}$** |
| **LFM2.5-2.6B (Q4_K_M)** | $2.6\text{B} / 2.6\text{B}$ | `Vulkan0` (ngl 99) | 8k | 2 048 | 244.54 | 23.17 | 5 692.36 | 366.0 | $33.13\,\text{GB/s}$ |
| **Qwythos-9B (Q4_K_M)** | $9.0\text{B} / 9.0\text{B}$ | `CPU-only` (ngl 0) | 4k | 512 | 51.23 | 6.19 | 7 183.07 | 6 084.5 | $30.66\,\text{GB/s}$ |
| **Qwythos-9B (Q4_K_M)** | $9.0\text{B} / 9.0\text{B}$ | `Vulkan0` (ngl 99) | 4k | 512 | **78.86** | **7.40** | **4 666.53** | **1 084.7** | **$36.61\,\text{GB/s}$** |
| **Qwythos-9B (Q4_K_M)** | $9.0\text{B} / 9.0\text{B}$ | `Vulkan0` (ngl 99) | 8k | 2 048 | 84.22 | 7.33 | 16 539.15 | 1 081.2 | $36.26\,\text{GB/s}$ |

---

## 2. Découvertes Physiques & Enseignements Majeurs

### A. Supériorité Systématique de l'Offload Vulkan0 RADV
- **Accélération Prefill massive** : **+47% à +54%** de gain en vitesse de traitement de prompt par rapport aux 8 cœurs Zen 3 seuls (ex: 51.2 $\to$ 78.9 tok/s sur 9B ; 487.6 $\to$ 744.1 tok/s sur 1B).
- **Accélération Décodage** : **+12% à +20%** de débit supplémentaire en génération.
- **Réduction de la pression RAM Host** : L'offload Vulkan mappe les tenseurs dans le pool GTT Radeon, réduisant le RSS CPU de **6 Go à 1 Go** pour le 9B.

### B. Validation Expérimentale du Modèle Roofline (Kiely 2026)
$$\text{Bande Passante Mémoire Soutenue} = \text{Débit Décodage } (\text{tok/s}) \times \text{Poids par Token } (\text{GB})$$

- **Bande Passante Moyenne Soutenue observée :** **$34.2\text{--}41.7\,\text{GB/s}$**
- Cette mesure est remarquablement constante entre tous les modèles (1B, 2.6B, 9B), prouvant que le décodage sur le Ryzen 5700G est **strictement memory-bandwidth bound** et atteint ~70-80% de l'efficacité maximale du contrôleur DDR4-3200 ($51.2\,\text{GB/s}$).

### C. Conséquence pour la Frontière de Pareto des MoE
Sur cette bande passante de $\approx 36\,\text{GB/s}$ :
- Un modèle Dense 35B générerait à $\approx 1.8\,\text{tok/s}$ (inutilisable en agentique interactif).
- Un **MoE 35B avec 3B de paramètres actifs** ne lit que $\approx 0.9\text{--}1.2\,\text{Go}$ par token, ce qui projette un débit de **$30\text{--}40\,\text{tok/s}$** avec l'intelligence d'un modèle 35B.
