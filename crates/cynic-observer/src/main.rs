use cynic_observer::watcher::Watcher;
use cynic_observer::parser::{PumpFunParser, PumpFunAction};
use tokio::sync::mpsc;
use log::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialisation du logger avec un niveau par défaut INFO
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    // Initialisation du CryptoProvider pour Rustls (nécessaire pour WSS)
    rustls::crypto::ring::default_provider().install_default().ok();
    
    info!("=== CYNIC Observer (Argo Node) ===");
    
    // 1. Initialisation du Métabolisme Matériel
    use cynic_host::ports::metabolism::{MetabolismProvider, OsMetabolism};
    let mut metabolism = OsMetabolism::new();
    let current_state = metabolism.current_state();
    info!("État métabolique du nœud au démarrage : {:?}", current_state);
    
    // 2. Initialisation du Write-Ahead Log (WAL)
    use cynic_observer::wal::{WriteAheadLog, WalRecord};
    let mut wal = WriteAheadLog::new("argo_node.wal").expect("Impossible de créer le WAL");
    let recovered_events = WriteAheadLog::replay("argo_node.wal").unwrap_or_default();
    info!("WAL: {} événements restaurés en mémoire depuis le disque.", recovered_events.len());

    info!("Démarrage du sous-système sensoriel...");

    // Création du canal nerveux (Nervous System) entre le Watcher (I/O) et le Cortex (Parser)
    let (tx, mut rx) = mpsc::channel(100);

    // Initialisation du Watcher
    let watcher = Watcher::new(None); // Par défaut: wss://api.mainnet-beta.solana.com/

    // Spawn du Watcher dans une tâche asynchrone pour ne pas bloquer le thread principal
    tokio::spawn(async move {
        if let Err(e) = watcher.watch(tx).await {
            log::error!("Erreur fatale du Watcher: {}", e);
        }
    });

    info!("Le Cortex est en attente d'événements...");

    // Boucle principale du Cortex : réception et parsing
    while let Some(raw_event) = rx.recv().await {
        // Sauvegarde WAL (Adaptative)
        let current_metabolism = metabolism.current_state();
        let record = WalRecord::EventReceived(raw_event.clone());
        if let Err(e) = wal.append(&record, &current_metabolism) {
            log::error!("Échec de l'écriture WAL : {}", e);
        }

        // Le Watcher a capté un événement. On le passe au Parser.
        let action = PumpFunParser::parse(&raw_event);

        match action {
            PumpFunAction::TokenCreated { token_address, user } => {
                info!("🟢 NOUVEAU TOKEN: {} créé par {}", token_address, user);
            }
            PumpFunAction::Trade { token_address, sol_amount, token_amount, is_buy } => {
                let trade_type = if is_buy { "ACHAT" } else { "VENTE" };
                info!("💱 TRADE ({}): {} SOL -> {} TOKENS sur {}", trade_type, sol_amount, token_amount, token_address);
            }
            PumpFunAction::CurveCompleted { token_address } => {
                info!("🚀 CURVE COMPLETED: Migration imminente pour {}", token_address);
            }
            PumpFunAction::Unknown => {
                warn!("⚪ Événement brut ignoré (Signature: {})", raw_event.signature);
            }
        }
    }

    Ok(())
}
