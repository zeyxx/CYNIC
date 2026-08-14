use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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

    tracing::warn!(
        "DEPRECATED: --mcp flag will be removed. Use the standalone cynic-mcp binary instead."
    );
    use rmcp::ServiceExt;

    let raw_addr =
        std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| "http://127.0.0.1:3030".into());
    let rest_addr = if raw_addr.starts_with("http://") || raw_addr.starts_with("https://") {
        raw_addr
    } else {
        format!("http://{raw_addr}")
    };
    let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_default();
    tracing::info!(
        rest_addr,
        "MCP proxy mode — forwarding to REST kernel (zero local state)"
    );

    let mcp_proxy = crate::api::mcp::proxy::CynicMcpProxy::new(rest_addr, api_key);

    let shutdown = CancellationToken::new();
    crate::infra::tasks::spawn_signal_handler(shutdown.clone());

    let transport = rmcp::transport::io::stdio();
    let server = mcp_proxy
        .serve(transport)
        .await
        .map_err(|e| format!("MCP proxy error: {e}"))?;

    tokio::select! {
        _ = server.waiting() => {}
        _ = shutdown.clone().cancelled_owned() => {
            tracing::info!("MCP proxy shutting down");
        }
    }

    shutdown.cancel();
    Ok(true)
}
