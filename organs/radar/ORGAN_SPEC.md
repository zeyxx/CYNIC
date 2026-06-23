# Organ: Radar (Veille Technologique)

## Rôle Biologique
Le système sensoriel de l'organisme CYNIC. Cet organe est responsable de la perception de l'environnement extérieur (marchés, protocoles, hébergement des partenaires, mises à jour des blockchains).

## Objectif (Pourquoi ?)
L'écosystème crypto mute à une vitesse vertigineuse (ex: mises à jour furtives des Creator Fees sur Pump.fun, downtimes d'infrastructures partenaires comme Holdex.live). Pour que CYNIC et ses contrats (comme Ignis) ne deviennent jamais des SPOF (Single Point of Failure) obsolètes, l'organisme doit capter la réalité technique et opérationnelle en temps réel, avant de coder.

## Sources de Perception
1. **Protocoles On-Chain :** Changements d'IDL, nouvelles adresses de programmes (Pump.fun, Bonk.fun, Raydium).
2. **Infrastructures Partenaires :** Ping et health checks des API externes (Holdex, RPC providers).
3. **Flux Sociaux / Devs :** Scrapping ciblé (GitHub releases, annonces Discord/Twitter des protocoles clés).

## Flux de Données (Constitution Compliant)
1. **Ingestion :** Les scripts (Python) interrogent les APIs et extraient la donnée brute vers `datasets/raw/radar/`.
2. **Digestion :** Un agent LLM (Hermes/Codex) ou un script déterministe transforme la donnée brute en JSON structuré dans `datasets/curated/` (ex: `pumpfun_physics.json`).
3. **Réflexe :** Si une anomalie critique est détectée (ex: Holdex.live down), une alerte est levée dans le Kernel (`cynic_observe`) pour que le Cortex ajuste la stratégie.

## Architecture
```text
organs/radar/
├── sensors/          # Scripts de collecte (Ping, Scraper, RPC listener)
├── state/            # État local de l'organe (derniers timestamps vérifiés)
└── systemd/          # Services persistants pour tourner 24/7 en tâche de fond
```
