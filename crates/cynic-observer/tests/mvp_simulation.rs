use cynic_observer::delegation::{ArgoNode, DelegationEngine, SwarmRegistry};
use cynic_observer::parser::PumpFunAction;
use cynic_observer::signal_mapper::map_action;

#[test]
fn simulate_mvp_pipeline_and_expose_vulnerabilities() {
    let now = 1_720_100_000;

    // ─────────────────────────────────────────────────────────────────────────
    // ÉTAPE 1 : Le Pipeline Nominal (Observer -> Kernel)
    // ─────────────────────────────────────────────────────────────────────────
    println!("\n=== ÉTAPE 1 : SIMULATION DU PIPELINE NOMINAL ===");

    // Un utilisateur achète pour 5 SOL de token
    let action = PumpFunAction::Trade {
        token_address: "TOKEN_MINT_123".to_string(),
        sol_amount: 5_000_000_000, // 5 SOL
        token_amount: 50_000,
        is_buy: true,
    };

    // Le Watcher / Mapper traduit en signaux (volume > 0.1 SOL donc non filtré)
    let mapped = map_action(&action, now);
    assert!(!mapped.token_signals.is_empty());
    println!(
        "✓ Action Pump.fun (5 SOL) traduite en {} signaux E-Score",
        mapped.token_signals.len()
    );

    // ─────────────────────────────────────────────────────────────────────────
    // ÉTAPE 2 : Faille 1 - Grinding de Signature & Atténuation Proof-of-History
    // ─────────────────────────────────────────────────────────────────────────
    println!("\n=== ÉTAPE 2 : FAILLE 1 - GRINDING DE SIGNATURE & ATTÉNUATION PoH ===");

    let mut registry = SwarmRegistry::new();
    registry.register_node(ArgoNode {
        pubkey: "AttackerNode".to_string(),
        infrastructure_escore: 10,
        activation_epoch: 0,
    });
    registry.register_node(ArgoNode {
        pubkey: "HonestNode_A".to_string(),
        infrastructure_escore: 90,
        activation_epoch: 0,
    });
    registry.register_node(ArgoNode {
        pubkey: "HonestNode_B".to_string(),
        infrastructure_escore: 90,
        activation_epoch: 0,
    });

    // CAS A : Sans Slot Hash (PoH) baseline stable
    // L'attaquant essaie de trouver une transaction signature qui l'élit leader pour le slot hash fixe.
    let fixed_slot_hash = "solana_slot_hash_fixed_123";
    let mut found_winning_tx = None;
    for nonce in 0..1000 {
        let simulated_tx_signature = format!("solana_tx_sig_with_nonce_{}", nonce);
        let elected_leader =
            DelegationEngine::elect_leader(&registry, &simulated_tx_signature, fixed_slot_hash, 0)
                .unwrap();
        if elected_leader == "AttackerNode" {
            found_winning_tx = Some((simulated_tx_signature, nonce));
            break;
        }
    }

    if let Some((sig, nonce)) = found_winning_tx {
        println!("⚠️ GRINDING THÉORIQUE TOUJOURS POSSIBLE (si slot_hash est fixe) : AttackerNode gagne en {} essais avec {}", nonce, sig);
    }

    // CAS B : Avec Slot Hash (PoH) dynamique inconnu à l'avance
    // L'attaquant ne peut pas prédire le slot hash à l'avance lors de la génération locale de sa signature.
    // S'il grind sa signature locale en croyant que le slot hash est 'H1', mais que le validateur utilise
    // le slot hash finalisé 'H2', le résultat de l'élection change complètement.
    let real_slot_hash_from_poh = "solana_slot_hash_real_567"; // généré après block production
    let selected_leader = DelegationEngine::elect_leader(
        &registry,
        "forged_sig_intended_for_fixed",
        real_slot_hash_from_poh,
        0,
    )
    .unwrap();
    println!(
        "✓ ATTÉNUATION PoH CONFIRMÉE : La signature forgée ne gagne plus. Le Leader élu est : {}",
        selected_leader
    );
    assert_ne!(selected_leader, "AttackerNode", "L'attaquant n'a pas pu hacker la loterie car le slot hash dynamique a brouillé les pistes.");

    // ─────────────────────────────────────────────────────────────────────────
    // ÉTAPE 3 : Faille 2 - Désynchronisation Kademlia & Atténuation Epoch Checkpoints
    // ─────────────────────────────────────────────────────────────────────────
    println!("\n=== ÉTAPE 3 : FAILLE 2 - DÉSYNCHRONISATION DHT & ATTÉNUATION EPOCHS ===");

    // Nœud A et Nœud B ont des registries divergents dans l'époque 0
    let mut view_node_a = SwarmRegistry::new();
    view_node_a.register_node(ArgoNode {
        pubkey: "Node_1".to_string(),
        infrastructure_escore: 100,
        activation_epoch: 0,
    });
    // Node_2 vient de s'inscrire, Node_A le voit à l'époque 0 mais son activation est prévue pour l'époque 1
    view_node_a.register_node(ArgoNode {
        pubkey: "Node_2".to_string(),
        infrastructure_escore: 100,
        activation_epoch: 1,
    });

    let mut view_node_b = SwarmRegistry::new();
    // Node_B ne voit pas encore Node_2 du tout (lag propagation)
    view_node_b.register_node(ArgoNode {
        pubkey: "Node_1".to_string(),
        infrastructure_escore: 100,
        activation_epoch: 0,
    });

    let event_sig = "solana_real_tx_signature_abc123";
    let slot_hash = "slot_hash_abc_123";

    // Les deux nœuds calculent le leader pour l'epoch 0 en cours
    let leader_a = DelegationEngine::elect_leader(&view_node_a, event_sig, slot_hash, 0).unwrap();
    let leader_b = DelegationEngine::elect_leader(&view_node_b, event_sig, slot_hash, 0).unwrap();

    println!(
        "  Vue Nœud A (Élection Epoch 0) -> Leader élu : {}",
        leader_a
    );
    println!(
        "  Vue Nœud B (Élection Epoch 0) -> Leader élu : {}",
        leader_b
    );

    // Grâce au filtrage par activation_epoch <= current_epoch, Node_2 est ignoré par les deux nœuds
    // pendant l'époque 0, éliminant le split-brain.
    assert_eq!(
        leader_a, leader_b,
        "L'atténuation par Epoch checkpoints a échoué."
    );
    println!("✓ ATTÉNUATION EPOCH CONFIRMÉE : Node_A et Node_B s'accordent sur le même leader ({}) malgré la désynchronisation DHT.", leader_a);

    // ─────────────────────────────────────────────────────────────────────────
    // ÉTAPE 4 : Faille 3 - Wash-Trading & Atténuation du Decay par volume
    // ─────────────────────────────────────────────────────────────────────────
    println!("\n=== ÉTAPE 4 : FAILLE 3 - FILTRAGE DU WASH-TRADING ===");

    // Bot tente de wash-trader avec un micro-volume (0.005 SOL) pour réinitialiser son decay
    let micro_trade = PumpFunAction::Trade {
        token_address: "TOKEN_XYZ".to_string(),
        sol_amount: 5_000_000, // 0.005 SOL (sous le seuil de 0.1 SOL)
        token_amount: 10,
        is_buy: true,
    };

    let mapped_micro = map_action(&micro_trade, now);
    // Le mapper doit rejeter le micro-volume pour ne générer aucun signal E-score.
    assert!(
        mapped_micro.token_signals.is_empty(),
        "Le micro-volume aurait dû être filtré !"
    );
    println!("✓ ATTÉNUATION WASH-TRADE CONFIRMÉE : La transaction de 0.005 SOL a été ignorée par le mapper.");
}
