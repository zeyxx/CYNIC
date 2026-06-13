use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::process::Command;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

#[derive(Deserialize)]
pub struct DeployRequest {
    pub public_key: String,
    pub message: String,
    pub signature: String,
    pub action: String, // e.g., "deploy_asdf", "restart_vpn"
}

#[derive(Serialize)]
pub struct DeployResponse {
    pub status: String,
    pub logs: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("🚀 CYNIC Cloud API (Bare-Metal Gateway) starting...");

    let app = Router::new()
        .route(
            "/health",
            get(|| async { "CYNIC Cloud Gateway [Status: Active]" }),
        )
        .route("/api/deploy", post(handle_deploy))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    info!("🎧 Listening for Web3 requests on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_deploy(
    Json(payload): Json<DeployRequest>,
) -> Result<Json<DeployResponse>, (StatusCode, String)> {
    info!(
        "📥 Reçu demande d'action '{}' par le Wallet: {}",
        payload.action, payload.public_key
    );

    // --- PHASE 1: LA DOUANE (Vérification Cryptographique SIWS) ---
    let pubkey_bytes = bs58::decode(&payload.public_key).into_vec().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid pubkey base58: {}", e),
        )
    })?;

    let sig_bytes = bs58::decode(&payload.signature).into_vec().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid signature base58: {}", e),
        )
    })?;

    let pubkey_bytes_arr: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Pubkey wrong length".to_string()))?;

    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes_arr).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid Ed25519 pubkey: {}", e),
        )
    })?;

    let signature_arr: [u8; 64] = sig_bytes.try_into().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Signature wrong length".to_string(),
        )
    })?;

    let signature = Signature::from_bytes(&signature_arr);

    if verifying_key
        .verify(payload.message.as_bytes(), &signature)
        .is_err()
    {
        error!("🚨 ALERTE SÉCURITÉ: Signature forgée détectée !");
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid SIWS cryptographic signature".to_string(),
        ));
    }

    // Vérification sémantique : le message signé doit mentionner l'action pour éviter le replay attack
    if !payload.message.contains(&payload.action) {
        error!("🚨 ALERTE SÉCURITÉ: L'action demandée n'est pas dans le message signé.");
        return Err((
            StatusCode::UNAUTHORIZED,
            "Action mismatch in signature".to_string(),
        ));
    }

    info!("✅ Signature cryptographique mathématiquement valide.");

    // --- PHASE 2: LE MUSCLE ORCHESTRATEUR (Exécution Dynamique) ---
    info!("💪 Initiation de l'exécution bare-metal...");

    // Dictionnaire de commandes autorisées sur le Proxmox
    let (vmid, script) = match payload.action.as_str() {
        "deploy_asdf" => (
            "200",
            "cd /app/organ-asdf && docker-compose pull && docker-compose up -d".to_string(),
        ),
        "open_freebox" => (
            "host",
            "python3 /root/CYNIC/organs/organ-freebox/organism_freebox.py".to_string(),
        ),
        "restart_vpn" => ("100", "systemctl restart headscale".to_string()),
        "update_forgejo" => {
            (
                "101",
                "apt-get update && apt-get install -y forgejo".to_string(),
            ) // Exemple
        }
        _ => {
            error!("❌ Action inconnue ou non autorisée: {}", payload.action);
            return Err((StatusCode::BAD_REQUEST, "Action not supported".to_string()));
        }
    };

    let output = if vmid == "host" {
        Command::new("bash").arg("-c").arg(&script).output()
    } else {
        Command::new("pct")
            .arg("exec")
            .arg(vmid)
            .arg("--")
            .arg("bash")
            .arg("-c")
            .arg(&script)
            .output()
    }
    .map_err(|e| {
        error!("Erreur système lors de l'appel d'exécution: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Execution failed".to_string(),
        )
    })?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        info!(
            "✅ Succès de '{}' sur LXC {}:\n{}",
            payload.action, vmid, stdout
        );
        Ok(Json(DeployResponse {
            status: "success".to_string(),
            logs: stdout,
        }))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!(
            "❌ Échec de '{}' sur LXC {}:\n{}",
            payload.action, vmid, stderr
        );
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("LXC Script Failed: {}", stderr),
        ))
    }
}
