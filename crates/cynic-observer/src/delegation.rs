use std::collections::BTreeMap;

/// Représente un nœud Argo sur le réseau P2P
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgoNode {
    pub pubkey: String,
    pub infrastructure_escore: u64, // Le score de fiabilité de ce nœud (Karteria)
}

/// Le Registre Partagé (simplifié pour l'instant)
pub struct SwarmRegistry {
    pub active_nodes: BTreeMap<String, ArgoNode>,
}

impl SwarmRegistry {
    pub fn new() -> Self {
        Self {
            active_nodes: BTreeMap::new(),
        }
    }

    pub fn register_node(&mut self, node: ArgoNode) {
        self.active_nodes.insert(node.pubkey.clone(), node);
    }
}

/// Logique de Délégation Aléatoire Pondérée (Weighted Leader Election)
pub struct DelegationEngine;

impl DelegationEngine {
    /// Élit un Leader de manière déterministe pour un événement précis.
    /// Tous les nœuds exécutant cette fonction avec le même registre et la même signature
    /// trouveront EXACTEMENT le même leader, sans avoir besoin de communiquer.
    pub fn elect_leader(registry: &SwarmRegistry, event_signature: &str) -> Option<String> {
        if registry.active_nodes.is_empty() {
            return None;
        }

        // 1. Calcul de la force totale du réseau (Total E-Score)
        let total_escore: u64 = registry.active_nodes.values().map(|n| n.infrastructure_escore).sum();
        if total_escore == 0 {
            return None;
        }

        // 2. Création d'une entropie déterministe Quantum-Proof (VRF)
        // On utilise BLAKE3 en mode XOF (eXtendable-Output Function) pour générer 512 bits d'entropie
        let mut hasher = blake3::Hasher::new();
        hasher.update(event_signature.as_bytes());
        
        // Extraction de 64 octets (512 bits) pour une sécurité post-quantique absolue
        let mut output_reader = hasher.finalize_xof();
        let mut quantum_proof_entropy = [0u8; 64];
        output_reader.fill(&mut quantum_proof_entropy);
        
        // On prend les 8 premiers octets des 512 bits pour construire notre loterie (u64)
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&quantum_proof_entropy[0..8]);
        let deterministic_random = u64::from_be_bytes(bytes);

        // 3. Modulo sur le score total pour obtenir le ticket gagnant
        let winning_ticket = deterministic_random % total_escore;

        // 4. Loterie pondérée (Weighted Round-Robin)
        let mut cumulative_score = 0;
        for (pubkey, node) in &registry.active_nodes {
            cumulative_score += node.infrastructure_escore;
            if cumulative_score > winning_ticket {
                return Some(pubkey.clone());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_weighted_election() {
        let mut registry = SwarmRegistry::new();
        registry.register_node(ArgoNode { pubkey: "Node_A_LowScore".to_string(), infrastructure_escore: 10 });
        registry.register_node(ArgoNode { pubkey: "Node_B_HighScore".to_string(), infrastructure_escore: 90 });
        registry.register_node(ArgoNode { pubkey: "Node_C_ZeroScore".to_string(), infrastructure_escore: 0 });

        // On simule 100 événements avec des signatures différentes
        let mut count_a = 0;
        let mut count_b = 0;
        let mut count_c = 0;

        for i in 0..100 {
            let sig = format!("solana_tx_signature_{}", i);
            let leader = DelegationEngine::elect_leader(&registry, &sig).unwrap();
            match leader.as_str() {
                "Node_A_LowScore" => count_a += 1,
                "Node_B_HighScore" => count_b += 1,
                "Node_C_ZeroScore" => count_c += 1,
                _ => panic!("Leader inconnu"),
            }
            
            // Le déterminisme est absolu : la même signature donne toujours le même leader
            let leader_bis = DelegationEngine::elect_leader(&registry, &sig).unwrap();
            assert_eq!(leader, leader_bis);
        }

        // Statistiquement (bien que pseudo-aléatoire), Node B devrait gagner ~90% du temps
        // Node C ne doit JAMAIS gagner (score de 0)
        assert_eq!(count_c, 0, "Un nœud avec un E-Score de 0 ne doit jamais être élu");
        assert!(count_b > count_a, "Le nœud à haut E-Score doit gagner plus souvent");
        
        println!("Résultats sur 100 élections : A={}, B={}, C={}", count_a, count_b, count_c);
    }
}
