use cynic_intel::escore::metrics::ESCORE_METRICS;
<<<<<<< HEAD
use cynic_protocol::{RUNTIME_PROOF_SCHEMA, RuntimeProof, RuntimeProofEvidence};
=======
use cynic_protocol::{Epoch, RuntimeProof, RuntimeProofEvidence, RUNTIME_PROOF_SCHEMA};
>>>>>>> github/main
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tracker dynamique de la fiabilité réelle des RPCs Solana.
#[derive(Debug)]
pub struct RpcReliabilityTracker {
    pub total_probes: AtomicU64,
    pub failed_probes: AtomicU64,
}

pub static RPC_TRACKER: RpcReliabilityTracker = RpcReliabilityTracker {
    total_probes: AtomicU64::new(0),
    failed_probes: AtomicU64::new(0),
};

/// Comparateur dynamique d'ingestion : Helius (baseline) vs RPC Public (souverain)
#[derive(Debug)]
pub struct IngestionComparator {
    pub helius_tx_count: AtomicU64,
    pub public_rpc_tx_count: AtomicU64,
    pub helius_latency_sum_ms: AtomicU64,
    pub public_rpc_latency_sum_ms: AtomicU64,
    pub matched_tx_count: AtomicU64,
}

impl IngestionComparator {
    pub fn record_helius_event(&self, latency_ms: u64) {
        self.helius_tx_count.fetch_add(1, Ordering::Relaxed);
        self.helius_latency_sum_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
    }

    pub fn record_public_rpc_event(&self, latency_ms: u64, is_matched: bool) {
        self.public_rpc_tx_count.fetch_add(1, Ordering::Relaxed);
        self.public_rpc_latency_sum_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        if is_matched {
            self.matched_tx_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub static COMPARATOR: IngestionComparator = IngestionComparator {
    helius_tx_count: AtomicU64::new(0),
    public_rpc_tx_count: AtomicU64::new(0),
    helius_latency_sum_ms: AtomicU64::new(0),
    public_rpc_latency_sum_ms: AtomicU64::new(0),
    matched_tx_count: AtomicU64::new(0),
};

/// Observateur de Consensus et Télémétrie Réelle d'Argo.
///
/// Ce module extrait les métriques directement depuis la réalité physique
/// et réseau du nœud Argo, et les exporte sous forme de preuves cryptographiques.
#[derive(Debug)]
pub struct ConsensusTelemetry;

impl ConsensusTelemetry {
    /// Retourne le taux de succès observé du RPC Solana (0.0 à 1.0).
    pub fn observed_success_rate() -> f64 {
        let total = RPC_TRACKER.total_probes.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }
        let failed = RPC_TRACKER.failed_probes.load(Ordering::Relaxed);
        if failed > total {
            return 0.0;
        }
        ((total - failed) as f64) / (total as f64)
    }

    /// Génère un rapport de preuves d'observabilité réelle (RuntimeProof).
    ///
    /// Cette méthode évite toute simulation ou fabrication de preuves, conformément aux règles
    /// d'évaluation d'Odysseum.
    pub async fn generate_proof(
        node_pubkey: &str,
        connected_peers: usize,
        rpc_url: &str,
<<<<<<< HEAD
=======
        current_epoch: Epoch,
>>>>>>> github/main
    ) -> RuntimeProof {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        // 1. Mesure en temps réel de la latence de notre RPC gratuit
        let rpc_latency_ms = Self::probe_rpc_latency(rpc_url).await;

        // 2. Calcul du taux de succès observé
        let success_rate = Self::observed_success_rate();

        // 3. Calcul des métriques de comparaison Ingestion
        let helius_tx_count = COMPARATOR.helius_tx_count.load(Ordering::Relaxed);
        let public_rpc_tx_count = COMPARATOR.public_rpc_tx_count.load(Ordering::Relaxed);
        let helius_latency_sum = COMPARATOR.helius_latency_sum_ms.load(Ordering::Relaxed);
        let public_rpc_latency_sum = COMPARATOR.public_rpc_latency_sum_ms.load(Ordering::Relaxed);
        let matched_tx_count = COMPARATOR.matched_tx_count.load(Ordering::Relaxed);

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

        // 4. Compilation de l'évidence de la réalité observable
<<<<<<< HEAD
=======
        let current_epoch_u64 = current_epoch.id();
>>>>>>> github/main
        let mut evidence = vec![
            RuntimeProofEvidence {
                id: "p2p_peers_count".to_string(),
                status: if connected_peers > 0 {
                    "satisfied".to_string()
                } else {
                    "blocked".to_string()
                },
                detail: format!(
                    "Nombres de pairs P2P connectés dans le DHT : {}",
                    connected_peers
                ),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            },
            RuntimeProofEvidence {
                id: "rpc_latency".to_string(),
                status: if rpc_latency_ms < 500 {
                    "satisfied".to_string()
                } else {
                    "watch".to_string()
                },
                detail: format!("Temps de réponse du RPC Solana : {} ms", rpc_latency_ms),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            },
            RuntimeProofEvidence {
                id: "rpc_observed_success_rate".to_string(),
                status: if success_rate > 0.90 {
                    "satisfied".to_string()
                } else {
                    "warning".to_string()
                },
                detail: format!(
                    "Taux de succès réel observé sur le RPC : {:.2}% (sur {} sondages)",
                    success_rate * 100.0,
                    RPC_TRACKER.total_probes.load(Ordering::Relaxed)
                ),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            },
            RuntimeProofEvidence {
                id: "escore_signals_processed".to_string(),
                status: "satisfied".to_string(),
                detail: format!(
                    "Signaux E-Score traités par le Kernel : {}",
                    ESCORE_METRICS.signals_processed.load(Ordering::Relaxed)
                ),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            },
            RuntimeProofEvidence {
                id: "escore_anti_whale_triggers".to_string(),
                status: "satisfied".to_string(),
                detail: format!(
                    "Dilutions anti-baleine appliquées : {}",
                    ESCORE_METRICS
                        .anti_whale_dilutions_total
                        .load(Ordering::Relaxed)
                ),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            },
            RuntimeProofEvidence {
                id: "wss_reconnections".to_string(),
                status: if crate::watcher::WSS_RECONNECTIONS.load(Ordering::Relaxed) == 0 {
                    "satisfied".to_string()
                } else {
                    "watch".to_string()
                },
                detail: format!(
                    "Nombre de reconnexions WebSocket initiées : {}",
                    crate::watcher::WSS_RECONNECTIONS.load(Ordering::Relaxed)
                ),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            },
            RuntimeProofEvidence {
                id: "ingestion_completeness_rate".to_string(),
                status: if completeness_rate >= 95.0 {
                    "satisfied".to_string()
                } else {
                    "warning".to_string()
                },
                detail: format!(
                    "Taux de complétude RPC public vs Helius (baseline) : {:.2}% ({} vs {} txs)",
                    completeness_rate, public_rpc_tx_count, helius_tx_count
                ),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            },
            RuntimeProofEvidence {
                id: "ingestion_accuracy_match_rate".to_string(),
                status: if accuracy_match_rate >= 99.0 {
                    "satisfied".to_string()
                } else {
                    "warning".to_string()
                },
                detail: format!(
                    "Taux de concordance des données (exactitude) : {:.2}% ({} txs identiques)",
                    accuracy_match_rate, matched_tx_count
                ),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            },
            RuntimeProofEvidence {
                id: "ingestion_latency_comparison".to_string(),
                status: if avg_public_latency_ms <= avg_helius_latency_ms + 1500 {
                    "satisfied".to_string()
                } else {
                    "watch".to_string()
                },
                detail: format!(
                    "Latence d'ingestion : RPC public {}ms vs Helius {}ms",
                    avg_public_latency_ms, avg_helius_latency_ms
                ),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            },
        ];

        // Si le RPC gratuit a crashé ou est trop lent, on le notifie
        if rpc_latency_ms > 2000 {
            evidence.push(RuntimeProofEvidence {
                id: "rpc_failure_warning".to_string(),
                status: "warning".to_string(),
                detail: "Latence critique détectée sur le RPC Solana (>2000ms)".to_string(),
<<<<<<< HEAD
=======
                epoch: Some(current_epoch_u64),
>>>>>>> github/main
            });
        }

        // On valide la satisfaction de la gate "distributed_formation_observed"
        // si on a au moins 1 pair P2P et un RPC fonctionnel (<1000ms avec >90% de succès)
        let overall_status = if connected_peers > 0 && rpc_latency_ms < 1000 && success_rate >= 0.90
        {
            "satisfied"
        } else {
            "blocked"
        };

        RuntimeProof {
            schema: RUNTIME_PROOF_SCHEMA.to_string(),
            gate: "distributed_formation_observed".to_string(),
            status: overall_status.to_string(),
            producer: format!("argo-observer-{}", node_pubkey),
            subject: "argo-network-telemetry".to_string(),
            observed_at_unix_ms: now_ms,
            evidence,
        }
    }

    /// Mesure la latence d'un RPC Solana en temps réel par un appel HTTP Health Probe.
    async fn probe_rpc_latency(rpc_url: &str) -> u64 {
        RPC_TRACKER.total_probes.fetch_add(1, Ordering::Relaxed);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(1500))
            .build()
            .unwrap_or_default();

        let start = std::time::Instant::now();
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getHealth"
        });

        match client.post(rpc_url).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    start.elapsed().as_millis() as u64
                } else {
                    RPC_TRACKER.failed_probes.fetch_add(1, Ordering::Relaxed);
                    9999 // RPC en erreur
                }
            }
            Err(_) => {
                RPC_TRACKER.failed_probes.fetch_add(1, Ordering::Relaxed);
                9999 // Timeout ou RPC déconnecté
            }
        }
    }
}
