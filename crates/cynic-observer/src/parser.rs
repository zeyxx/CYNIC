use serde::{Deserialize, Serialize};
use crate::ObserverEvent;

/// Représentation Agnostique (Domaine) d'une action Pump.fun
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PumpFunAction {
    /// Création d'un nouveau token
    TokenCreated { token_address: String, user: String },
    /// Un trade a eu lieu (Achat/Vente)
    Trade { token_address: String, sol_amount: u64, token_amount: u64, is_buy: bool },
    /// Le bonding curve est complet (Migration vers Raydium)
    CurveCompleted { token_address: String },
    /// Action inconnue ou non pertinente
    Unknown,
}

/// Le Parser (Le Cortex)
/// Architecture Hexagonale : Il ne fait pas d'I/O. 
/// Il prend un modèle d'entrée brut (ObserverEvent) et ressort un modèle métier (PumpFunAction).
pub struct PumpFunParser;

impl PumpFunParser {
    /// Analyse statique et synchrone d'un événement brut.
    pub fn parse(event: &ObserverEvent) -> PumpFunAction {
        // En Rust, l'event payload est un Vec<u8> contenant potentiellement le JSON du RPC
        let text = match std::str::from_utf8(&event.payload) {
            Ok(t) => t,
            Err(_) => return PumpFunAction::Unknown,
        };

        // Très basique: on utilise des heuristiques textuelles sur les logs (Program log)
        // Pump.fun émet généralement des logs spécifiques comme "Program log: Instruction: Create"
        // Une vraie implémentation parserait le layout binaire de l'instruction, 
        // mais le RPC `logsSubscribe` nous donne des strings.

        if text.contains("Instruction: Create") || text.contains("Program log: Create") {
            // Extraction simulée. Dans le monde réel on parserait le json pour trouver l'adresse.
            return PumpFunAction::TokenCreated {
                token_address: "SIMULATED_TOKEN_ADDR".to_string(),
                user: "SIMULATED_USER".to_string(),
            };
        } else if text.contains("Instruction: Buy") || text.contains("Instruction: Sell") {
            let is_buy = text.contains("Instruction: Buy");
            return PumpFunAction::Trade {
                token_address: "SIMULATED_TOKEN_ADDR".to_string(),
                sol_amount: 0,
                token_amount: 0,
                is_buy,
            };
        } else if text.contains("Instruction: Complete") || text.contains("Migration") {
            return PumpFunAction::CurveCompleted {
                token_address: "SIMULATED_TOKEN_ADDR".to_string(),
            };
        }

        PumpFunAction::Unknown
    }
}
