use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use crate::ObserverEvent;

const PUMP_FUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfX9PNbMzXpB8vN3bL";
const PUBLIC_WSS_RPC: &str = "wss://api.mainnet-beta.solana.com/";

/// Le Watcher est l'organe sensoriel du nœud Argo.
/// Il se connecte en WebSocket à un nœud RPC Solana et filtre spécifiquement
/// les événements liés au programme Pump.fun.
pub struct Watcher {
    rpc_url: String,
}

impl Watcher {
    pub fn new(rpc_url: Option<String>) -> Self {
        Self {
            rpc_url: rpc_url.unwrap_or_else(|| PUBLIC_WSS_RPC.to_string()),
        }
    }

    /// Ouvre les yeux : démarre la connexion WebSocket et déverse les événements
    /// pertinents dans le channel de destination.
    pub async fn watch(&self, tx: mpsc::Sender<ObserverEvent>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Watcher: Tentative de connexion au RPC WSS: {}", self.rpc_url);
        
        let (mut ws_stream, _) = connect_async(&self.rpc_url).await?;
        log::info!("Watcher: Connecté avec succès. Envoi de la souscription logsSubscribe...");

        // Souscription aux logs de transactions qui impliquent le programme Pump.fun
        let subscribe_msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "logsSubscribe",
            "params": [
                {
                    "mentions": [PUMP_FUN_PROGRAM_ID]
                },
                {
                    "commitment": "confirmed"
                }
            ]
        });

        ws_stream.send(Message::Text(subscribe_msg.to_string().into())).await?;

        // Boucle d'écoute infinie (Le système nerveux)
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let text_str = text.as_str();
                    // Analyse très sommaire pour l'instant (le "cerveau" / parser fera le reste)
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text_str) {
                        if let Some(params) = parsed.get("params") {
                            if let Some(result) = params.get("result") {
                                if let Some(value) = result.get("value") {
                                    // Extraction de la signature (Identifiant unique de la transaction)
                                    let signature = value.get("signature")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("UNKNOWN")
                                        .to_string();
                                    
                                    // Dans une implémentation complète, nous devrons fetcher la transaction 
                                    // entière pour parser le payload binaire exact (les instructions CPI).
                                    // Pour l'instant, on se contente du log brut comme preuve de vie.
                                    
                                    let event = ObserverEvent {
                                        signature,
                                        slot: 0, // Slot non présent dans le payload de log de base, nécessitera un enrichissement
                                        program_id: PUMP_FUN_PROGRAM_ID.to_string(),
                                        payload: text_str.as_bytes().to_vec(),
                                    };

                                    // Transmission de l'influx nerveux
                                    if let Err(e) = tx.send(event).await {
                                        log::error!("Watcher: Canal de transmission rompu: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Message::Ping(p)) => {
                    // Maintien de la connexion (Heartbeat)
                    ws_stream.send(Message::Pong(p)).await?;
                }
                Ok(Message::Close(_)) => {
                    log::warn!("Watcher: Connexion WSS fermée par le serveur distant.");
                    break;
                }
                Err(e) => {
                    log::error!("Watcher: Erreur réseau sur le flux WebSocket: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
