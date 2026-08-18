use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

use crate::domain::UniversalObserverEvent;
use crate::{EventStatus, ObserverEvent};

/// Une entrée dans le Write-Ahead Log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalRecord {
    /// Événement blockchain legacy détecté localement ou reçu du P2P.
    EventReceived(ObserverEvent),
    /// Événement universel multi-domaine (Blockchain, Fleet, System, Inference, Social).
    UniversalEventReceived(UniversalObserverEvent),
    /// Le consensus ou le moteur a statué sur l'événement.
    StatusUpdated {
        signature: String,
        status: EventStatus,
    },
}

/// Le Write-Ahead Log (WAL) garantit la résilience et la rejouabilité des observations.
/// Il stocke les états en "append-only" (JSON Lines).
#[derive(Debug)]
pub struct WriteAheadLog {
    file: File,
}

impl WriteAheadLog {
    /// Ouvre ou crée le fichier WAL.
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;

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

        // Logique adaptative: fsync si ressources suffisantes, asynchrone si survie
        use cynic_host::ports::metabolism::MetabolicState;
        match metabolic_state {
            MetabolicState::HighPerformance | MetabolicState::Standard => {
                self.file.sync_data()?;
            }
            MetabolicState::Survival => {
                // Pas de sync_data immédiat pour épargner le disque et l'énergie
            }
        }

        Ok(())
    }

    /// Rejoue (Replay) le journal depuis le début pour reconstruire l'état mémoire
    /// de manière déterministe lors du démarrage.
    pub fn replay<P: AsRef<Path>>(path: P) -> io::Result<Vec<WalRecord>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut records = Vec::new();

        for line_res in reader.lines() {
            let line = line_res?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<WalRecord>(&line) {
                Ok(record) => records.push(record),
                Err(e) => {
                    log::warn!("Corruption détectée dans le WAL, arrêt du replay : {}", e);
                    break;
                }
            }
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_host::ports::metabolism::MetabolicState;
    use tempfile::NamedTempFile;

    #[test]
    fn wal_appends_and_replays_universal_records() {
        let temp = NamedTempFile::new().expect("temp file");
        let path = temp.path();

        let mut wal = WriteAheadLog::new(path).expect("new wal");
        let event = UniversalObserverEvent::system_event(
            "node-01",
            "cpu",
            "load_avg",
            serde_json::json!({"load_1m": 0.5}),
        );
        let record = WalRecord::UniversalEventReceived(event.clone());

        wal.append(&record, &MetabolicState::Standard).expect("append");

        let replayed = WriteAheadLog::replay(path).expect("replay");
        assert_eq!(replayed.len(), 1);
        match &replayed[0] {
            WalRecord::UniversalEventReceived(replayed_event) => {
                assert_eq!(replayed_event.id, event.id);
            }
            _ => panic!("expected universal event record"),
        }
    }
}
