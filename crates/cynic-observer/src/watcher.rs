use crate::ObserverEvent;
use futures_util::SinkExt;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const PUMP_FUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
const PUBLIC_WSS_RPC: &str = "wss://api.mainnet-beta.solana.com/";

/// Compteur global des déconnexions/reconnexions WebSocket observées.
pub static WSS_RECONNECTIONS: AtomicU64 = AtomicU64::new(0);

/// Le Watcher est l'organe sensoriel du nœud Argo.
/// Il se connecte en WebSocket à un nœud RPC Solana et filtre spécifiquement
/// les événements liés au programme Pump.fun.
#[derive(Debug)]
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
    /// pertinents dans le channel de destination. Intègre une boucle de reconnexion
    /// automatique avec backoff exponentiel pour résister aux coupures du RPC public.
    pub async fn watch(
        &self,
        tx: mpsc::Sender<ObserverEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut retry_delay = std::time::Duration::from_secs(1);

        loop {
            log::info!(
                "Watcher: Tentative de connexion au RPC WSS: {}",
                self.rpc_url
            );

            match connect_async(&self.rpc_url).await {
                Ok((mut ws_stream, _)) => {
                    log::info!(
                        "Watcher: Connecté avec succès. Envoi de la souscription logsSubscribe..."
                    );
                    retry_delay = std::time::Duration::from_secs(1); // Réinitialisation du backoff

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

                    if let Err(e) = ws_stream
                        .send(Message::Text(subscribe_msg.to_string().into()))
                        .await
                    {
                        log::error!(
                            "Watcher: Échec de l'envoi du message de souscription : {}",
                            e
                        );
                        WSS_RECONNECTIONS.fetch_add(1, Ordering::Relaxed);
                        tokio::time::sleep(retry_delay).await;
                        retry_delay =
                            std::cmp::min(retry_delay * 2, std::time::Duration::from_secs(30));
                        continue;
                    }

                    // Boucle d'écoute (Le système nerveux)
                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                let text_str = text.as_str();
                                log::info!(
                                    "Watcher: Raw WSS message: {}",
                                    if text_str.len() > 150 {
                                        &text_str[..150]
                                    } else {
                                        text_str
                                    }
                                );
                                if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(text_str)
                                {
                                    if let Some(params) = parsed.get("params") {
                                        if let Some(result) = params.get("result") {
                                            if let Some(value) = result.get("value") {
                                                // Extraction de la signature (Identifiant unique de la transaction)
                                                let signature = value
                                                    .get("signature")
                                                    .and_then(|s| s.as_str())
                                                    .unwrap_or("UNKNOWN")
                                                    .to_string();

                                                let event = ObserverEvent {
                                                    signature,
                                                    slot: 0,
                                                    program_id: PUMP_FUN_PROGRAM_ID.to_string(),
                                                    payload: text_str.as_bytes().to_vec(),
                                                };

                                                // Transmission de l'influx nerveux
                                                if let Err(e) = tx.send(event).await {
                                                    log::error!(
                                                        "Watcher: Canal de transmission rompu : {}",
                                                        e
                                                    );
                                                    return Ok(());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(Message::Ping(p)) => {
                                if let Err(e) = ws_stream.send(Message::Pong(p)).await {
                                    log::error!("Watcher: Échec de l'envoi du Pong : {}", e);
                                    break;
                                }
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
                }
                Err(e) => {
                    log::error!("Watcher: Échec de connexion au WebSocket : {}", e);
                }
            }

            WSS_RECONNECTIONS.fetch_add(1, Ordering::Relaxed);
            log::warn!(
                "Watcher: Déconnexion détectée. Tentative de reconnexion dans {:?}",
                retry_delay
            );
            tokio::time::sleep(retry_delay).await;
            retry_delay = std::cmp::min(retry_delay * 2, std::time::Duration::from_secs(30));
        }
    }
}

/// Récupère les détails enrichis d'une transaction via la méthode standard `getTransaction`
pub async fn fetch_transaction_details(
    rpc_url: &str,
    signature: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            signature,
            {
                "encoding": "jsonParsed",
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let resp = client.post(rpc_url).json(&payload).send().await?;
    let json_resp: serde_json::Value = resp.json().await?;
    Ok(json_resp)
}
