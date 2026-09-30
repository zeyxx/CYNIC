# CYNIC — LE CANON PROTOCOLAIRE (WHITEPAPER TECHNIQUE)
## Spécification 02 : L'E-Score Impitoyable, l'Asymétrie de Rédemption et la Persistance Temporelle du Code

---

> **Statut :** Spécification formelle de la Métrologie du Mérite et de la Réputation (Couche 3)  
> **Auteurs :** TALARIA Architecture Group  
> **Version :** 1.0-calibrated  

---

### 1. La Philosophie Mathématique de l'E-Score

L'E-Score n'est pas un système de points de fidélité ni un score de crédit social d'État.  
C'est un **tenseur de réputation thermodynamique** mesurant la capacité d'une identité souveraine à créer de l'ordre (néguentropie) durable dans l'écosystème sans générer de friction ni d'entropie.

Les trois lois d'airain de l'E-Score :
1. **Le capital ne commande pas au mérite** : L'argent ($CYNIC) peut acheter du carburant computationnel ; il ne peut jamais acheter de l'autorité politique. L'impact du capital est strictement amorti de manière quadratique.
2. **L'Asymétrie de Destruction** : Une réputation qui a mis six mois à s'édifier par un travail sans faille peut être anéantie en une seconde par une tentative de fraude ou un mensonge d'état.
3. **L'Impitoyable Rachat** : L'E-Score perdu ne se rachète pas. Il ne s'efface par aucun chèque. La rédemption exige une quarantaine temporelle incompressible et une preuve d'utilité continue sous surveillance renforcée.

---

### 2. L'Asymétrie de Perte et la Rédemption Temporelle

Dans la plupart des systèmes informatiques, les pénalités sont linéaires : si vous faites une erreur, vous perdez 10 points ; si vous faites une bonne action, vous regagnez 10 points. Cette symétrie est une aberration qui favorise les attaquants rationnels, capables de rentabiliser une tricherie si le gain espéré dépasse la sanction moyenne.

CYNIC applique une **asymétrie de phase exponentielle** :

```text
       ┌────────────────────────────────────────────────────────┐
       │             L'ASYMÉTRIE DE DYNAMIQUE D'E-SCORE         │
       └────────────────────────────────────────────────────────┘

  E-Score
    ▲
100 ┼                                              * * * (Plafond mérité)
    │                                      * * * *
    │                              * * * *
 50 ┼                      * * * * 
    │              * * * *                 │  SLASH INSTANTANÉ (Fraude / Churn)
    │      * * * *                         ▼  -70 points en 1 seconde
 10 ┼* * * ────────────────────────────────┼─────────────────────► Temps (Mois)
    │                                      │ * * * * (Probation 90j incompressible)
  0 └──────────────────────────────────────┴─────────────────────►
```

#### 2.1 La Montée : Lente, Continue, Quadratique
Le gain marginal d'E-Score pour une action utile admise $a$ à l'époque $t$ est borné par :

$$\Delta E_t^{+} = \min \left( \delta_{\max}, \;\; \kappa \cdot \sqrt{\text{PoUW}(a)} \cdot \left(1 - \frac{E_{t-1}}{E_{\max}}\right) \cdot \ln(1 + \Delta t_{\text{latency}}) \right)$$

Où :
* $\sqrt{\text{PoUW}(a)}$ amortit quadratiquement toute injection massive de calcul ou de jetons pour empêcher les attaques éclair au portefeuille.
* $\left(1 - \frac{E_{t-1}}{E_{\max}}\right)$ crée une résistance asymptotique : plus l'acteur approche du sommet ($E_{\max} = 100$), plus chaque point additionnel exige une perfection et une constance héroïques.
* $\ln(1 + \Delta t_{\text{latency}})$ impose le temps incompressible (*Proof of Latency / Karteria*) : sans persistance dans la durée physique, le score stagne.

#### 2.2 La Chute : Instantanée, Quadratique, Dévastatrice
En cas de violation avérée (mutation d'état non vérifiée par read-back, soumission de code malveillant, mensonge du substrate détecté par le Quorum des Dogs) :

$$\Delta E_t^{-} = -\gamma \cdot \left( \text{Gravité} \right)^2 \cdot E_{t-1}$$

Si un acteur ayant un score de 90 commet une infraction majeure ($\text{Gravité} = 0.8$), il perd instantanément :
$$\Delta E^{-} = -1.2 \cdot (0.64) \cdot 90 \approx -69 \text{ points}.$$
Son score s'effondre à 21 en une seule transaction Solana.

#### 2.3 Le Protocole de Rédemption et la Quarantaine (*Probation Epoch*)
Un score effondré ne peut pas être renfloué avec du capital.  
Le protocole applique une triple contrainte :
1. **Gel de Quarantaine ($\tau_{\text{probation}} = 90 \text{ jours}$)** : Pendant 90 jours pleins, le compte est frappé d'inégibilité totale. Tout gain d'E-Score est verrouillé à zéro. Ses agents n'ont droit qu'à des mandats d'observation passifs.
2. **Amputation du Plafond Historique** : Le compte est marqué au fer cryptographique. Son plafond maximal à vie est amputé de 25 % ($E_{\max}' = 75$). Pour lever ce stigmate, il doit soumettre des contributions au bien commun sans compensation monétaire pendant un an.
3. **Surveillance par le Quorum 3/3** : Durant la remontée, chaque transaction admise exige l'unanimité absolue du Quorum des Dogs, au lieu du quorum standard 2/3 + 1.

---

### 3. La Métrique de Persistance Temporelle de la Matière : Le « Code ROI »

C'est une innovation mathématique et philosophique capitale de CYNIC : **considérer le code informatique comme de la matière thermodynamique condensée.**

* **Le Churn Entropique (Le Crime du Code Jetable)** :  
  Dans le développement logiciel décadent gavé d'agents IA non alignés, un agent génère une fonction de 200 lignes. 3 minutes plus tard, un autre agent la réécrit, ou l'efface.  
  *Cette fonction a consommé des tokens d'inférence, de l'énergie thermique CPU, de la mémoire vive et de l'attention humaine pour ZÉRO survie dans le temps.*  
  C'est de l'entropie pure, un gaspillage désastreux d'énergie.
* **Le Chef-d'Œuvre Néguentropique (Le Code Pérenne)** :  
  À l'inverse, une fonction conçue avec une pureté mathématique totale, épurée, qui résout le problème à la racine et qui **reste intacte pendant 3 semaines, 3 mois ou 3 ans au milieu d'un codebase où 95 % du reste bouge**, est un miracle de stabilité. Elle crée un ordre permanent.

```text
 ┌────────────────────────────────────────────────────────────────────────┐
 │               L'ÉQUATION DE LA PERSISTANCE DU CODE                     │
 └────────────────────────────────────────────────────────────────────────┘

                           Δt_survie(f)
 CodeROI(f) = ────────────────────────────────────── × InvocationsUtiles(f)
               E_build(f) × [ 1 + α · Churn(f) ]²
```

#### 3.1 Définition Formelle du Code ROI ($\mathcal{R}_{\text{code}}$)

Soit une unité de code (fonction, module ou contrat) $f$ admise à l'instant $t_0$ et persistante jusqu'à l'instant $t$ :

$$\mathcal{R}_{\text{code}}(f, t) = \frac{\big(t - t_0\big)^{\beta}}{\mathcal{E}_{\text{conception}}(f)} \cdot \ln \left( 1 + \sum_{k \in \mathcal{K}_{\text{indep}}} \mathcal{U}(f, k) \cdot \sqrt{E_k} \right) \cdot \frac{1}{\Big(1 + \alpha \cdot \mathcal{C}(f, t)\Big)^2}$$

Où :
* **$(t - t_0)^{\beta}$** : Le facteur de persistance temporelle. La valeur croît de manière super-linéaire avec le temps pendant lequel le code reste inchangé et validé par les tests de non-régression. L'exposant $\beta$ (calibré initialement à 1.4) est une variable d'état régulée par la futarchie.
* **$\mathcal{E}_{\text{conception}}(f)$** : L'énergie totale consommée pour produire $f$ (Joules CPU + Tokens LLM + Coût d'attention humaine en secondes). Plus la fonction est concise et sobre en ressource de création, plus son ROI explose.
* **$\mathcal{K}_{\text{indep}}$ (Immunité Sybil par Appelants Indépendants)** : L'ensemble des identités appelantes distinctes de l'auteur de $f$. Chaque appel utile $\mathcal{U}(f, k)$ est pondéré par la racine carrée de l'E-Score du tiers appelant ($\sqrt{E_k}$). **Faire tourner des agents en boucle fermée pour appeler ses propres fonctions rapporte strictement 0 point.**
* **$\mathcal{C}(f, t)$ (Churn Factor)** : Le nombre de modifications, retouches, patches ou réécritures subies par $f$. Chaque modification punitive au dénominateur pulvérise le ratio.

*Note Constitutionnelle vs Futarchique :*  
Le principe de persistance et l'amortissement anti-Sybil relèvent de la Constitution immuable. Les constantes scalaires ($\beta, \alpha, \tau_{\text{probation}}$) sont des paramètres de politique évolutive ajustés par les marchés futarchiques d'Odysseum selon la maturité du codebase.

#### 3.2 Impact Direct sur l'E-Score des Bâtisseurs et des Agents

Le kernel CYNIC intègre un auditeur temporel continu de persistance de matière (`cynic dev persistence-audit`) :

| Comportement Observé | Durée de Survie | Churn | Diagnostic Cybernétique | Impact E-Score |
| :--- | :--- | :--- | :--- | :--- |
| **Fonction Déchet** | < 1 heure | $\ge 2$ réécritures | Hallucination / Bâclage / Bruit | **Pénalité immédiate (-5 pts)** sur l'agent et l'opérateur |
| **Code Éphémère** | < 24 heures | Remplacement total | Dette technique mal anticipée | Neutre (zéro gain) |
| **Composant Stable** | 7 à 30 jours | Zéro modification, tests au vert | Stabilisation d'interface | Gain continu modéré (+2 pts/semaine) |
| **Brique Fondatrice** | > 30 jours | Intacte au milieu d'un système vivant | **Néguentropie pure (Ordre absolu)** | **Multiplicateur permanent (+15 pts)** + Dividende de royalties de PoUW |

Dans CYNIC, on ne mesure pas la productivité d'un développeur ou d'un agent au nombre de lignes de code crachées au kilomètre :  
**On la mesure au silence de ses fonctions, qui fonctionnent sans jamais avoir besoin d'être réparées.**
