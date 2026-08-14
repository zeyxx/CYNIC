use crate::ObserverEvent;
use serde::{Deserialize, Serialize};

/// Représentation Agnostique (Domaine) d'une action Pump.fun
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PumpFunAction {
    /// Création d'un nouveau token
    TokenCreated { token_address: String, user: String },
    /// Un trade a eu lieu (Achat/Vente)
    Trade {
        token_address: String,
        sol_amount: u64,
        token_amount: u64,
        is_buy: bool,
    },
    /// Le bonding curve est complet (Migration vers Raydium)
    CurveCompleted { token_address: String },
    /// Action inconnue ou non pertinente
    Unknown,
}

/// Le Parser (Le Cortex)
/// Architecture Hexagonale : Il ne fait pas d'I/O.
#[derive(Debug)]
pub struct PumpFunParser;

impl PumpFunParser {
    /// Analyse statique et synchrone d'un événement brut.
    /// Il supporte de manière transparente les webhooks Helius enrichis et les réponses RPC getTransaction publiques.
    pub fn parse(event: &ObserverEvent) -> PumpFunAction {
        let text = match std::str::from_utf8(&event.payload) {
            Ok(t) => t.trim(),
            Err(_) => return PumpFunAction::Unknown,
        };

        // 1. Détection format Webhook Helius (JSON Array)
        if text.starts_with('[') {
            if let Some(action) = Self::parse_helius_webhook(text).into_iter().next() {
                return action;
            }
        }

        // 2. Détection format JSON RPC getTransaction standard
        if text.starts_with('{') {
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(text) {
                if json_val.get("result").is_some() {
                    return Self::parse_public_rpc(&json_val);
                }
            }
        }

        // 3. Repli : heuristique textuelle sur les logs bruts (utile pour les simulations ou les logs WSS simples)
        if text.contains("Instruction: Create") || text.contains("Program log: Create") {
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

    /// Décodeur souverain pour les réponses JSON-RPC publiques standard de `getTransaction`
    pub fn parse_public_rpc(tx_data: &serde_json::Value) -> PumpFunAction {
        let meta = match tx_data.get("result").and_then(|r| r.get("meta")) {
            Some(m) => m,
            None => return PumpFunAction::Unknown,
        };

        // Éliminer les transactions ayant échoué
        if meta.get("err").map(|e| !e.is_null()).unwrap_or(false) {
            return PumpFunAction::Unknown;
        }

        let logs = meta
            .get("logMessages")
            .and_then(|l| l.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<&str>>())
            .unwrap_or_default();

        let mut action_type = "unknown";
        for log in &logs {
            if log.contains("Instruction: Create") {
                action_type = "create";
                break;
            } else if log.contains("Instruction: Buy") {
                action_type = "buy";
                break;
            } else if log.contains("Instruction: Sell") {
                action_type = "sell";
                break;
            } else if log.contains("Instruction: Complete") {
                action_type = "complete";
                break;
            }
        }

        if action_type == "unknown" {
            return PumpFunAction::Unknown;
        }

        // Récupérer le Mint Token
        let token_address = meta
            .get("postTokenBalances")
            .and_then(|t| t.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("mint"))
            .and_then(|m| m.as_str())
            .unwrap_or("UNKNOWN_MINT")
            .to_string();

        let transaction = match tx_data.get("result").and_then(|r| r.get("transaction")) {
            Some(t) => t,
            None => return PumpFunAction::Unknown,
        };

        // Récupérer l'utilisateur (le premier signataire)
        let user = transaction
            .get("message")
            .and_then(|m| m.get("accountKeys"))
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .map(|first| {
                if let Some(s) = first.as_str() {
                    s.to_string()
                } else if let Some(pubkey) = first.get("pubkey").and_then(|p| p.as_str()) {
                    pubkey.to_string()
                } else {
                    "UNKNOWN_USER".to_string()
                }
            })
            .unwrap_or_else(|| "UNKNOWN_USER".to_string());

        match action_type {
            "create" => PumpFunAction::TokenCreated {
                token_address,
                user,
            },
            "complete" => PumpFunAction::CurveCompleted { token_address },
            "buy" | "sell" => {
                // Calcul du montant de SOL : différence sur les comptes autres que le payer
                let pre_bal = meta.get("preBalances").and_then(|b| b.as_array());
                let post_bal = meta.get("postBalances").and_then(|b| b.as_array());

                let mut sol_amount = 0u64;
                if let (Some(pre), Some(post)) = (pre_bal, post_bal) {
                    let mut max_change = 0u64;
                    for i in 1..pre.len().min(post.len()) {
                        let pre_val = pre[i].as_u64().unwrap_or(0);
                        let post_val = post[i].as_u64().unwrap_or(0);
                        let change = (post_val as i128 - pre_val as i128).abs() as u64;
                        if change > max_change {
                            max_change = change;
                        }
                    }
                    sol_amount = max_change;
                }

                // Calcul du montant de Tokens : net change pour le mint ciblé
                let mut token_amount = 0u64;
                let post_token = meta.get("postTokenBalances").and_then(|b| b.as_array());
                let pre_token = meta.get("preTokenBalances").and_then(|b| b.as_array());
                if let (Some(post), Some(pre)) = (post_token, pre_token) {
                    let mut pre_sum = 0.0f64;
                    let mut post_sum = 0.0f64;
                    for item in pre {
                        if item.get("mint").and_then(|m| m.as_str()) == Some(&token_address) {
                            pre_sum += item
                                .get("uiTokenAmount")
                                .and_then(|a| a.get("amount"))
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                        }
                    }
                    for item in post {
                        if item.get("mint").and_then(|m| m.as_str()) == Some(&token_address) {
                            post_sum += item
                                .get("uiTokenAmount")
                                .and_then(|a| a.get("amount"))
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                        }
                    }
                    token_amount = (post_sum - pre_sum).abs() as u64;
                }

                PumpFunAction::Trade {
                    token_address,
                    sol_amount,
                    token_amount,
                    is_buy: action_type == "buy",
                }
            }
            _ => PumpFunAction::Unknown,
        }
    }

    /// Décodeur pour le format Webhook transactionnel enrichi d'Helius
    pub fn parse_helius_webhook(payload: &str) -> Vec<PumpFunAction> {
        let parsed: serde_json::Value = match serde_json::from_str(payload) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        let txs = match parsed.as_array() {
            Some(arr) => arr,
            None => return vec![],
        };

        let mut actions = Vec::new();
        for tx in txs {
            let source = tx.get("source").and_then(|s| s.as_str()).unwrap_or("");
            if source != "PUMP_FUN" {
                continue;
            }

            let tx_type = tx.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let fee_payer = tx
                .get("feePayer")
                .and_then(|s| s.as_str())
                .unwrap_or("UNKNOWN_USER")
                .to_string();

            let token_address = tx
                .get("tokenTransfers")
                .and_then(|t| t.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("mint"))
                .and_then(|m| m.as_str())
                .unwrap_or("UNKNOWN_MINT")
                .to_string();

            match tx_type {
                "CREATE" => {
                    actions.push(PumpFunAction::TokenCreated {
                        token_address,
                        user: fee_payer,
                    });
                }
                "SWAP" | "BUY" | "SELL" => {
                    let mut sol_amount = 0u64;
                    if let Some(transfers) = tx.get("nativeTransfers").and_then(|t| t.as_array()) {
                        for tr in transfers {
                            sol_amount += tr.get("amount").and_then(|a| a.as_u64()).unwrap_or(0);
                        }
                    }

                    let mut token_amount = 0u64;
                    if let Some(transfers) = tx.get("tokenTransfers").and_then(|t| t.as_array()) {
                        for tr in transfers {
                            token_amount += tr
                                .get("tokenAmount")
                                .and_then(|a| a.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0) as u64;
                        }
                    }

                    actions.push(PumpFunAction::Trade {
                        token_address,
                        sol_amount,
                        token_amount,
                        is_buy: tx_type == "BUY" || tx_type == "SWAP",
                    });
                }
                _ => {}
            }
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_public_rpc_buy() {
        let tx_data = json!({
            "result": {
                "meta": {
                    "err": null,
                    "logMessages": [
                        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]",
                        "Program log: Instruction: Buy",
                        "Program log: Buy tokens",
                        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P success"
                    ],
                    "preBalances": [10000000000u64, 5000000000u64, 1000000000u64],
                    "postBalances": [9480000000u64, 5000000000u64, 1500000000u64],
                    "preTokenBalances": [
                        {
                            "accountIndex": 1,
                            "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                            "uiTokenAmount": {
                                "amount": "0",
                                "decimals": 6,
                                "uiAmount": 0.0,
                                "uiAmountString": "0"
                            }
                        }
                    ],
                    "postTokenBalances": [
                        {
                            "accountIndex": 1,
                            "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                            "uiTokenAmount": {
                                "amount": "50000000",
                                "decimals": 6,
                                "uiAmount": 50.0,
                                "uiAmountString": "50"
                            }
                        }
                    ]
                },
                "transaction": {
                    "message": {
                        "accountKeys": [
                            "UserWalletAddress1111111111111111111111111",
                            "UserTokenAccountAddress2222222222222222222",
                            "BondingCurveAddress3333333333333333333333"
                        ]
                    }
                }
            }
        });

        let action = PumpFunParser::parse_public_rpc(&tx_data);
        match action {
            PumpFunAction::Trade {
                token_address,
                sol_amount,
                token_amount,
                is_buy,
            } => {
                assert_eq!(
                    token_address,
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
                );
                assert_eq!(sol_amount, 500_000_000); // 0.5 SOL delta for bonding curve
                assert_eq!(token_amount, 50_000_000);
                assert!(is_buy);
            }
            _ => panic!("Should have parsed as a Trade"),
        }
    }

    #[test]
    fn test_parse_helius_webhook_create() {
        let payload = json!([
            {
                "description": "User created a new token on Pump.fun",
                "type": "CREATE",
                "source": "PUMP_FUN",
                "feePayer": "CreatorWalletAddress9999999999999999999999",
                "tokenTransfers": [
                    {
                        "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                        "tokenAmount": "1000000000"
                    }
                ]
            }
        ]);

        let actions = PumpFunParser::parse_helius_webhook(&payload.to_string());
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            PumpFunAction::TokenCreated {
                token_address,
                user,
            } => {
                assert_eq!(
                    token_address,
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
                );
                assert_eq!(user, "CreatorWalletAddress9999999999999999999999");
            }
            _ => panic!("Should have parsed as TokenCreated"),
        }
    }

    #[test]
    fn test_parse_transparent_delegation() {
        // Test parsing standard log message format (fallback)
        let log_event = ObserverEvent {
            signature: "SIG".to_string(),
            slot: 0,
            program_id: "6EF".to_string(),
            payload: "Program log: Instruction: Create".to_string().into_bytes(),
        };
        let action = PumpFunParser::parse(&log_event);
        assert!(matches!(action, PumpFunAction::TokenCreated { .. }));

        // Test transparent parsing of Helius JSON array
        let helius_event = ObserverEvent {
            signature: "SIG".to_string(),
            slot: 0,
            program_id: "6EF".to_string(),
            payload: json!([
                {
                    "type": "CREATE",
                    "source": "PUMP_FUN",
                    "feePayer": "USER",
                    "tokenTransfers": [{"mint": "MINT"}]
                }
            ])
            .to_string()
            .into_bytes(),
        };
        let action = PumpFunParser::parse(&helius_event);
        match action {
            PumpFunAction::TokenCreated {
                token_address,
                user,
            } => {
                assert_eq!(token_address, "MINT");
                assert_eq!(user, "USER");
            }
            _ => panic!("Transparent parsing failed"),
        }
    }
}
