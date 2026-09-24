# CYNIC — LE CANON PROTOCOLAIRE (WHITEPAPER TECHNIQUE)
## Spécification 01 : Proof of Humanity Continue, Métriques Physiologiques et Chaos Biologique

---

> **Statut :** Spécification formelle de la Couche d'Admission (Couche 1)  
> **Auteurs :** TALARIA Architecture Group  
> **Version :** 1.0-calibrated  

---

### 1. Le Diagnostic des Systèmes d'Identité Défaillants

Toutes les tentatives antérieures de « Proof of Humanity » dans l'écosystème numérique souffrent d'une tare fatale : **l'instantanéité ponctuelle (*One-Shot Check*)**.
* **Le modèle Captcha / Cloudflare** : Résolu en quelques millisecondes par des modèles de vision et des fermes de solveurs offshore pour une fraction de centime.
* **Le modèle Biométrique Centralisé (Worldcoin / Iris Scan)** : Un scan ponctuel d'iris stocké ou haché dans un registre privé crée un pot de miel biométrique vulnérable au vol d'identité physique, tout en étant contournable dès le lendemain par l'achat d'un compte scanné auprès d'un tiers précaire.
* **L'illusion du KYC étatique** : Confier la vérification d'humanité à un passeport d'État réintroduit la censure, la surveillance politique et l'asymétrie administrative.

**Le Principe Fondamental de CYNIC :**  
L'humanité n'est pas un tampon gravé une fois pour toutes : **l'humanité est un flux métabolique continu.**  
On ne prouve pas que l'on est humain en montrant un papier ; on le prouve par **la signature thermodynamique ininterrompue de son interaction avec la machine**.

---

### 2. Télémétrie en Continu : Les 4 Vecteurs Physiologiques et Comportementaux

La Proof of Humanity (PoH) de CYNIC opère une **mesure en flux continu (*continuous streaming telemetry*)** capturant le chaos biologique inforgeable à travers 4 vecteurs orthogonaux :

```text
 ┌────────────────────────────────────────────────────────────────────────┐
 │           LE PIPELINE DE CAPTURE DE LA PROOF OF HUMANITY               │
 └────────────────────────────────────────────────────────────────────────┘

 [CAPTEURS MATÉRIELS & PÉRIPHÉRIQUES DU BÂTISSEUR]
   │
   ├──► Vecteur 1 : Dynamique de Frappe & Micro-Latences (Keystroke Dynamics)
   ├──► Vecteur 2 : Télémétrie Biomécanique & Gigue de Visée (Mouse/Trackpad)
   ├──► Vecteur 3 : Variabilité Cardiaque & Micro-Tremblements (HRV / IMU)
   └──► Vecteur 4 : Entropie Décisionnelle sous Pression Cognitive
   │
   ▼
 [LE MOTEUR DE SIGNATURE DU KERNEL LOCAL (Rust)]
   • Agrégation en fenêtres glissantes de 60 secondes ($W_\tau$).
   • Extraction des distributions statistiques (Asymétrie, Kurtosis, Entropie de Shannon).
   • ZÉRO transmission de données brutes (Préservation intégrale de la vie privée).
   │
   ▼
 [GÉNÉRATEUR DE PREUVE ZK (Aura Zero-Knowledge Proof)]
   • Preuve mathématique : « Le signal $S$ appartient à la distribution humaine valide $\mathcal{H}$
     avec une confiance $C > 0.999$, sans révéler aucune métrique physiologique brute ».
   │
   ▼
 [REGLEMENT ON-CHAIN SUR SOLANA]
   • Vérification de la preuve ZK en < 5ms.
   • Émission du certificat d'admission : `HumanContinuityProof(epoch, hash, confidence)`.
```

#### 2.1 Vecteur 1 : Dynamique de Frappe et Micro-Latences (*Keystroke Dynamics*)
L'IA ou le script génère du texte par paquets instantanés ou simule une vitesse de frappe artificielle (ex: 80 mots/minute réguliers).  
L'humain, lui, possède une empreinte biomécanique musculaire unique :
* **Dwell Time ($T_d$)** : La durée exacte d'enfoncement physique de chaque touche (microsecondes).
* **Flight Time ($T_f$)** : Le temps de vol entre le relâchement d'une touche et l'enfoncement de la suivante.
* **Gigue d'Hésitation Cognitive** : Les micro-pauses précédant des choix syntaxiques complexes ou des corrections (Backspaces) suivent une loi de distribution log-normale humaine que les réseaux de neurones ne peuvent pas contrefaire en temps réel sans trahir un coût de latence anomal.

#### 2.2 Vecteur 2 : Télémétrie Biomécanique et Gigue de Visée
Le mouvement de la souris ou du curseur sur un écran par un bras humain est contraint par les leviers osseux, les frictions musculaires et le tremblement physiologique naturel (fréquence de 8 à 12 Hz) :
* **Loi de Fitts biologique** : La relation entre la distance à une cible et le temps de déplacement suit une courbe logarithmique amortie spécifique à la biomécanique humaine.
* **Accélération et Jerk (Dérivée de l'accélération)** : Le déplacement d'un bot ou d'un agent synthétique présente des discontinuités mathématiques ou des lissages béziers trop parfaits. Le muscle vivant produit des micro-corrections chaotiques continues.

#### 2.3 Vecteur 3 : Variabilité Cardiaque et Métabolisme (HRV & Capteurs Haptiques)
Pour les stations de travail souveraines équipées de capteurs haptiques (trackpads sensibles à la pression, montres connectées locales, bagues de télémétrie) :
* **Variabilité de la Fréquence Cardiaque (HRV)** : La signature d'un cœur vivant n'est jamais métronomique. La variabilité d'intervalle R-R reflète l'activité du système nerveux autonome (sympathique/parasympathique) face à la concentration et au stress.
* Ce signal est l'étalon physique suprême : **aucun algorithme ne peut simuler en temps réel la réponse d'un ventricule cardiaque humain sous contrainte cognitive sans être relié à un organisme vivant.**

#### 2.4 Vecteur 4 : Entropie Décisionnelle sous Pression Cognitive
L'asymétrie de décision humaine :
* Face à un dilemme d'architecture ou un bug complexe, l'IA calcule immédiatement ses probabilités de tokens.
* L'humain lit, relit, s'arrête, doute, expérimente des hypothèses absurdes, efface son code, change de direction. Cette entropie de recherche non-linéaire est la signature pure du saut créatif.

---

### 3. La Formulation Mathématique du Score d'Humanité $\mathcal{H}_t$

À chaque époque d'observation $t$ de durée $\tau$, le kernel calcule le tenseur de caractéristiques biométriques $X_t \in \mathbb{R}^d$.  
Le score d'humanité $\mathcal{H}_t \in [0, 1]$ est défini par :

$$\mathcal{H}_t = \sigma \left( \sum_{i=1}^4 w_i \cdot \mathcal{D}_{\text{KL}}\big(P_i(X_t) \parallel P_{\text{synth}}\big) - \lambda \cdot \mathcal{A}(X_t) \right)$$

Où :
* $\mathcal{D}_{\text{KL}}\big(P_i(X_t) \parallel P_{\text{synth}}\big)$ mesure la divergence de Kullback-Leibler entre le signal observé et les distributions connues des bots et modèles d'IA générative.
* $\mathcal{A}(X_t)$ est la fonction d'autocorrélation temporelle : un signal synthétique en boucle répète ses patterns à haute fréquence ($\mathcal{A} \to 1$), ce qui pénalise instantanément le score via le multiplicateur $\lambda$.
* $\sigma(z) = \frac{1}{1 + e^{-z}}$ est la fonction logistique assurant une sortie bornée sur $[0, 1]$.

**Règle d'Admission :**  
Un compte conserve son statut actif si et seulement si :

$$\frac{1}{K} \sum_{k=0}^{K-1} \mathcal{H}_{t-k} \ge \theta_{\text{admit}} \quad (\text{avec } \theta_{\text{admit}} = 0.985)$$

Si le flux de télémétrie s'interrompt ou si le score s'effondre en dessous du seuil critique, le compte bascule en mode **Probation (Surveillance Renforcée)** : ses prérogatives de vote et d'admission sont immédiatement suspendues.

---

### 4. Préservation Absolue de la Vie Privée : La Barrière ZK

CYNIC refuse catégoriquement d'édifier une base de données biométrique globale.  
Aucune empreinte digitale, aucun enregistrement de frappe brut, aucun tracé d'électrocardiogramme ne quitte jamais la machine locale de l'opérateur.

1. **Calcul Local et Éphémère** : Les données brutes sont traitées en mémoire volatile (RAM sécurisée, mlock) et immédiatement détruites après l'extraction des moments statistiques.
2. **Circuit ZK-SNARK (Aura)** : Le kernel local génère une preuve de connaissance nulle attestant que les invariants de l'équation $\mathcal{H}_t$ sont satisfaits.
3. **Publication On-Chain** : Seule la preuve ZK et l'horodatage Solana sont publiés. Le réseau vérifie que l'acteur est un humain vivant avec une certitude cryptographique totale, **sans jamais savoir qui il est, où il se trouve, ni quelle touche il a pressée.**

L'humanité est prouvée par la physique.  
La souveraineté de l'individu reste inviolée.
