use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::protocol::Message;

pub async fn run_server(
    bind_addr: &str,
    connected_peers: Arc<std::sync::atomic::AtomicU64>,
    tx_broadcast: broadcast::Sender<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(bind_addr).await?;
    log::info!(
        "🖥️ Server Dashboard HTTP + WS listening on http://{}",
        bind_addr
    );

<<<<<<< HEAD
    // Démarrage du thread de diffusion périodique de télémétrie multi-dimensionnelle
=======
    // Démarrage du thread de diffusion périodique de télémétrie
>>>>>>> github/main
    let tx_telemetry = tx_broadcast.clone();
    let peers_clone = connected_peers.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

<<<<<<< HEAD
            // 1. Dimension Flotte & Mesh P2P
            let peers = peers_clone.load(std::sync::atomic::Ordering::Relaxed);

            // 2. Dimension Blockchain & RPC
=======
            // Collecter les métriques réelles du nœud
            let peers = peers_clone.load(std::sync::atomic::Ordering::Relaxed);
>>>>>>> github/main
            let success_rate = crate::consensus::ConsensusTelemetry::observed_success_rate();
            let total_probes = crate::consensus::RPC_TRACKER
                .total_probes
                .load(std::sync::atomic::Ordering::Relaxed);
            let failed_probes = crate::consensus::RPC_TRACKER
                .failed_probes
                .load(std::sync::atomic::Ordering::Relaxed);

            let helius_tx_count = crate::consensus::COMPARATOR
                .helius_tx_count
                .load(std::sync::atomic::Ordering::Relaxed);
            let public_rpc_tx_count = crate::consensus::COMPARATOR
                .public_rpc_tx_count
                .load(std::sync::atomic::Ordering::Relaxed);
            let helius_latency_sum = crate::consensus::COMPARATOR
                .helius_latency_sum_ms
                .load(std::sync::atomic::Ordering::Relaxed);
            let public_rpc_latency_sum = crate::consensus::COMPARATOR
                .public_rpc_latency_sum_ms
                .load(std::sync::atomic::Ordering::Relaxed);
            let matched_tx_count = crate::consensus::COMPARATOR
                .matched_tx_count
                .load(std::sync::atomic::Ordering::Relaxed);

            let avg_helius_latency_ms = if helius_tx_count > 0 {
                helius_latency_sum / helius_tx_count
            } else {
                0
            };
            let avg_public_latency_ms = if public_rpc_tx_count > 0 {
                public_rpc_latency_sum / public_rpc_tx_count
            } else {
                0
            };

            let completeness_rate = if helius_tx_count > 0 {
                (public_rpc_tx_count as f64 / helius_tx_count as f64) * 100.0
            } else if public_rpc_tx_count > 0 {
                100.0
            } else {
                0.0
            };

            let accuracy_match_rate = if helius_tx_count > 0 {
                (matched_tx_count as f64 / helius_tx_count as f64) * 100.0
            } else if public_rpc_tx_count > 0 {
                100.0
            } else {
                0.0
            };

<<<<<<< HEAD
            // 3. Dimension E-Score & Intel
=======
>>>>>>> github/main
            use cynic_intel::escore::metrics::ESCORE_METRICS;
            let signals_processed = ESCORE_METRICS
                .signals_processed
                .load(std::sync::atomic::Ordering::Relaxed);
            let total_score_units = ESCORE_METRICS
                .total_score_units_accumulated
                .load(std::sync::atomic::Ordering::Relaxed);
            let anti_whale_triggers = ESCORE_METRICS
                .anti_whale_dilutions_total
                .load(std::sync::atomic::Ordering::Relaxed);
            let wss_reconnections =
                crate::watcher::WSS_RECONNECTIONS.load(std::sync::atomic::Ordering::Relaxed);

<<<<<<< HEAD
            // 4. Dimension Métabolisme Système
            use crate::metabolism::{MetabolismProvider, OsMetabolism};
            let mut metabolism = OsMetabolism::new();
            let metabolic_state = format!("{:?}", metabolism.current_state());

=======
>>>>>>> github/main
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let payload = serde_json::json!({
                "type": "telemetry",
<<<<<<< HEAD
                "dimensions": {
                    "fleet": {
                        "connected_peers": peers,
                        "p2p_status": if peers > 0 { "federated" } else { "discovering" }
                    },
                    "blockchain": {
                        "rpc_success_rate": success_rate,
                        "rpc_total_probes": total_probes,
                        "rpc_failed_probes": failed_probes,
                        "helius_tx_count": helius_tx_count,
                        "public_rpc_tx_count": public_rpc_tx_count,
                        "avg_helius_latency_ms": avg_helius_latency_ms,
                        "avg_public_latency_ms": avg_public_latency_ms,
                        "completeness_rate": completeness_rate,
                        "accuracy_match_rate": accuracy_match_rate,
                        "wss_reconnections": wss_reconnections
                    },
                    "system": {
                        "metabolic_state": metabolic_state
                    },
                    "escore": {
                        "signals_processed": signals_processed,
                        "total_score_units": total_score_units,
                        "anti_whale_triggers": anti_whale_triggers
                    }
                },
=======
>>>>>>> github/main
                "connected_peers": peers,
                "rpc_success_rate": success_rate,
                "rpc_total_probes": total_probes,
                "rpc_failed_probes": failed_probes,
                "signals_processed": signals_processed,
                "total_score_units": total_score_units,
                "anti_whale_triggers": anti_whale_triggers,
                "helius_tx_count": helius_tx_count,
                "public_rpc_tx_count": public_rpc_tx_count,
                "avg_helius_latency_ms": avg_helius_latency_ms,
                "avg_public_latency_ms": avg_public_latency_ms,
                "completeness_rate": completeness_rate,
                "accuracy_match_rate": accuracy_match_rate,
                "wss_reconnections": wss_reconnections,
                "timestamp": now
            });

            let _ = tx_telemetry.send(payload);
        }
    });

<<<<<<< HEAD
    let tx_for_accept = tx_broadcast.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let rx = tx_for_accept.subscribe();
            tokio::spawn(async move {
                handle_connection(stream, rx).await;
=======
    let rx_broadcast = tx_broadcast.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let rx = rx_broadcast.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, rx).await {
                    log::debug!("Fin de connexion du Dashboard : {}", e);
                }
>>>>>>> github/main
            });
        }
    });

    Ok(())
}

async fn handle_connection(
<<<<<<< HEAD
    stream: TcpStream,
    mut rx_broadcast: broadcast::Receiver<serde_json::Value>,
) {
    let mut peek_buf = [0u8; 4];
    if let Ok(n) = stream.peek(&mut peek_buf).await {
        if n >= 3 && &peek_buf[0..3] == b"GET" {
            // Lecture complète de la requête HTTP
            let mut buffer = [0; 2048];
            let mut stream = stream;
            use tokio::io::AsyncReadExt;
            if let Ok(read_bytes) = stream.read(&mut buffer).await {
                let req = String::from_utf8_lossy(&buffer[..read_bytes]);

                // Si la requête contient "Upgrade: websocket", c'est une connexion WS
                if req.contains("Upgrade: websocket") || req.contains("upgrade: websocket") {
                    if let Ok(mut ws_stream) = tokio_tungstenite::accept_async(stream).await {
                        while let Ok(msg) = rx_broadcast.recv().await {
                            if let Ok(json_str) = serde_json::to_string(&msg) {
                                if ws_stream
                                    .send(Message::Text(json_str.into()))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    // Sinon, renvoyer la page HTML du Dashboard
                    use tokio::io::AsyncWriteExt;
                    let html_content = include_str!("dashboard.html");
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html_content.len(),
                        html_content
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.flush().await;
                }
            }
        }
    }
=======
    mut stream: TcpStream,
    rx_broadcast: broadcast::Sender<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;

    let mut buf = [0; 2048];
    // Peek dans le socket sans consommer pour identifier la nature HTTP ou WS
    let n = stream.peek(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let request_str = std::str::from_utf8(&buf[..n]).unwrap_or("");

    if request_str.contains("Upgrade: websocket") || request_str.contains("upgrade: websocket") {
        // Upgrade de la connexion en WebSocket
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        let (mut ws_sender, mut _ws_receiver) = ws_stream.split();

        // S'abonner aux événements du flux
        let mut rx = rx_broadcast.subscribe();

        // Loop de transmission des événements au format WS
        while let Ok(msg) = rx.recv().await {
            let text = msg.to_string();
            if let Err(_) = ws_sender.send(Message::Text(text.into())).await {
                break;
            }
        }
    } else if request_str.contains("GET / ") || request_str.contains("GET /index.html ") {
        // Servir le fichier HTML du Dashboard embarqué
        let html_content = include_str!("dashboard.html");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            html_content.len(),
            html_content
        );
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;
    } else {
        // Servir HTTP 404
        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;
    }

    Ok(())
>>>>>>> github/main
}
