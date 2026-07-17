use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::{ObserverEvent, EventStatus};

/// Une entrée dans le Write-Ahead Log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalRecord {
    /// Un nouvel événement a été détecté par le watcher local ou reçu du P2P.
    EventReceived(ObserverEvent),
    /// Le consensus a statué sur l'événement.
    StatusUpdated { signature: String, status: EventStatus },
}

/// Le Write-Ahead Log (WAL) garantit la résilience du nœud Argo.
/// Il stocke les états en "append-only" (JSON Lines).
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
    pub fn append(&mut self, record: &WalRecord, metabolic_state: &cynic_host::ports::metabolism::MetabolicState) -> io::Result<()> {
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
            },
            MetabolicState::Survival => {
                // Pas de sync_data. On trade de la durabilité immédiate pour de la survie.
            }
        }
        
        Ok(())
    }

    /// Rejoue (Replay) le journal depuis le début pour reconstruire l'état mémoire
    /// de manière déterministe lors du (re)démarrage.
    pub fn replay<P: AsRef<Path>>(path: P) -> io::Result<Vec<WalRecord>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let mut records = Vec::new();

        for line_res in reader.lines() {
            let line = line_res?;
            if line.trim().is_empty() {
                continue;
            }
            
            // Si la ligne est corrompue (ex: crash en plein write), 
            // on ignore silencieusement ou on s'arrête.
            // Pour un WAL robuste, un crash au milieu d'une ligne marque la fin valide du WAL.
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
