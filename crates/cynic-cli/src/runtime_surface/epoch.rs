use std::process::ExitCode;

use clap::CommandFactory;
use cynic_protocol::Epoch;

use crate::cli_json::emit_pretty_json;

/// Inspecte ou avance l'epoch logique du kernel via l'API REST.
pub(crate) fn execute_epoch(json: bool, advance: bool, endpoint: Option<String>) -> ExitCode {
    let endpoint = endpoint.unwrap_or_else(|| "http://127.0.0.1:3030".to_string());

    if advance {
        // Mutation : POST /temporal/epoch/advance
        let client = reqwest::blocking::Client::new();
        let result = client
            .post(format!("{}/temporal/epoch/advance", endpoint.trim_end_matches('/')))
            .send();

        match result {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    eprintln!("Échec de l'avance d'epoch: {}", status);
                    return ExitCode::from(1);
                }
                match resp.json::<serde_json::Value>() {
                    Ok(body) => {
<<<<<<< HEAD
                        emit_pretty_json(&body);
=======
                        emit_pretty_json(&body, json);
>>>>>>> github/main
                    }
                    Err(e) => {
                        eprintln!("Réponse invalide du kernel: {}", e);
                        return ExitCode::from(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Impossible de contacter le kernel sur {}: {}", endpoint, e);
                return ExitCode::from(1);
            }
        }
    } else {
        // Lecture : GET /temporal/epoch
        let client = reqwest::blocking::Client::new();
        let result = client
            .get(format!("{}/temporal/epoch", endpoint.trim_end_matches('/')))
            .send();

        match result {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    eprintln!("Échec de l'interrogation de l'epoch: {}", status);
                    return ExitCode::from(1);
                }
                match resp.json::<serde_json::Value>() {
                    Ok(body) => {
<<<<<<< HEAD
                        emit_pretty_json(&body);
=======
                        emit_pretty_json(&body, json);
>>>>>>> github/main
                    }
                    Err(e) => {
                        eprintln!("Réponse invalide du kernel: {}", e);
                        return ExitCode::from(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Impossible de contacter le kernel sur {}: {}", endpoint, e);
                return ExitCode::from(1);
            }
        }
    }

    // Marqueur de non-erreur pour le compilateur (utilise Epoch pour typage)
    let _ = Epoch::ZERO;
    ExitCode::SUCCESS
}
