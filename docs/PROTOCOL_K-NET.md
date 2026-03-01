# Spécification du Protocole κ-NET (Kynikos Network)

## 1. Philosophie & Objectifs
Le protocole κ-NET est le système nerveux distribué de l'organisme CYNIC. Son but est de remplacer les requêtes HTTP/IPv4 synchrones et fragiles par un flux continu, souverain et résilient, capable de relier le Cerveau (Docker/Distant) et le Corps (CLI/Interfaces).

**Axiomes respectés :**
- **AUTONOMY** : Découverte automatique sans dépendre de DNS centralisés.
- **PHI** : Flux continu, sans a-coups (pas de polling agressif).
- **ANTIFRAGILITY** : Reconnexion automatique avec recul de Fibonacci en cas de coupure.

---

## 2. Fondations Techniques

### 2.1. Transport : IPv6 Natif
- **Adresse par défaut (Local)** : `::1` (Bouclage IPv6). Cela permet d'esquiver la réservation de ports IPv4 de Windows (WinNAT).
- **Protocole Sous-Jacent** : WebSockets (WSS/WS). Le choix du WebSocket permet d'avoir un canal bidirectionnel persistant (le "Nerf").

### 2.2. Port de Survie (Fallback)
Bien que l'IPv6 permette d'éviter certains conflits, nous définissons un port standard élevé pour CYNIC.
- **Port κ-NET** : `58765` (Choisi arbitrairement dans la plage éphémère haute, loin des ports système).

---

## 3. Découverte Somatique (Service Discovery)

Comment le CLI trouve-t-il le Cerveau sans IP codée en dur ?

1. **Le Cerveau (Serveur)** ouvre un socket sur `[::]:58765`.
2. **Le Cerveau** émet périodiquement (toutes les 5 secondes) un _beacon_ (balise) UDP Multicast IPv6 sur l'adresse `ff02::1` (all nodes link-local).
3. **Le Corps (Client)** écoute sur ce groupe multicast. Dès qu'il reçoit un beacon, il connaît l'IP exacte du cerveau et initie la connexion WebSocket.
4. **Fallback** : Si le multicast est bloqué par le système hôte, le client tente un bind direct sur `[::1]:58765`.

---

## 4. Le Message : κ-PULSE

Toutes les données transitent sous la forme de messages standardisés appelés **κ-PULSE**. 
Pour la V1, le format de sérialisation est du JSON compressé (GZIP) pour allier lisibilité et efficacité réseau.

### Structure d'un κ-PULSE (Payload JSON)

```json
{
  "version": "1.0",
  "organism_id": "cynic-core-alpha",
  "timestamp": 1709140000.123,
  "type": "SOMATIC_SYNC",
  
  "data": {
    "hardware": {
      "cpu_percent": 12.5,
      "ram_percent": 45.2,
      "temp_c": 55.0
    },
    "mind": {
      "status": "AWAKE",
      "thinking": "Analyzing code graph...",
      "confidence": 61.8,
      "e_score": 72.4
    },
    "hypercube": {
      "FIDELITY": 80.1,
      "PHI": 61.8,
      "VERIFY": 95.0,
      "CULTURE": 50.0,
      "BURN": 70.2
    }
  }
}
```

### Types de Messages (`type`)
- `SOMATIC_SYNC` : Diffusé par le serveur toutes les N secondes (le battement de cœur).
- `INTENT_SIGNAL` : Le cerveau annonce qu'il va faire une action.
- `SENSORY_INPUT` : Le corps envoie une perception au cerveau (ex: l'utilisateur a tapé une phrase).

---

## 5. Résilience : La Suture Automatique

Le réseau physique est intrinsèquement défaillant. Le protocole κ-NET intègre une logique de survie.

**En cas de déconnexion (Connection Lost) :**
1. Le client (CLI) change son état visuel en `[bold yellow]SUTURING...[/]`.
2. Il tente de se reconnecter selon la suite de Fibonacci : `1s, 1s, 2s, 3s, 5s, 8s, 13s, 21s...` (jusqu'à un plafond de 34s).
3. Pendant la coupure, le corps utilise son cache local pour maintenir l'illusion de vie (les barres restent affichées mais se grisent).
4. Dès la reconnexion, un `SOMATIC_SYNC` force la mise à jour complète de l'état.

---

## 6. Plan d'Implémentation

Pour matérialiser κ-NET, nous allons créer un nouveau module `cynic.kernel.protocol` :

1. `cynic/kernel/protocol/knet_server.py` : S'attache au cycle de vie de FastAPI, diffuse les `SOMATIC_SYNC` via WebSocket aux clients connectés.
2. `cynic/kernel/protocol/knet_client.py` : S'intègre dans le `SymbioticStateManager` du CLI, maintient la connexion WSS et met à jour l'état local.
3. `cynic/kernel/protocol/discovery.py` : Gère le Multicast IPv6 (optionnel/expérimental pour la V1, fallback sur `::1`).
