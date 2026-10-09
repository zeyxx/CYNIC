# CYNIC E-Score : Registres de Communauté v0.1

**Status :** Première itération — doctrine en construction.
**Dernière mise à jour :** 2026-07-15

Les registres sont la couche sémantique skinnable du moteur E-Score.
Le moteur est universel. Le langage est choisi par chaque communauté.

Chaque registre définit :
- Les noms d'affichage des 7 stats primaires
- Les noms des 8 classes (facettes émergentes)
- Les noms des 5 tiers d'accès

La mécanique sous-jacente est identique pour tous.

Référence : `CYNIC_ESCORE_SPECIFICATION.md §8`

---

## Clés internes (invariantes cross-registres)

| Clé interne   | Dimension          | Motto universel                             |
|---------------|--------------------|---------------------------------------------|
| `karteria`    | Endurance          | Tu as tenu quand tout mourait               |
| `parrhesia`   | Vérité courageuse  | Tu as dit la vérité contre la foule         |
| `anaideia`    | Audace éhontée     | Tu es allé le premier sans filet            |
| `philanthropia` | Don              | Tu as donné sans attendre de retour         |
| `autarkeia`   | Souveraineté       | Tu maintiens le réseau par toi-même         |
| `poiesis`     | Création           | Tu crées ce qui fait vivre la communauté    |
| `koinonia`    | Communauté         | Tu es la colle humaine entre les individus  |

---

## 1. Registre CYNIC (Philosophie Cynique)

**Vibe :** Souverainiste, intellectuel, cypherpunk. Le registre natif de
l'écosystème. Ancré dans la philosophie des Cyniques grecs — Diogène,
Antisthène — qui affirmaient que la vertu se prouve par l'acte, pas par
le statut. Citoyen du monde (*Kosmopolitês*), non du système.

**Usage naturel :** CYNIC World Order, Odysseum natif, $ASDFASDFA

### Stats

| Clé           | Affichage    | Devise                                    |
|---------------|--------------|-------------------------------------------|
| karteria      | Karteria     | *L'endurance est la seule richesse réelle* |
| parrhesia     | Parrhesia    | *Dis la vérité même si ça brûle*          |
| anaideia      | Anaideia     | *L'éhontée bravoure du premier rang*      |
| philanthropia | Philanthropia| *Donne comme si tu n'en avais pas besoin* |
| autarkeia     | Autarkeia    | *Être sa propre fondation*                |
| poiesis       | Poiesis      | *Créer, c'est résister*                   |
| koinonia      | Koinonia     | *La tribu avant le capital*               |

### Classes

| Classe      | Stats dominantes              | Archétype                                |
|-------------|-------------------------------|------------------------------------------|
| Spartiate   | Karteria + Anaideia           | Tient la ligne ET frappe en premier      |
| Oracle      | Parrhesia + Autarkeia         | Voit la vérité, maintient l'infra        |
| Prométhée   | Anaideia + Philanthropia      | Prend le risque pour donner le feu       |
| Hermès      | Poiesis + Koinonia            | Crée et propage la culture               |
| Dédale      | Autarkeia + Poiesis           | Bâtit en silence, infra + création       |
| Socrate     | Koinonia + Parrhesia          | Pose les bonnes questions à la communauté|
| Agathon     | Philanthropia + Koinonia      | Le cœur généreux de la tribu             |
| **Odysseus**| >= 5 stats >= 60e pct         | *Le citoyen souverain complet*           |

### Tiers d'accès

| Tier | Nom        |
|------|------------|
| 0    | Visiteur   |
| 1    | Citoyen    |
| 2    | Steward    |
| 3    | Archonte   |
| 4    | **Souverain** |

---

## 2. Registre CYPHERPUNK (Hacker / Privacy / Résistance)

**Vibe :** "Code is Speech." Le langage des bâtisseurs de systèmes
incensurables, des cryptographes, des lanceurs d'alerte et des opérateurs
de nœuds underground. Chaque badge est un exploit. Chaque statut est
un niveau d'accès.

**Usage naturel :** Token Chat Control, Aura Protocol, communautés
privacy-first, mouvements de résistance numérique.

### Stats

| Clé           | Affichage | Devise                                       |
|---------------|-----------|----------------------------------------------|
| karteria      | Patience  | *Les systèmes tombent, les daemons tiennent* |
| parrhesia     | Signal    | *Sépare le signal du bruit. Toujours.*       |
| anaideia      | Exploit   | *Zero-day : tu y vas avant que le patch sorte* |
| philanthropia | Relay     | *Tu routes pour les autres. Sans logs.*      |
| autarkeia     | Daemon    | *Tourne en background. Personne ne te voit.* |
| poiesis       | Commit    | *Le code est la seule preuve qui compte*     |
| koinonia      | Mesh      | *Le réseau, c'est toi et tes pairs*          |

### Classes

| Classe        | Stats dominantes      | Archétype                                    |
|---------------|-----------------------|----------------------------------------------|
| Ghost         | Patience + Exploit    | Tient sa position, frappe au bon moment      |
| Oracle        | Signal + Daemon       | Lit la vérité dans le bruit, maintient l'infra|
| Zero-Day      | Exploit + Relay       | Prend le risque, partage l'exploit           |
| Contributor   | Commit + Mesh         | Code pour les autres, connecte le réseau     |
| Sysadmin      | Daemon + Commit       | L'infra tourne grâce à lui                   |
| Whistleblower | Mesh + Signal         | Révèle la vérité à la communauté             |
| Coordinator   | Relay + Mesh          | Donne et connecte sans centraliser           |
| **Sovereign** | >= 5 stats >= 60e pct | *Root. Accès total. Aucune dépendance.*      |

### Tiers d'accès

| Tier | Nom          |
|------|--------------|
| 0    | Lurker       |
| 1    | User         |
| 2    | Operator     |
| 3    | Admin        |
| 4    | **Root**     |

---

## 3. Registre NORDIC (Viking / Guerrier)

**Vibe :** Les anciens Norse ne faisaient pas confiance aux mots — seulement
aux actes en bataille et à la générosité au banquet. Ton E-Score est ton
honneur (*drengskapr*). Une fois perdu, il ne se rachète pas. Les classes
s'inspirent du cosmos nordique : des Nornir aux Konungar.

**Usage naturel :** Communautés maximalistes, tokens guerriers, cultures
de conviction absolue.

### Stats

| Clé           | Affichage   | Devise                                         |
|---------------|-------------|------------------------------------------------|
| karteria      | Uthald      | *L'enclume ne se plaint pas*                   |
| parrhesia     | Spádom      | *La prophétie n'est utile que si elle est juste* |
| anaideia      | Freki       | *Le loup affamé ne demande pas la permission*  |
| philanthropia | Gáfar       | *Le généreux est plus riche que l'avare*       |
| autarkeia     | Sjálfdæmi   | *Nul seigneur au-dessus de soi*               |
| poiesis       | Sköpun      | *Ce que tu bâtis survit à ta mort*            |
| koinonia      | Félag       | *Le *félag* ou mourir seul*                   |

### Classes

| Classe      | Stats dominantes        | Archétype                                      |
|-------------|-------------------------|------------------------------------------------|
| Berserker   | Uthald + Freki          | Endure et frappe. Sans calcul.                 |
| Seiðr       | Spádom + Sjálfdæmi      | Le magicien prophétique, souverain de soi      |
| Skáld       | Freki + Gáfar           | Audacieux ET généreux — le poète de guerre     |
| Smiðr       | Sköpun + Félag          | L'artisan qui forge pour le clan               |
| Dvergar     | Sjálfdæmi + Sköpun      | Le nain-forgeron, maître souverain de son art  |
| Þing-maðr   | Félag + Spádom          | L'homme d'assemblée, dit la vérité au Þing     |
| Húsfreya    | Gáfar + Félag           | La maîtresse du foyer, cœur de la communauté  |
| **Konungr** | >= 5 stats >= 60e pct   | *Le Roi. Élu par les actes, pas par le sang.*  |

### Tiers d'accès

| Tier | Nom          |
|------|--------------|
| 0    | Thrall       |
| 1    | Karl         |
| 2    | Hersir       |
| 3    | Jarl         |
| 4    | **Konungr**  |

---

## 4. Registre DEGEN (Crypto Natif / Accessible)

**Vibe :** La langue de la rue Solana. Aucune friction culturelle.
Les badges parlent directement aux holders de pump.fun, aux CT degens,
aux NFT maxis. Le plus accessible pour onboarder des communautés
qui découvrent Odysseum sans background philosophique.

**Usage naturel :** Pump.fun survivors, CTOs chaotiques, communautés
Solana grand public, onboarding masse.

### Stats

| Clé           | Affichage | Devise                                         |
|---------------|-----------|------------------------------------------------|
| karteria      | Diamond   | *Paper hands rekt. Diamond hands eat.*         |
| parrhesia     | Alpha     | *Real alpha doesn't ask permission*            |
| anaideia      | Chad      | *First in, first blessed*                      |
| philanthropia | Based     | *Genuinely based: gives without wanting clout* |
| autarkeia     | Node      | *You run the machine. You are the machine.*    |
| poiesis       | Cook      | *Always cooking. Never slop.*                  |
| koinonia      | Fren      | *Not your fren, not your community*            |

### Classes

| Classe           | Stats dominantes    | Archétype                                   |
|------------------|---------------------|---------------------------------------------|
| Diamond-Handed   | Diamond + Chad      | Tient ET va en premier. Le holder ultime.   |
| Alpha Node       | Alpha + Node        | L'alpha caller qui fait aussi tourner l'infra|
| Based Chad       | Chad + Based        | Prend le risque pour aider les autres       |
| Content Fren     | Cook + Fren         | Crée du contenu ET connecte le réseau       |
| Builder          | Node + Cook         | Infra + création. Le dev silencieux.        |
| Alpha Caller     | Fren + Alpha        | Partage l'alpha à la communauté             |
| Gigabased        | Based + Fren        | Maximum générosité + communauté             |
| **Sovereign Fren**| >= 5 stats >= 60e pct | *Le degen complet. Respecté de tous.*   |

### Tiers d'accès

| Tier | Nom               |
|------|-------------------|
| 0    | Lurker            |
| 1    | Holder            |
| 2    | Degen             |
| 3    | Whale             |
| 4    | **Gigachad**      |

---

## 5. Registre BUSHIDO (Japonais / Anime)

**Vibe :** Le code d'honneur du guerrier transposé on-chain.
Chaque stat est une vertu du Bushido. Chaque badge est une épreuve.
La classe apex n'est pas conquise — elle est reconnue par les autres.
Fort potentiel viral sur les marchés asiatiques et les communautés anime.

**Usage naturel :** Asian Pacific Solana market, communautés anime/manga,
tokens avec identité culturelle japonaise.

### Stats

| Clé           | Affichage      | Kanji | Devise                                  |
|---------------|----------------|-------|-----------------------------------------|
| karteria      | Kennin         | 堅忍  | *石の意志 — La volonté de la pierre*    |
| parrhesia     | Dōsatsu        | 洞察  | *真実は隠せない — La vérité ne se cache pas* |
| anaideia      | Yūmō           | 勇猛  | *先陣を切る — Ouvrir la voie*           |
| philanthropia | Se / Gi        | 施/義  | *与える喜び — La joie de donner*       |
| autarkeia     | Jiritsu        | 自律  | *自分が主人 — Être son propre maître*   |
| poiesis       | Sōzō           | 創造  | *魂を込めて — Avec toute son âme*      |
| koinonia      | Kizuna         | 絆   | *絆は鋼より強い — Le lien plus fort que l'acier* |

### Classes

| Classe      | Kanji | Stats dominantes    | Archétype                               |
|-------------|-------|---------------------|-----------------------------------------|
| Samurai     | 侍    | Kennin + Yūmō       | Endure et tranche. Honneur absolu.      |
| Kenja       | 賢者  | Dōsatsu + Jiritsu   | Le sage souverain qui voit tout         |
| Gikyō       | 義侠  | Yūmō + Gi           | Le héros chevaleresque, prend risque pour justice |
| Geinin      | 芸人  | Sōzō + Kizuna       | L'artiste qui connecte les âmes         |
| Shokunin    | 職人  | Jiritsu + Sōzō      | Le maître artisan, perfection silencieuse|
| Shi         | 師    | Kizuna + Dōsatsu    | Le maître qui guide par la vérité       |
| Bosatsu     | 菩薩  | Gi + Kizuna         | Le bodhisattva, compassion sans limite  |
| **Daimyō** | 大名  | >= 5 stats >= 60e pct | *Le seigneur reconnu par ses actes.*  |

### Tiers d'accès

| Tier | Nom           | Kanji |
|------|---------------|-------|
| 0    | Minarai       | 見習い |
| 1    | Shi           | 士    |
| 2    | Bushi         | 武士  |
| 3    | Shō           | 将    |
| 4    | **Daimyō**    | 大名  |

---

## 6. Registre GNOSTIC (Mission / Résistance Spirituelle)

**Vibe :** Pour les communautés à cause forte. La résistance numérique
comme acte de foi. Chaque badge est un témoignage. Le langage est
universel et trans-culturel — ni chrétien exclusivement, ni ésotérique —
mais ancré dans l'archétype du porteur de lumière contre les systèmes
opaques.

**Usage naturel :** Mouvements freedom of speech, anti-surveillance,
tokens militants, communautés à mission explicite.

### Stats

| Clé           | Affichage    | Devise                                         |
|---------------|--------------|------------------------------------------------|
| karteria      | Perseverance | *The light holds. Even in darkness.*          |
| parrhesia     | Prophecy     | *Speak truth. Even at cost.*                  |
| anaideia      | Courage      | *First witness. No guarantees.*               |
| philanthropia | Mercy        | *Give as if it were the last time.*           |
| autarkeia     | Stewardship  | *Care for the temple. No one else will.*      |
| poiesis       | Creation     | *Build the world you want to inhabit.*        |
| koinonia      | Fellowship   | *No one is free until all are free.*          |

### Classes

| Classe      | Stats dominantes          | Archétype                                     |
|-------------|---------------------------|-----------------------------------------------|
| Martyr      | Perseverance + Courage    | Tient sous pression, témoin jusqu'au bout     |
| Prophet     | Prophecy + Stewardship    | Voit et maintient. Garde la flamme.           |
| Apostle     | Courage + Mercy           | Va le premier, porte la bonne nouvelle        |
| Evangelist  | Creation + Fellowship     | Crée et propage la vision                     |
| Architect   | Stewardship + Creation    | Bâtit et entretient le sanctuaire             |
| Shepherd    | Fellowship + Prophecy     | Guide la communauté par la vérité             |
| Deacon      | Mercy + Fellowship        | Serviteur du collectif, invisible et essentiel|
| **Patriarch** | >= 5 stats >= 60e pct  | *Gardien des gardiens. Témoin ultime.*        |

### Tiers d'accès

| Tier | Nom           |
|------|---------------|
| 0    | Seeker        |
| 1    | Initiate      |
| 2    | Disciple      |
| 3    | Apostle       |
| 4    | **Patriarch** |

---

## 7. Registre CUSTOM

**Vibe :** Pour les communautés avec une identité suffisamment forte
pour définir leur propre langage. Aucun registre existant ne les représente.

**Processus de création :**
1. La communauté soumet un `registry_proposal` via une proposition futarchique
2. Le schéma doit satisfaire le `escore-registry.schema.json` (voir §8)
3. Chaque clé interne doit être mappée (les 7 stats, les 8 classes, les 5 tiers)
4. Le marché futarchique vote. Si Pass → le registre est inscrit on-chain.

**Contraintes :**
- Les noms d'affichage doivent être distincts des 6 registres officiels
- La classe apex doit être nommée (pas d'index vide)
- Le schéma JSON doit passer la validation avant soumission

---

## 8. Tableau Comparatif

| Registre    | Stat Karteria  | Stat Parrhesia | Classe Apex      | Tier 4        |
|-------------|----------------|----------------|------------------|---------------|
| CYNIC       | Karteria       | Parrhesia      | Odysseus         | Souverain     |
| CYPHERPUNK  | Patience       | Signal         | Sovereign        | Root          |
| NORDIC      | Uthald         | Spádom         | Konungr          | Konungr       |
| DEGEN       | Diamond        | Alpha          | Sovereign Fren   | Gigachad      |
| BUSHIDO     | Kennin 堅忍    | Dōsatsu 洞察   | Daimyō 大名      | Daimyō 大名   |
| GNOSTIC     | Perseverance   | Prophecy       | Patriarch        | Patriarch     |
| CUSTOM      | Défini         | Défini         | Défini           | Défini        |

---

## 9. Références

- [`CYNIC_ESCORE_SPECIFICATION.md`](./CYNIC_ESCORE_SPECIFICATION.md)
- [`escore-registry.schema.json`](./escore-registry.schema.json)
