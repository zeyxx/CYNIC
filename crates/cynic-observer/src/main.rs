<<<<<<< HEAD
//! CYNIC Observer Daemon Entrypoint

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    cynic_observer::runner::run_argo_node().await
=======
use cynic_protocol::Epoch;
use cynic_observer::parser::{PumpFunAction, PumpFunParser};
use cynic_observer::watcher::Watcher;
use log::{info, warn};
use tokio::sync::mpsc;

fn load_helius_key() -> Option<String> {
    if let Ok(key) = std::env::var("CYNIC_HELIUS_API_KEY") {
        return Some(key.trim().to_string());
    }
    let home = std::env::var("HOME").ok()?;
    let path = std::path::PathBuf::from(home).join(".config/cynic/helius-api-key");
    if path.exists() {
        if let Ok(key) = std::fs::read_to_string(path) {
            return Some(key.trim().to_string());
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialisation du logger avec un niveau par défaut INFO
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Initialisation du CryptoProvider pour Rustls (nécessaire pour WSS)
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    info!("=== CYNIC Observer (Argo Node) ===");

    // Configuration des URLs RPC via l'environnement ou fallbacks publics
    let rpc_url = std::env::var("CYNIC_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let wss_url = std::env::var("CYNIC_WSS_URL")
        .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com/".to_string());

    // Détection de la clé Helius pour le comparateur baseline
    let mut helius_rpc_url = None;
    let mut target_wss_url = wss_url.clone();
    if let Some(key) = load_helius_key() {
        info!("🔑 Clé Helius détectée (présence: true). Ingestion comparative activée.");
        helius_rpc_url = Some(format!("https://mainnet.helius-rpc.com/?api-key={}", key));
        if std::env::var("CYNIC_WSS_URL").is_err() {
            target_wss_url = format!("wss://mainnet.helius-rpc.com/?api-key={}", key);
        }
    }

    // 1. Initialisation du Métabolisme Matériel
    use cynic_host::ports::metabolism::{MetabolismProvider, OsMetabolism};
    let mut metabolism = OsMetabolism::new();
    let current_state = metabolism.current_state();
    info!(
        "État métabolique du nœud au démarrage : {:?}",
        current_state
    );

    // 2. Initialisation du Write-Ahead Log (WAL)
    use cynic_observer::wal::{WalRecord, WriteAheadLog};
    let wal = std::sync::Arc::new(std::sync::Mutex::new(
        WriteAheadLog::new("argo_node.wal").expect("Impossible de créer le WAL"),
    ));
    let recovered_events = WriteAheadLog::replay("argo_node.wal").unwrap_or_default();
    info!(
        "WAL: {} événements restaurés en mémoire depuis le disque.",
        recovered_events.len()
    );

    // 3. Initialisation et démarrage de la Fédération P2P (libp2p)
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    let connected_peers = Arc::new(AtomicU64::new(0));

    // Initialisation du canal de diffusion pour le Dashboard
    let (broadcast_tx, _) = tokio::sync::broadcast::channel::<serde_json::Value>(100);

    // Lancement du serveur Dashboard
    let broadcast_tx_server = broadcast_tx.clone();
    let connected_peers_server = connected_peers.clone();
    let dashboard_addr =
        std::env::var("CYNIC_DASHBOARD_ADDR").unwrap_or_else(|_| "127.0.0.1:3031".to_string());
    tokio::spawn(async move {
        if let Err(e) = cynic_observer::dashboard::run_server(
            &dashboard_addr,
            connected_peers_server,
            broadcast_tx_server,
        )
        .await
        {
            log::error!("Erreur critique du serveur Dashboard : {}", e);
        }
    });

    let mut fed_node = cynic_observer::federation::FederationNode::new()?;
    fed_node.start().await?;
    let fed_peer_count_clone = connected_peers.clone();
    tokio::spawn(async move {
        if let Err(e) = fed_node.run(fed_peer_count_clone).await {
            log::error!("Erreur dans la boucle du nœud P2P Fédération : {}", e);
        }
    });

    // Variable partagée pour l'epoch courant (accessible par la boucle principale et la télémétrie)
    use std::sync::Mutex as StdMutex;
    let current_epoch_shared = Arc::new(StdMutex::new(Epoch::ZERO));

    // 4. Démarrage de la boucle de Télémétrie et Preuves du Consensus en tâche de fond
    let node_pubkey = std::env::var("CYNIC_NODE_PUBKEY")
        .unwrap_or_else(|_| "argo-sovereign-node-dev".to_string());
    let rpc_url_clone = rpc_url.clone();
    let connected_peers_telemetry = connected_peers.clone();
    let wal_telemetry = wal.clone();
    let metabolism_telemetry = metabolism.clone();
    let epoch_shared_clone = current_epoch_shared.clone();

    tokio::spawn(async move {
        let mut current_epoch = Epoch::ZERO;
        let mut last_epoch_mark = std::time::Instant::now();
        loop {
            let peers_count = connected_peers_telemetry.load(Ordering::Relaxed);

            // Incrémenter l'epoch périodiquement (toutes les 60 secondes)
            if last_epoch_mark.elapsed().as_secs() >= 60 {
                current_epoch = current_epoch.next();
                last_epoch_mark = std::time::Instant::now();

                // Partager le nouvel epoch avec la boucle principale
                {
                    let mut epoch_guard = epoch_shared_clone.lock().unwrap();
                    *epoch_guard = current_epoch;
                }

                // Marquer l'epoch dans le WAL
                let current_metabolism = metabolism_telemetry.current_state();
                let registry_digest = format!(
                    "sha256:epoch-{}-peers-{}",
                    current_epoch.id(),
                    peers_count
                );
                if let Err(e) = wal_telemetry.lock().unwrap().mark_epoch(
                    current_epoch,
                    registry_digest,
                    &current_metabolism,
                ) {
                    log::warn!("Échec de l'écriture EpochMarker dans le WAL : {}", e);
                }
                log::info!("🔄 Epoch {} marqué dans le WAL", current_epoch.id());
            }

            info!("ConsensusTelemetry: Enregistrement et mesure de l'état du consensus...");
            let proof = cynic_observer::consensus::ConsensusTelemetry::generate_proof(
                &node_pubkey,
                peers_count as usize,
                &rpc_url_clone,
                current_epoch,
            )
            .await;

            if let Ok(proof_json) = serde_json::to_string_pretty(&proof) {
                let proof_path = std::env::var("CYNIC_ARGO_PROOF_PATH")
                    .unwrap_or_else(|_| "argo-proof.json".to_string());

                if let Err(e) = std::fs::write(&proof_path, &proof_json) {
                    log::error!(
                        "Impossible de sauvegarder la preuve argo-proof sur le disque : {}",
                        e
                    );
                } else {
                    info!(
                        "✓ Preuve d'observation écrite avec succès dans {}",
                        proof_path
                    );
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

    info!(
        "Démarrage du sous-système sensoriel (Watcher WSS : {})...",
        wss_url
    );

    // Création du canal nerveux (Nervous System) entre le Watcher (I/O) et le Cortex (Parser)
    let (tx, mut rx) = mpsc::channel(100);

    // Poller actif de signatures en parallèle du Watcher WSS
    // pour garantir la réception d'événements réels même si le WSS public est bridé
    let tx_poll = tx.clone();
    let rpc_url_poll = helius_rpc_url.clone().unwrap_or_else(|| rpc_url.clone());
    info!(
        "📡 Poller de signatures configuré sur l'URL : {}",
        if helius_rpc_url.is_some() {
            "Helius RPC"
        } else {
            "RPC Public"
        }
    );
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let mut seen_signatures = std::collections::HashSet::new();

        loop {
            let payload = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getSignaturesForAddress",
                "params": [
                    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
                    {
                        "limit": 10
                    }
                ]
            });

            match client.post(&rpc_url_poll).json(&payload).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        log::warn!("Poller de signatures: Status d'erreur RPC: {}", status);
                    } else if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(err) = json.get("error") {
                            log::warn!(
                                "Poller de signatures: Erreur renvoyée par le RPC: {:?}",
                                err
                            );
                        } else if let Some(result) = json.get("result").and_then(|r| r.as_array()) {
                            let mut new_sigs_count = 0;
                            // Parcourir les signatures de la plus ancienne à la plus récente
                            for tx_info in result.iter().rev() {
                                if let Some(sig) = tx_info.get("signature").and_then(|s| s.as_str())
                                {
                                    if seen_signatures.contains(sig) {
                                        continue;
                                    }

                                    if seen_signatures.len() > 500 {
                                        seen_signatures.clear();
                                    }
                                    seen_signatures.insert(sig.to_string());
                                    new_sigs_count += 1;

                                    let event = cynic_observer::ObserverEvent {
                                        signature: sig.to_string(),
                                        slot: tx_info
                                            .get("slot")
                                            .and_then(|s| s.as_u64())
                                            .unwrap_or(0),
                                        program_id: "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
                                            .to_string(),
                                        payload: vec![],
                                    };

                                    if let Err(_) = tx_poll.send(event).await {
                                        break;
                                    }
                                }
                            }
                            if new_sigs_count > 0 {
                                log::info!("📡 Poller de signatures: {} nouvelles signatures captées sur le réseau.", new_sigs_count);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Poller de signatures: Échec de l'appel RPC: {}", e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    // Initialisation du Watcher avec l'URL configurée (éventuellement optimisée Helius)
    let watcher = Watcher::new(Some(target_wss_url));

    // Spawn du Watcher dans une tâche asynchrone pour ne pas bloquer le thread principal
    let tx_watch = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = watcher.watch(tx_watch).await {
            log::error!("Erreur fatale du Watcher: {}", e);
        }
    });

    // Variable partagée pour l'epoch courant (déjà initialisée ci-dessus)
    info!("Le Cortex est en attente d'événements...");

    // Boucle principale du Cortex : réception et parsing
    while let Some(raw_event) = rx.recv().await {
        // Sauvegarde WAL (Adaptative)
        let current_metabolism = metabolism.current_state();
        let record = WalRecord::EventReceived(raw_event.clone());
        if let Err(e) = wal.lock().unwrap().append(&record, &current_metabolism) {
            log::error!("Échec de l'écriture WAL : {}", e);
        }

        let is_helius = std::str::from_utf8(&raw_event.payload)
            .map(|t| t.trim().starts_with('['))
            .unwrap_or(false);

        if is_helius {
            cynic_observer::consensus::COMPARATOR.record_helius_event(1); // Webhook instantané
        }

        // Enrichissement adaptatif : Si l'événement est un log brut WSS, on interroge le RPC public pour avoir la transaction complète
        let mut resolved_event = raw_event.clone();

        if raw_event.signature != "UNKNOWN" && raw_event.signature != "SIMULATED_SIG" {
            info!(
                "Enrichissement de la transaction {} via RPC public...",
                raw_event.signature
            );

            // Requête 1 : RPC Public
            let start_pub = std::time::Instant::now();
            let pub_res =
                cynic_observer::watcher::fetch_transaction_details(&rpc_url, &raw_event.signature)
                    .await;

            // Requête 2 : Helius (si disponible)
            let mut helius_res = None;
            if let Some(ref h_url) = helius_rpc_url {
                let start_hel = std::time::Instant::now();
                if let Ok(json) =
                    cynic_observer::watcher::fetch_transaction_details(h_url, &raw_event.signature)
                        .await
                {
                    let helius_latency = start_hel.elapsed().as_millis() as u64;
                    helius_res = Some(json);
                    cynic_observer::consensus::COMPARATOR.record_helius_event(helius_latency);
                }
            }

            match pub_res {
                Ok(enriched_json) => {
                    let public_rpc_latency = start_pub.elapsed().as_millis() as u64;
                    resolved_event.payload = enriched_json.to_string().into_bytes();

                    let is_matched = if let Some(h_json) = helius_res {
                        // Comparaison de concordance
                        let parsed_pub = PumpFunParser::parse_public_rpc(&enriched_json);
                        let parsed_hel = PumpFunParser::parse_public_rpc(&h_json);
                        match (&parsed_pub, &parsed_hel) {
                            (
                                PumpFunAction::Trade {
                                    token_address: t1,
                                    sol_amount: s1,
                                    ..
                                },
                                PumpFunAction::Trade {
                                    token_address: t2,
                                    sol_amount: s2,
                                    ..
                                },
                            ) => t1 == t2 && s1 == s2,
                            (
                                PumpFunAction::TokenCreated {
                                    token_address: t1, ..
                                },
                                PumpFunAction::TokenCreated {
                                    token_address: t2, ..
                                },
                            ) => t1 == t2,
                            (
                                PumpFunAction::CurveCompleted { token_address: t1 },
                                PumpFunAction::CurveCompleted { token_address: t2 },
                            ) => t1 == t2,
                            (PumpFunAction::Unknown, PumpFunAction::Unknown) => true,
                            _ => false,
                        }
                    } else {
                        true
                    };

                    let is_action_valid = !matches!(
                        PumpFunParser::parse(&resolved_event),
                        PumpFunAction::Unknown
                    );
                    cynic_observer::consensus::COMPARATOR
                        .record_public_rpc_event(public_rpc_latency, is_matched && is_action_valid);

                    if helius_rpc_url.is_none() {
                        cynic_observer::consensus::COMPARATOR.record_helius_event(5);
                        // Baseline fictive (5ms)
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Impossible d'enrichir la transaction {} via RPC public : {}",
                        raw_event.signature,
                        e
                    );
                }
            }
        }

        // Le Watcher a capté un événement. On le passe au Parser.
        let action = PumpFunParser::parse(&resolved_event);
        let current_epoch = *current_epoch_shared.lock().unwrap();

        // Envoyer la transaction au diffuseur du Dashboard
        let now_unix_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let tx_payload = match &action {
            PumpFunAction::TokenCreated {
                token_address,
                user,
            } => Some(serde_json::json!({
                "type": "transaction",
                "signature": resolved_event.signature.clone(),
                "slot": resolved_event.slot,
                "action": "TokenCreated",
                "details": {
                    "token_address": token_address,
                    "user": user
                },
                "timestamp": now_unix_sec
            })),
            PumpFunAction::Trade {
                token_address,
                sol_amount,
                token_amount,
                is_buy,
            } => Some(serde_json::json!({
                "type": "transaction",
                "signature": resolved_event.signature.clone(),
                "slot": resolved_event.slot,
                "action": "Trade",
                "details": {
                    "token_address": token_address,
                    "sol_amount": sol_amount,
                    "token_amount": token_amount,
                    "is_buy": is_buy
                },
                "timestamp": now_unix_sec
            })),
            PumpFunAction::CurveCompleted { token_address } => Some(serde_json::json!({
                "type": "transaction",
                "signature": resolved_event.signature.clone(),
                "slot": resolved_event.slot,
                "action": "CurveCompleted",
                "details": {
                    "token_address": token_address
                },
                "timestamp": now_unix_sec
            })),
            _ => None,
        };

        if let Some(payload) = tx_payload {
            let _ = broadcast_tx.send(payload);
        }

        // Maper l'action en signaux E-Score et mettre à jour les métriques globales
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs() as i64;
        let mapped = cynic_observer::signal_mapper::map_action(
            &action,
            now_unix,
            current_epoch,
        );

        let wallet_signals_count = mapped.wallet_signals.len() as u64;
        let token_signals_count = mapped.token_signals.len() as u64;
        let total_signals = wallet_signals_count + token_signals_count;

        if total_signals > 0 {
            use cynic_intel::escore::metrics::ESCORE_METRICS;
            use std::sync::atomic::Ordering;
            ESCORE_METRICS
                .signals_processed
                .fetch_add(total_signals, Ordering::Relaxed);

            let mut total_weight = 0u64;
            for sig in mapped
                .wallet_signals
                .iter()
                .chain(mapped.token_signals.iter())
            {
                total_weight += sig.raw_weight as u64;
            }
            ESCORE_METRICS
                .total_score_units_accumulated
                .fetch_add(total_weight, Ordering::Relaxed);
            info!(
                "E-Score: {} signaux E-Score générés et transmis au Kernel (Poids: {} ScoreUnits)",
                total_signals, total_weight
            );
        }

        match action {
            PumpFunAction::TokenCreated {
                token_address,
                user,
            } => {
                info!("🟢 NOUVEAU TOKEN: {} créé par {}", token_address, user);
            }
            PumpFunAction::Trade {
                token_address,
                sol_amount,
                token_amount,
                is_buy,
            } => {
                let trade_type = if is_buy { "ACHAT" } else { "VENTE" };
                info!(
                    "💱 TRADE ({}): {} SOL ({} lamports) -> {} TOKENS sur {}",
                    trade_type,
                    (sol_amount as f64 / 1_000_000_000.0),
                    sol_amount,
                    token_amount,
                    token_address
                );
            }
            PumpFunAction::CurveCompleted { token_address } => {
                info!(
                    "🚀 CURVE COMPLETED: Migration imminente pour {}",
                    token_address
                );
            }
            PumpFunAction::Unknown => {
                warn!(
                    "⚪ Événement brut ignoré ou filtré (Signature: {})",
                    raw_event.signature
                );
            }
        }
    }

    Ok(())
>>>>>>> github/main
}
