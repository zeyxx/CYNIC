# CYNIC : Pacte des Fondateurs & Genesis Allies

Ce document fige la politique d'allocation interne, le traitement des alliés de la genèse et la gestion des risques humains pour le noyau dur du réseau CYNIC.

Il inscrit la séparation stricte entre l'Entité de Production (Talaria SAS) et l'Entité de Valeur (Fondation / Protocole), neutralisant les failles de gouvernance humaine par l'application d'un *Code is Law* inaltérable.

---

## 1. La Genèse Découplée (TGE)

Lancer une entreprise et un protocole le même jour est une erreur juridique (risque de requalification) et opérationnelle. La genèse est donc scindée en deux événements temporels asynchrones :

*   **T0 (23 Juillet 2026) — On-Chain Genesis :** Le smart contract `cynic-kernel` est déployé. Le réseau commence à respirer de manière autonome. Le chronomètre des 255 mois est initié. Le protocole existe dans la nature, sans attache juridique directe.
*   **T1 (22-24 Août 2026) — Corporate Genesis :** Création légale de TALARIA SAS. L'entreprise naît en tant qu'éditeur logiciel qui décide de s'interfacer avec un protocole *préexistant*. Cela renforce le statut de "Software Editor" face au régulateur.

---

## 2. Le Modèle Temporel Organique (Les 255 Mois)

L'allocation du noyau (Bloc de 38,2 % réservé à l'équipe, la retraite et les alliés) est soumise à une contrainte mathématique absolue (valeur maximale d'un entier non-signé 8-bits `u8`).

*   **Phase 1 : Le Cliff de 76 mois (T0 à Nov 2032)**
    *   0% de jetons libérés. 
    *   *Raison :* Verrouiller l'équipe fondatrice et garantir à la communauté (le bloc de 61,8 %) que l'écosystème ne subira aucune extraction de liquidité interne avant que le réseau ne soit pleinement mature.
*   **Phase 2 : Le Vesting Asymptotique de 178 mois (Nov 2032 à Nov 2047)**
    *   Libération décroissante (géométrique résiduelle) d'environ 1,67 % du solde restant par mois.
    *   *Raison :* Un flux initial fort pour financer la vie des fondateurs au moment de la retraite/maturité, s'aplatissant drastiquement à mesure que l'économie du réseau se densifie (rareté algorithmique).

> **Axiome Technique :** Le "temps" est mesuré exclusivement par le sysvar `Clock::get()?.unix_timestamp` de Solana. L'utilisation des *Slots* ou *Blocks* est interdite pour le macro-temps afin d'éviter toute accélération de la courbe en cas de réduction du temps de bloc (ex: Firedancer).

---

## 3. La "Doctrine Stanislas" (Le Pare-Feu Sanitaire)

Comment récompenser la loyauté originelle d'un frère d'armes ("True Fren") tout en protégeant le projet de son instabilité personnelle (ego, chaos) ? La solution réside dans l'asymétrie structurelle :

### A. TALARIA SAS (Zéro Equity)
Stanislas ne possède **aucune action** dans l'entreprise française.
*   *Justification :* TALARIA est l'usine de production. Un associé minoritaire instable peut bloquer les Assemblées Générales, exiger des audits hostiles ou paralyser l'entreprise par pur ego. Le pare-feu est total. TALARIA reste souveraine.

### B. Le STAMP Offshore (Preuve de Loyauté)
Stanislas reçoit son allocation dans le bloc des 38,2 % au niveau du protocole (Caymans / Fondation), via un *Simple Token Agreement*.
*   *Justification :* Sa contribution originelle au lore de `$ASDF` est honorée sans friction légale corporate. Il est un "Warlord" du réseau, pas un employé.

### C. Le Bouclier Thérapeutique du Cliff
Les jetons de Stanislas sont soumis au même Cliff de 76 mois que le reste du noyau.
*   *Justification :* C'est une protection mutuelle. Stanislas ne peut pas détruire le marché sur un coup de tête impulsif. Parallèlement, ce délai de 6 ans lui offre le temps nécessaire pour accomplir son "processus de dissolution de l'ego". Lorsque la distribution débutera en 2032, l'humain et le réseau seront stabilisés. *Code is Law* remplace la thérapie d'entreprise.
