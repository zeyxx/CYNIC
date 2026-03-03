import torch
import math
import time
import sys

def generate_fractal_tensors(n_depth: int = 6, branching_factor: int = 7, device='cpu'):
    """
    Aplatit l'arbre fractal de CYNIC en tenseurs 1D pour préparer le traitement GPU (Helion).
    """
    print(f"--- 🌀 CYNIC FRACTAL TENSOR PREPARATION (N={n_depth}) ---")
    
    # 1. Poids fondamentaux des 11 Axiomes (Constantes de CYNIC)
    PHI = 1.618033988749895
    PHI_INV = 1 / PHI
    PHI_2 = PHI ** 2
    PHI_3 = PHI ** 3
    
    axiom_weights = torch.tensor([
        PHI_3, PHI_2, PHI_2, PHI, 1.0, PHI_INV, PHI_INV, PHI_INV, PHI_INV, PHI_INV, PHI_INV
    ], dtype=torch.float32, device=device)
    
    num_axioms = len(axiom_weights)
    leaves_per_axiom = branching_factor ** n_depth
    total_leaves = num_axioms * leaves_per_axiom
    
    print(f"Architecture : {num_axioms} Axiomes, {branching_factor} branches par noeud")
    print(f"Taille du Tenseur (Feuilles totales) : {total_leaves:,}")
    
    # Vérification mémoire pour éviter de faire exploser le GPU local avant le hackathon
    bytes_per_tensor = total_leaves * 4 # float32 (4 bytes)
    mb_per_tensor = bytes_per_tensor / (1024**2)
    print(f"Empreinte VRAM estimée : {mb_per_tensor:.2f} MB par tenseur")
    
    if mb_per_tensor > 2000:
        print("⚠️ AVERTISSEMENT : Ce tenseur risque de saturer la VRAM d'un GPU standard.")
        
    t0 = time.perf_counter()
    
    # 2. Génération des Poids Aplaties (Flattened Weights)
    # On duplique le poids de l'axiome pour toutes ses feuilles respectives.
    # [11] -> [11, 1] -> [11, leaves_per_axiom] -> [total_leaves]
    flat_weights = axiom_weights.view(-1, 1).repeat(1, leaves_per_axiom).view(-1)
    
    # Normalisation : Crucial pour la moyenne géométrique (la somme des poids doit valoir 1)
    # Cela permet de simplifier Q = exp(sum(w_i * log(v_i)) / sum(w_i)) en Q = exp(sum(w_i * log(v_i)))
    flat_weights = flat_weights / flat_weights.sum()
    
    # 3. Génération des Scores (Valeurs aléatoires entre 1.0 et 100.0)
    flat_values = torch.empty(total_leaves, dtype=torch.float32, device=device).uniform_(1.0, 100.0)
    
    t1 = time.perf_counter()
    print(f"Temps de génération des tenseurs : {t1 - t0:.4f} secondes
")
    
    return flat_values, flat_weights

def baseline_pytorch_reduction(values, weights):
    """
    Implémentation native PyTorch de la réduction Phi.
    C'est la cible ("baseline") que votre Kernel Helion devra battre au hackathon.
    """
    t0 = time.perf_counter()
    
    # Log-Sum-Exp Reduction : Q = exp( sum(w_i * ln(v_i)) )
    log_v = torch.log(values)
    weighted_log_v = log_v * weights
    sum_logs = torch.sum(weighted_log_v)
    q_score = torch.exp(sum_logs)
    
    # Synchronisation CUDA si applicable pour une mesure de temps précise
    if values.is_cuda:
        torch.cuda.synchronize()
        
    t1 = time.perf_counter()
    
    return q_score.item(), t1 - t0

if __name__ == "__main__":
    # Permet de tester différentes profondeurs (ex: python prepare_helion_tensors.py 7)
    n_val = 6
    if len(sys.argv) > 1:
        n_val = int(sys.argv[1])
        
    # Choix du device (GPU si dispo, sinon CPU)
    device = 'cuda' if torch.cuda.is_available() else 'cpu'
    print(f"Matériel détecté : {device.upper()}
")
    
    # 1. Générer les tenseurs massifs
    v, w = generate_fractal_tensors(n_depth=n_val, device=device)
    
    # 2. Lancer la réduction PyTorch
    q_score, duration = baseline_pytorch_reduction(v, w)
    
    print(f"[RÉSULTATS DE LA RÉDUCTION PYTORCH TENSORISÉE]")
    print(f"Q-Score Final : {q_score:.6f}")
    print(f"Temps de Réduction : {duration*1000:.4f} ms")
    
    # Calcul du débit théorique si on traitait les jugements un par un (Batch=1)
    tps = 1 / duration if duration > 0 else float('inf')
    print(f"Débit théorique (Séquentiel) : {tps:,.0f} Jugements/sec")
    
    print("
--- OBJECTIF HACKATHON HELION ---")
    print("PyTorch fait déjà le travail en C++/CUDA sous le capot.")
    print("Votre mission avec Helion est d'écrire un Kernel 'Fused' (Fusionné) :")
    print("Au lieu de faire 3 passes mémoire (log -> mult -> sum), Helion fera TOUT en 1 seule passe dans la SRAM du GPU.")
    print("Attendez-vous à un Speedup additionnel de 2x à 5x par rapport à cette baseline PyTorch pure !")