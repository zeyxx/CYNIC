use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

use crate::{EventStatus, ObserverEvent};
use cynic_protocol::Epoch;

/// Une entrée dans le Write-Ahead Log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalRecord {
    /// Un nouvel événement a été détecté par le watcher local ou reçu du P2P.
    EventReceived(ObserverEvent),
    /// Le consensus a statué sur l'événement.
    StatusUpdated {
        signature: String,
        status: EventStatus,
    },
    /// Marqueur d'epoch : délimite une frontière temporelle dans le WAL.
    /// Ce marqueur permet le replay sélectif à partir d'un epoch donné.
    EpochMarker {
        /// L'epoch auquel ce marqueur est posé.
        epoch: Epoch,
        /// Le digest de l'état du registre au moment de ce marqueur.
        registry_digest: String,
        /// Horodatage Unix (pour la compatibilité et le debugging).
        timestamp_unix: i64,
    },
    /// Snapshot de l'état complet à un epoch donné.
    /// Permet la récupération rapide sans rejouer tout le WAL.
    Snapshot {
        /// L'epoch auquel ce snapshot a été pris.
        epoch: Epoch,
        /// Le digest de l'état snapshot.
        state_digest: String,
        /// Les signatures des événements inclus dans le snapshot.
        event_signatures: Vec<String>,
        /// Horodatage Unix.
        timestamp_unix: i64,
    },
}

/// Le Write-Ahead Log (WAL) garantit la résilience du nœud Argo.
/// Il stocke les états en "append-only" (JSON Lines).
///
/// Les entrées `EpochMarker` et `Snapshot` permettent :
/// - Le replay sélectif à partir d'un epoch donné
/// - La récupération rapide après redémarrage (rollback temporel possible)
/// - La synchronisation entre nœuds via des epochs certifiés
#[derive(Debug)]
pub struct WriteAheadLog {
    file: File,
}

impl WriteAheadLog {
    /// Ouvre ou crée le fichier WAL.
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(Self { file })
    }

    /// Ajoute une nouvelle entrée dans le journal (append-only).
    /// Le flush sur le disque (fsync) est adaptatif en fonction de l'état métabolique.
    pub fn append(
        &mut self,
        record: &WalRecord,
        metabolic_state: &cynic_host::ports::metabolism::MetabolicState,
    ) -> io::Result<()> {
        let mut json = serde_json::to_string(record)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        json.push('\n');

        self.file.write_all(json.as_bytes())?;

        // Logique adaptative: on n'appelle fsync (qui coûte très cher en énergie/disque)
        // que si on a de la batterie. Si on est en mode survie, on laisse l'OS gérer
        // le buffer cache en asynchrone pour épargner le matériel.
        use cynic_host::ports::metabolism::MetabolicState;
        match metabolic_state {
            MetabolicState::HighPerformance | MetabolicState::Standard => {
                self.file.sync_data()?;
            }
            MetabolicState::Survival => {
                // Pas de sync_data. On trade de la durabilité immédiate pour de la survie.
            }
        }

        Ok(())
    }

    /// Écrit un marqueur d'epoch dans le WAL.
    /// Ce marqueur délimite une frontière temporelle et permet le replay sélectif.
    pub fn mark_epoch(
        &mut self,
        epoch: Epoch,
        registry_digest: impl Into<String>,
        metabolic_state: &cynic_host::ports::metabolism::MetabolicState,
    ) -> io::Result<()> {
        let timestamp_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let record = WalRecord::EpochMarker {
            epoch,
            registry_digest: registry_digest.into(),
            timestamp_unix,
        };
        self.append(&record, metabolic_state)
    }

    /// Écrit un snapshot de l'état complet dans le WAL.
    /// Permet la récupération rapide sans rejouer tout le WAL.
    pub fn write_snapshot(
        &mut self,
        epoch: Epoch,
        state_digest: impl Into<String>,
        event_signatures: Vec<String>,
        metabolic_state: &cynic_host::ports::metabolism::MetabolicState,
    ) -> io::Result<()> {
        let timestamp_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let record = WalRecord::Snapshot {
            epoch,
            state_digest: state_digest.into(),
            event_signatures,
            timestamp_unix,
        };
        self.append(&record, metabolic_state)
    }

    /// Rejoue (Replay) le journal depuis le début pour reconstruire l'état mémoire
    /// de manière déterministe lors du (re)démarrage.
    pub fn replay<P: AsRef<Path>>(path: P) -> io::Result<Vec<WalRecord>> {
        Self::replay_from_epoch(path, Epoch::ZERO)
    }

    /// Rejoue le journal à partir d'un epoch donné.
    /// Cette fonction trouve le dernier `Snapshot` ou `EpochMarker` à ou après
    /// l'epoch spécifié, puis rejoue les enregistrements suivants.
    ///
    /// C'est le mécanisme de base pour le rollback temporel : si un nœud est
    /// désynchronisé à l'epoch N, il peut demander de rejouer à partir de l'epoch
    /// N-1 ou depuis le dernier snapshot disponible.
    pub fn replay_from_epoch<P: AsRef<Path>>(
        path: P,
        target_epoch: Epoch,
    ) -> io::Result<Vec<WalRecord>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut records = Vec::new();
        let mut found_epoch_marker = false;

        for line_res in reader.lines() {
            let line = line_res?;
            if line.trim().is_empty() {
                continue;
            }

            // Si la ligne est corrompue (ex: crash en plein write),
            // on ignore silencieusement ou on s'arrête.
            // Pour un WAL robuste, un crash au milieu d'une ligne marque la fin valide du WAL.
            match serde_json::from_str::<WalRecord>(&line) {
                Ok(record) => {
                    // Si on cherche un epoch spécifique, on doit passer les EpochMarkers
                    // jusqu'à ce que l'epoch cible soit atteint.
                    if target_epoch.id() > 0 {
                        if let WalRecord::EpochMarker { epoch, .. } = &record {
                            if epoch < target_epoch {
                                // On saute les enregistrements avant l'epoch cible
                                continue;
                            }
                            found_epoch_marker = true;
                        }
                        if !found_epoch_marker {
                            // Pas encore atteint l'epoch cible
                            continue;
                        }
                    }
                    records.push(record);
                }
                Err(e) => {
                    log::warn!("Corruption détectée dans le WAL, arrêt du replay : {}", e);
                    break;
                }
            }
        }

        Ok(records)
    }

    /// Trouve le dernier epoch marker connu dans le WAL.
    /// Utile pour déterminer à quel epoch le nœud est actuellement synchronisé.
    pub fn last_epoch_marker<P: AsRef<Path>>(path: P) -> io::Result<Option<Epoch>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut last_epoch = None;

        for line_res in reader.lines() {
            let line = line_res?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(WalRecord::EpochMarker { epoch, .. }) =
                serde_json::from_str::<WalRecord>(&line)
            {
                last_epoch = Some(epoch);
            }
        }

        Ok(last_epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_host::ports::metabolism::MetabolicState;

    fn test_wal_records() -> Vec<WalRecord> {
        vec![
            WalRecord::EventReceived(ObserverEvent {
                signature: "sig-1".to_string(),
                slot: 1000,
                program_id: "pump".to_string(),
                payload: vec![0u8; 32],
            }),
            WalRecord::EpochMarker {
                epoch: Epoch::new(1),
                registry_digest: "sha256:digest1".to_string(),
                timestamp_unix: 1_720_000_001,
            },
            WalRecord::Snapshot {
                epoch: Epoch::new(1),
                state_digest: "sha256:snapshot1".to_string(),
                event_signatures: vec!["sig-1".to_string()],
                timestamp_unix: 1_720_000_001,
            },
            WalRecord::EventReceived(ObserverEvent {
                signature: "sig-2".to_string(),
                slot: 1002,
                program_id: "pump".to_string(),
                payload: vec![1u8; 16],
            }),
            WalRecord::EpochMarker {
                epoch: Epoch::new(2),
                registry_digest: "sha256:digest2".to_string(),
                timestamp_unix: 1_720_000_003,
            },
        ]
    }

    #[test]
    fn epoch_marker_serializes_without_error() {
        let marker = WalRecord::EpochMarker {
            epoch: Epoch::new(5),
            registry_digest: "sha256:test".to_string(),
            timestamp_unix: 1_720_000_010,
        };

        let json = serde_json::to_string(&marker).unwrap();
        let round_trip: WalRecord = serde_json::from_str(&json).unwrap();

        if let WalRecord::EpochMarker { epoch, .. } = round_trip {
            assert_eq!(epoch, Epoch::new(5));
        } else {
            panic!("Expected EpochMarker after round-trip");
        }
    }

    #[test]
    fn snapshot_serializes_without_error() {
        let snapshot = WalRecord::Snapshot {
            epoch: Epoch::new(3),
            state_digest: "sha256:snap".to_string(),
            event_signatures: vec!["a".to_string(), "b".to_string()],
            timestamp_unix: 1_720_000_020,
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let round_trip: WalRecord = serde_json::from_str(&json).unwrap();

        if let WalRecord::Snapshot {
            epoch,
            event_signatures,
            ..
        } = round_trip
        {
            assert_eq!(epoch, Epoch::new(3));
            assert_eq!(event_signatures.len(), 2);
        } else {
            panic!("Expected Snapshot after round-trip");
        }
    }

    #[test]
    fn last_epoch_marker_extraction_works() {
        let records = test_wal_records();
        let temp_dir = std::env::temp_dir();
        let wal_path = temp_dir.join("test_wal_epoch_marker.jsonl");

        // Write records to a temp file
        {
            let mut file = std::fs::File::create(&wal_path).unwrap();
            for record in &records {
                let json = serde_json::to_string(record).unwrap();
                writeln!(file, "{}", json).unwrap();
            }
        }

        let last_epoch = WriteAheadLog::last_epoch_marker(&wal_path).unwrap();
        assert_eq!(last_epoch, Some(Epoch::new(2)));

        // Cleanup
        std::fs::remove_file(&wal_path).ok();
    }
}
