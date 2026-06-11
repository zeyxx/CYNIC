import time
import json
import requests

KERNEL_URL = "http://localhost:3030/agent-tasks"  # Simulation du CYNIC_REST_ADDR

def fetch_raw_tasks():
    # En réalité, on appellerait le Kernel pour récupérer les tâches au statut "raw"
    return []

def enrich_prompt(raw_text: str) -> str:
    # 1. RAG Injection : On irait chercher du contexte (SurrealDB / llama-embed)
    # 2. Application du Template : (CoT, Contraintes, etc.)
    enriched = f"""<thought>
Tu dois analyser ce problème étape par étape avant d'écrire du code.
</thought>
<constraints>
- Sécurité: Pas d'injection SQL
- RGPD: Minimiser la collecte de données
</constraints>

[TÂCHE UTILISATEUR]
{raw_text}
"""
    return enriched

def main():
    print("[organ-prompt-enricher] Démarrage du service d'écoute du Kernel...")
    while True:
        # Boucle de supervision infinie (Flawless Loop)
        tasks = fetch_raw_tasks()
        for task in tasks:
            print(f"Interception de la tâche : {task['id']}")
            enriched_text = enrich_prompt(task['prompt'])
            
            # On met à jour la tâche dans le Kernel au statut "enriched"
            # requests.put(f"{KERNEL_URL}/{task['id']}", json={"prompt": enriched_text, "status": "enriched"})
            print(f"Tâche {task['id']} enrichie et remise dans le Mempool.")
            
        time.sleep(5) # Polling (ou WebSockets dans une vraie infra)

if __name__ == "__main__":
    main()
