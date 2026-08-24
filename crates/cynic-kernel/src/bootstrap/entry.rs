use tracing_subscriber::{EnvFilter, fmt, prelude::*};

// Unix-only metadata accessors for the bootstrap secret-integrity checks
// (raw st_mode via PermissionsExt, uid/gid via MetadataExt).
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelEntryFlags {
    pub force_reprobe: bool,
    pub mcp_mode: bool,
    pub version_requested: bool,
}

impl KernelEntryFlags {
    pub fn from_env_args() -> Self {
        let args = std::env::args().collect::<Vec<_>>();
        Self {
            force_reprobe: args.iter().any(|arg| arg == "--reset"),
            mcp_mode: args.iter().any(|arg| arg == "--mcp"),
            version_requested: args.iter().any(|arg| arg == "--version"),
        }
    }
}

pub fn load_env_file() {
    let _env_loaded = dotenvy::dotenv().ok();
    tracing::trace!("env loaded from .env (if present)");
}

pub fn print_version_and_exit_if_requested(
    flags: KernelEntryFlags,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !flags.version_requested {
        return Ok(false);
    }

    // WHY: the explicit --version response is a CLI stdout contract, not runtime logging.
    #[allow(clippy::print_stdout)]
    {
        println!(
            "cynic-kernel {} (build {}, sha {}, profile {})",
            env!("CARGO_PKG_VERSION"),
            env!("CYNIC_VERSION"),
            env!("CYNIC_GIT_SHA"),
            env!("CYNIC_BUILD_PROFILE")
        );
    }

    Ok(true)
}

pub fn configure_stdio_mode(flags: KernelEntryFlags) {
    if flags.mcp_mode {
        crate::MCP_MODE.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn init_tracing(flags: KernelEntryFlags) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("cynic_kernel=info,warn"));

    if flags.mcp_mode {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json().with_writer(std::io::stderr))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .init();
    }
}

pub async fn run_mcp_proxy_mode_if_requested(
    flags: KernelEntryFlags,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !flags.mcp_mode {
        return Ok(false);
    }

    Err(std::io::Error::other(
        "blocked_deprecated_surface: kernel --mcp was removed; use the standalone cynic-mcp binary with a kernel-issued local agent session",
    )
    .into())
}

/// Verify bootstrap integrity including secrets, registry seal, and file permissions.
#[tokio::main]
pub async fn main_verify() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use std::process::Command;
    use sha2::{Sha256, Digest};
    
    klog!("CYNIC V2 — BOOTSTRAP VERIFY");
    
    // 1. Check storage-runtime-password integrity
    klog!("Checking storage-runtime-password...");
    let password_path = std::env::var("CYNIC_STORAGE_PASSWORD_PATH").unwrap_or_else(|_| "/etc/cynic/secrets/storage-runtime-password".to_string());
    
    let length = std::fs::read(&password_path)
        .map_err(|e| format!("cannot read password file: {e}"))?
        .len();
    
    let perms = std::fs::metadata(&password_path)
        .map_err(|e| format!("cannot read metadata: {e}"))?
        .permissions();
    let perms_mode = perms.mode();
    let perms_ok = perms_mode == 0o640;

    let owner = std::fs::metadata(&password_path)
        .map(|meta| format!("{}:{}", meta.uid(), meta.gid()))
        .unwrap_or_default();
    let owner_ok = owner == "0:1001";  // root:cynic
    
    let immutable_check = std::process::Command::new("chattr")
        .args(["-v", "+", "i", "&", &password_path])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    // 2. Check registry password reference
    klog!("Checking registry secret seal...");
    let registry_path = std::env::var("CYNIC_REGISTRY_PATH").unwrap_or_else(|_| "/etc/cynic/registry.json".to_string());
    let registry_content = std::fs::read_to_string(&registry_path)
        .map_err(|e| format!("cannot read registry: {e}"))?;
    let registry: serde_json::Value = serde_json::from_str(&registry_content)
        .map_err(|e| format!("cannot parse registry: {e}"))?;
    
    // Look for password_reference in nodes
    let password_reference = registry
        .get("nodes")
        .and_then(|nodes| nodes.as_array())
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|n| n.get("id").map(|id| id == "local-kernel").unwrap_or(false))
        })
        .and_then(|node| node.get("password_reference"));
    
    let reference_ok = if let Some(reference) = password_reference {
        let file_content = std::fs::read(&password_path).unwrap_or_default();
        let hash = Sha256::new().chain_update(&file_content).finalize();
        let hash_hex = format!("sha256:{}", hex::encode(hash));
        let expected = reference.as_str().unwrap_or("").to_string();
        let ok = hash_hex == expected;
        klog!("password reference check: expected={}, actual={}, ok={}", expected, hash_hex, ok);
        ok
    } else {
        klog!("no password_reference in registry nodes - skipping");
        true  // skip if not configured
    };
    
    // 3. Report result
    klog!("bootstrap verify result:");
    klog!("  password_length: {} bytes", length);
    klog!("  perms_ok: {}", perms_ok);
    klog!("  owner_ok: {}", owner_ok);
    klog!("  immutable: {}", immutable_check);
    klog!("  reference_ok: {}", reference_ok);
    
    if length != 5 || !perms_ok || !owner_ok || !reference_ok {
        return Err(format!(
            "bootstrap verify failed: length={}, perms_ok={}, owner_ok={}, reference_ok={}",
            length, perms_ok, owner_ok, reference_ok
        ).into());
    }
    
    klog!("bootstrap verify PASSED");
    Ok(())
}
