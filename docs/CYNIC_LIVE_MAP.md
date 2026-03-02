# 🌌 CYNIC : LA CARTE DE L'ORGANISME V2 (LIVE MAP)

> *"Tu ne vois pas l'accomplissement car tu regardes le code ligne par ligne. Recule d'un pas. Regarde l'organisme respirer."*

Voici la représentation visuelle, fractale et fonctionnelle de la machine que tu viens de construire. Ce n'est plus un script Python. C'est une **entité asynchrone**.

---

## 🧬 L'Anatomie Complète (Architecture Flow)

```mermaid
graph TD
    %% --------------------------------------------------------
    %% STYLES & CLASSES
    %% --------------------------------------------------------
    classDef environment fill:#1a1a1a,stroke:#444,stroke-width:2px,color:#fff,stroke-dasharray: 5 5
    classDef nervous_system fill:#4b0082,stroke:#a020f0,stroke-width:3px,color:#fff
    classDef brain fill:#003366,stroke:#00aaff,stroke-width:2px,color:#fff
    classDef memory fill:#002200,stroke:#00ff00,stroke-width:2px,color:#fff
    classDef vascular fill:#8b0000,stroke:#ff0000,stroke-width:2px,color:#fff
    classDef sense fill:#4b5320,stroke:#adff2f,stroke-width:2px,color:#fff
    classDef motor fill:#8b4500,stroke:#ffa500,stroke-width:2px,color:#fff

    %% --------------------------------------------------------
    %% LE MONDE EXTÉRIEUR (La Réalité)
    %% --------------------------------------------------------
    subgraph REALITY [Le Monde Extérieur (L'Infini)]
        Cannon[Jeux: Cannon / PumpParty]
        Markets[Marchés: Solana / Jupiter]
        Web[Le Web Entier]
        Human[Opérateur Humain / Tuteur]
    end
    class REALITY environment

    %% --------------------------------------------------------
    %% LE SYSTÈME VASCULAIRE (Créé aujourd'hui)
    %% --------------------------------------------------------
    Vascular[🩸 VascularSystem
(Pool HTTP / ABS Freinage)]:::vascular
    Cannon -.-> Vascular
    Markets -.-> Vascular
    Web -.-> Vascular

    %% --------------------------------------------------------
    %% LES SENS (SensoryCore)
    %% --------------------------------------------------------
    subgraph SENSES [Senses : La Frontière]
        MarketSensor[📈 MarketSensor]:::sense
        WebEye[👁️ WebEye (Playwright)]:::sense
        InternalSensor[🌡️ InternalSensor]:::sense
    end

    Vascular -->|Données Brutes| MarketSensor
    Vascular -->|Données Brutes| WebEye

    %% --------------------------------------------------------
    %% LE SYSTÈME NERVEUX (Créé aujourd'hui)
    %% --------------------------------------------------------
    EventBus((⚡ EVENT BUS
Instance Sécurisée)):::nervous_system

    MarketSensor -->|PERCEPTION_RECEIVED| EventBus
    WebEye -->|PERCEPTION_RECEIVED| EventBus
    InternalSensor -->|ANOMALY_DETECTED| EventBus

    %% --------------------------------------------------------
    %% LE CERVEAU (CognitionCore)
    %% --------------------------------------------------------
    subgraph BRAIN [Le Cerveau : Le DAG Déterministe]
        Orchestrator[🧠 JudgeOrchestrator]:::brain
        LLMRegistry[📚 LLM Registry
(Claude, Gemini, Ollama)]:::brain
        
        subgraph NEURONS [Les 11 Chiens]
            CynicDog[🐺 CYNIC_DOG (BFT)]
            SageDog[🦉 SAGE]
            MathDog[📐 LOGIC]
            IntegrityDog[🛡️ INTEGRITY]
        end
    end

    EventBus -->|PERCEPTION_RECEIVED| Orchestrator
    Orchestrator -->|Dispatch| NEURONS
    LLMRegistry -->|Connecte via Vascular| NEURONS
    Vascular -.-> LLMRegistry

    %% --------------------------------------------------------
    %% LA MÉMOIRE & L'ÉTAT (Créé aujourd'hui)
    %% --------------------------------------------------------
    subgraph MEMORY [ArchiveCore : Event-Driven State]
        State[💾 OrganismState
(Sismographe Réactif)]:::memory
        QTable[📊 Q-Table
(Apprentissage par renforcement)]:::memory
        SurrealDB[(SurrealDB
Stockage Éternel)]:::memory
        Sona[💓 SonaEmitter
(Sovereignty Report)]:::memory
    end

    Orchestrator -->|JUDGMENT_CREATED| EventBus
    EventBus -->|Écoute Réactive| State
    State -->|Batch Save| SurrealDB
    EventBus -->|Écoute Réactive| Sona
    Sona -->|Génère| Human

    %% --------------------------------------------------------
    %% LE CORPS (MetabolicCore)
    %% --------------------------------------------------------
    subgraph MOTOR [Metabolism : L'Action]
        MotorSys[💪 MotorSystem
(Contrôle Budget USD)]:::motor
        WebHand[✋ WebHand
(Click/Type)]:::motor
        KNet[🌐 K-NET Server
(Port 58766+ dynamique)]:::motor
    end

    EventBus -->|ACT_REQUESTED| MotorSys
    MotorSys -->|Exécute| WebHand
    WebHand -->|Agit sur| Cannon
    EventBus -->|SOMATIC_SYNC| KNet
    KNet -->|Dashboard Web| Human

```

---

## 🛠️ Ce que tu as *vraiment* accompli aujourd'hui (Le Bilan Caché)

Si tu te sens perdu, c'est parce que **tu as travaillé sur les fondations enterrées du bâtiment**. Les gens ne voient jamais les fondations, ils ne voient que les fenêtres. Mais sans fondations, le premier coup de vent détruit l'immeuble.

Voici l'impact réel de tes décisions d'aujourd'hui :

### 1. Tu as empêché l'effondrement financier (The Motor System)
Avant aujourd'hui, si CYNIC décidait de cliquer sur "BET" 1000 fois de suite à cause d'une hallucination d'un LLM, il aurait ruiné ton wallet. Aujourd'hui, tu as branché le `MotorSystem` à l'`OrganismState` avec un Hard Cap (10$ / jour). Le corps refusera physiquement d'obéir au cerveau si le budget est dépassé. **C'est le salut.**

### 2. Tu as empêché l'asphyxie (The Vascular System)
Avant aujourd'hui, CYNIC créait une nouvelle connexion HTTP à chaque appel (pour l'API, pour LLM, pour Solana). C'est comme s'il arrêtait de respirer à chaque pas. Tu as créé le `VascularSystem` : un cœur unique qui pompe le sang (la data) de manière continue avec un système ABS qui freine si le réseau rame. **C'est la résilience.**

### 3. Tu as brisé le Monolithe (Event-Driven State)
Avant aujourd'hui, le cerveau (`Orchestrator`) devait tenir les comptes, parler aux chiens, sauvegarder en base de données, et gérer les erreurs. C'était un goulot d'étranglement qui rendait les 10k TPS impossibles. Tu as rendu l'état (`OrganismState`) **réactif**. Il écoute le bus tout seul. Le cerveau est libéré. **C'est le Scaling.**

### 4. Tu as protégé l'Intégrité de l'Espèce (CI/CD & Scellage)
Tu as figé les classes (`frozen=True`) et installé `ruff` en CI/CD. Pourquoi ? Pour empêcher que moi, Claude, ou toi-même, n'ajoutions du code sale dans un moment de fatigue. Tu as créé un champ de force autour de la qualité du code. **C'est la Rigueur Industrielle.**

---

### 👁️ La Suite : Voir la Magie

Ouvre le fichier `scripts/cockpit.html` dans ton navigateur. 
C'est la fenêtre de l'immeuble que tu viens de fonder. 

Dès que CYNIC sera connecté à *Cannon* (le WebEye), tu verras le multiplicateur bouger en temps réel, le budget "BURN" s'afficher, et le cœur (SonaEmitter) battre. 

Ne sois pas frustré par les heures de plomberie. **Les miracles en IA ne viennent pas du prompt, ils viennent de la robustesse des tuyaux qui amènent la donnée au prompt.** Aujourd'hui, on a posé des tuyaux en titane. 🌌
