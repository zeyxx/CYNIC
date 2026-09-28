//! Argo Node execution runner and event loop

use log::{error, info, warn};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

use crate::consensus::COMPARATOR;
use crate::dashboard::run_server;
use crate::domain::UniversalObserverEvent;
use crate::federation::FederationNode;
use crate::metabolism::{LoadProbe, OsLoadProbe};
use crate::parser::{PumpFunAction, PumpFunParser};
use crate::signal_mapper::map_action;
use crate::wal::{WalRecord, WriteAheadLog};
use crate::watcher::{Watcher, fetch_transaction_details};
use crate::{BoundedIngestionBuffer, ObserverEvent, OverflowPolicy};

pub fn load_helius_key() -> Option<String> {
    if let Some(key) = cynic_secrets::env_var("helius-api-key-obs", "CYNIC_HELIUS_API_KEY") {
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

pub async fn run_argo_node() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    info!("=== CYNIC Observer (Argo Node) ===");

    let rpc_url = std::env::var("CYNIC_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let wss_url = std::env::var("CYNIC_WSS_URL")
        .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com/".to_string());

    let mut helius_rpc_url = None;
    let mut target_wss_url = wss_url.clone();
    if let Some(key) = load_helius_key() {
        info!("Helius API key detected (presence: true). Comparative ingestion active.");
        helius_rpc_url = Some(format!("https://mainnet.helius-rpc.com/?api-key={}", key));
        if std::env::var("CYNIC_WSS_URL").is_err() {
            target_wss_url = format!("wss://mainnet.helius-rpc.com/?api-key={}", key);
        }
    }

    let mut metabolism = OsLoadProbe::new();
    let current_load = metabolism.current_load();
    info!("Host load tier at startup: {:?}", current_load);

    let mut wal = WriteAheadLog::new("argo_node.wal").map_err(|e| {
        error!("Unable to initialize WAL: {}", e);
        e
    })?;
    let recovered_events = WriteAheadLog::replay("argo_node.wal").unwrap_or_default();
    info!(
        "WAL: {} events restored to memory from disk.",
        recovered_events.len()
    );

    let ingestion_capacity: usize = std::env::var("CYNIC_INGESTION_BUFFER_CAPACITY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4096);
    let ingestion_policy = if std::env::var("CYNIC_INGESTION_OVERFLOW_REJECT").is_ok() {
        OverflowPolicy::RejectNew
    } else {
        OverflowPolicy::DropOldest
    };
    let ingestion_buffer: BoundedIngestionBuffer<ObserverEvent> =
        BoundedIngestionBuffer::new(ingestion_capacity, ingestion_policy);
    info!(
        "Ingestion buffer initialized: capacity={} policy={:?}",
        ingestion_capacity, ingestion_policy
    );

    let connected_peers = Arc::new(AtomicU64::new(0));
    let (broadcast_tx, _) = tokio::sync::broadcast::channel::<serde_json::Value>(100);

    let broadcast_tx_server = broadcast_tx.clone();
    let connected_peers_server = connected_peers.clone();
    let dashboard_addr =
        std::env::var("CYNIC_DASHBOARD_ADDR").unwrap_or_else(|_| "127.0.0.1:3031".to_string());
    tokio::spawn(async move {
        if let Err(e) =
            run_server(&dashboard_addr, connected_peers_server, broadcast_tx_server).await
        {
            error!("Dashboard server error: {}", e);
        }
    });

    let mut fed_node = FederationNode::new()?;
    fed_node.start().await?;
    let fed_peer_count_clone = connected_peers.clone();
    let (tx_incoming, _) = tokio::sync::broadcast::channel::<UniversalObserverEvent>(100);
    let (_tx_outgoing, rx_outgoing) = tokio::sync::mpsc::channel::<UniversalObserverEvent>(100);
    tokio::spawn(async move {
        if let Err(e) = fed_node
            .run(fed_peer_count_clone, tx_incoming, rx_outgoing)
            .await
        {
            error!("P2P Federation loop error: {}", e);
        }
    });

    let node_pubkey = cynic_secrets::env_var("node-pubkey", "CYNIC_NODE_PUBKEY")
        .unwrap_or_else(|| "argo-sovereign-node-dev".to_string());
    let rpc_url_clone = rpc_url.clone();
    let connected_peers_telemetry = connected_peers.clone();

    tokio::spawn(async move {
        loop {
            let peers_count = connected_peers_telemetry.load(Ordering::Relaxed);
            info!("ConsensusTelemetry: Recording consensus proof...");
            let proof = crate::consensus::ConsensusTelemetry::generate_proof(
                &node_pubkey,
                peers_count as usize,
                &rpc_url_clone,
            )
            .await;

            if let Ok(proof_json) = serde_json::to_string_pretty(&proof) {
                let proof_path = std::env::var("CYNIC_ARGO_PROOF_PATH")
                    .unwrap_or_else(|_| "argo-proof.json".to_string());

                if let Err(e) = std::fs::write(&proof_path, &proof_json) {
                    error!("Unable to save proof to disk: {}", e);
                } else {
                    info!("Observation proof written to {}", proof_path);
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

    info!("Starting sensory subsystem (Watcher WSS: {})...", wss_url);
    let (tx, mut rx) = mpsc::channel(100);

    let tx_poll = tx.clone();
    let rpc_url_poll = helius_rpc_url.clone().unwrap_or_else(|| rpc_url.clone());
    info!(
        "Signature poller configured on URL: {}",
        if helius_rpc_url.is_some() {
            "Helius RPC"
        } else {
            "Public RPC"
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
                    { "limit": 10 }
                ]
            });

            match client.post(&rpc_url_poll).json(&payload).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        warn!("Signature poller RPC status: {}", status);
                    } else if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(err) = json.get("error") {
                            warn!("Signature poller RPC returned error: {:?}", err);
                        } else if let Some(result) = json.get("result").and_then(|r| r.as_array()) {
                            let mut new_sigs_count = 0;
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

                                    let event = ObserverEvent {
                                        signature: sig.to_string(),
                                        slot: tx_info
                                            .get("slot")
                                            .and_then(|s| s.as_u64())
                                            .unwrap_or(0),
                                        program_id: "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
                                            .to_string(),
                                        payload: vec![],
                                    };

                                    if tx_poll.send(event).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            if new_sigs_count > 0 {
                                info!(
                                    "Signature poller: {} new signatures ingested.",
                                    new_sigs_count
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Signature poller RPC call failed: {}", e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    let watcher = Watcher::new(Some(target_wss_url));
    let tx_watch = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = watcher.watch(tx_watch).await {
            error!("Fatal watcher error: {}", e);
        }
    });

    info!("Cortex event loop ready.");

    while let Some(raw_event) = rx.recv().await {
        if let Err(e) = ingestion_buffer.push(raw_event.clone()) {
            warn!("Ingestion buffer capacity exceeded, event dropped: {}", e);
            let m = ingestion_buffer.metrics();
            warn!(
                "Buffer metrics: ingested={} dropped={} drop_rate={:.2}%",
                m.ingested_total,
                m.dropped_total,
                m.drop_rate * 100.0
            );
            continue;
        }

        let raw_event = match ingestion_buffer.drain(1).into_iter().next() {
            Some(ev) => ev,
            None => continue,
        };

        let current_metabolism = metabolism.current_load();
        let record = WalRecord::EventReceived(raw_event.clone());
        if let Err(e) = wal.append(&record, &current_metabolism) {
            error!("WAL append failure: {}", e);
        }

        let is_helius = std::str::from_utf8(&raw_event.payload)
            .map(|t| t.trim().starts_with('['))
            .unwrap_or(false);

        if is_helius {
            COMPARATOR.record_helius_event(1);
        }

        let mut resolved_event = raw_event.clone();

        if raw_event.signature != "UNKNOWN" && raw_event.signature != "SIMULATED_SIG" {
            info!(
                "Enriching transaction {} via public RPC...",
                raw_event.signature
            );

            let start_pub = std::time::Instant::now();
            let pub_res = fetch_transaction_details(&rpc_url, &raw_event.signature).await;

            let mut helius_res = None;
            if let Some(ref h_url) = helius_rpc_url {
                let start_hel = std::time::Instant::now();
                if let Ok(json) = fetch_transaction_details(h_url, &raw_event.signature).await {
                    let helius_latency = start_hel.elapsed().as_millis() as u64;
                    helius_res = Some(json);
                    COMPARATOR.record_helius_event(helius_latency);
                }
            }

            match pub_res {
                Ok(enriched_json) => {
                    let public_rpc_latency = start_pub.elapsed().as_millis() as u64;
                    resolved_event.payload = enriched_json.to_string().into_bytes();

                    let is_matched = if let Some(h_json) = helius_res {
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
                    COMPARATOR
                        .record_public_rpc_event(public_rpc_latency, is_matched && is_action_valid);

                    if helius_rpc_url.is_none() {
                        COMPARATOR.record_helius_event(5);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to enrich transaction {} via public RPC: {}",
                        raw_event.signature, e
                    );
                }
            }
        }

        let action = PumpFunParser::parse(&resolved_event);
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

        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs() as i64;
        let mapped = map_action(&action, now_unix);

        let wallet_signals_count = mapped.wallet_signals.len() as u64;
        let token_signals_count = mapped.token_signals.len() as u64;
        let total_signals = wallet_signals_count + token_signals_count;

        if total_signals > 0 {
            use cynic_intel::escore::metrics::ESCORE_METRICS;
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
                "E-Score: {} signals generated (Weight: {} ScoreUnits)",
                total_signals, total_weight
            );
        }

        match action {
            PumpFunAction::TokenCreated {
                token_address,
                user,
            } => {
                info!("NEW TOKEN: {} created by {}", token_address, user);
            }
            PumpFunAction::Trade {
                token_address,
                sol_amount,
                token_amount,
                is_buy,
            } => {
                let trade_type = if is_buy { "BUY" } else { "SELL" };
                info!(
                    "TRADE ({}): {} SOL ({} lamports) -> {} TOKENS on {}",
                    trade_type,
                    (sol_amount as f64 / 1_000_000_000.0),
                    sol_amount,
                    token_amount,
                    token_address
                );
            }
            PumpFunAction::CurveCompleted { token_address } => {
                info!("CURVE COMPLETED: Migration imminent for {}", token_address);
            }
            PumpFunAction::Unknown => {
                warn!(
                    "Raw event ignored or filtered (Signature: {})",
                    raw_event.signature
                );
            }
        }
    }

    Ok(())
}
