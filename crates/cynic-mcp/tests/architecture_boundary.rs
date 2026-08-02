use std::fs;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_required_file(root: &Path, relative: &str) -> String {
    let path = root.join(relative);
    match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => panic!("read {}: {error}", path.display()),
    }
}

#[test]
fn mcp_binary_stays_a_thin_stdio_bootstrap_adapter() {
    let root = crate_root();
    let main_rs = read_required_file(&root, "src/main.rs");
    let line_count = main_rs.lines().count();

    assert!(
        line_count <= 220,
        "cynic-mcp main.rs must remain a thin bootstrap adapter; current line count is {line_count}"
    );
    for forbidden in [
        "#[tool(",
        "#[tool_router(",
        "reqwest::Client::builder",
        "tokio::process::Command::new",
        "std::process::Command::new",
    ] {
        assert!(
            !main_rs.contains(forbidden),
            "main.rs must not own MCP tool or process/runtime client detail: {forbidden}"
        );
    }
    assert!(
        main_rs.contains("KernelSessionClient::bootstrap")
            && main_rs.contains("cynic_mcp::proxy::CynicMcpProxy::new")
            && main_rs.contains("rmcp::transport::io::stdio()"),
        "main.rs must stay focused on bounded kernel-session bootstrap, proxy composition, and stdio transport wiring"
    );
}

#[test]
fn kernel_auth_uses_the_local_session_transport_not_an_api_key() {
    let root = crate_root();
    let main_rs = read_required_file(&root, "src/main.rs");
    let proxy_rs = read_required_file(&root, "src/proxy.rs");
    let session_rs = read_required_file(&root, "src/kernel_session.rs");

    assert!(!main_rs.contains("CYNIC_API_KEY"));
    assert!(!proxy_rs.contains("bearer_auth(&self.api_key)"));
    assert!(session_rs.contains(".unix_socket(path)"));
    assert!(session_rs.contains("/auth/agent/root-launch"));
    assert!(session_rs.contains("/auth/agent/session/renew"));
    assert!(session_rs.contains("pub async fn maintain"));
    assert!(session_rs.contains("X-Cynic-Agent-Session"));
}

#[test]
fn mcp_never_accepts_authentication_secrets_through_a_tool_call() {
    let root = crate_root();
    for relative in [
        "src/proxy.rs",
        "src/routers/local.rs",
        "src/types_split/coord.rs",
    ] {
        let content = read_required_file(&root, relative);
        assert!(
            !content.contains("cynic_auth") && !content.contains("AuthParams"),
            "{relative} must not expose a manual secret-bearing MCP authentication fallback"
        );
    }
}

#[test]
fn canonical_mcp_startup_is_the_rust_binary_not_a_shell_wrapper() {
    let root = crate_root();
    let exposure_source = read_required_file(&root, "../../infra/agent-exposure.json");
    assert!(
        !exposure_source.contains("\"fallback_tools\"")
            && exposure_source.contains("\"operator_tools\""),
        "operator CLI surfaces must not be advertised as agent shell fallbacks"
    );
    let exposure: serde_json::Value = serde_json::from_str(&exposure_source)
        .unwrap_or_else(|error| panic!("parse infra/agent-exposure.json: {error}"));
    let command = exposure
        .pointer("/mcp_servers/0/command")
        .and_then(serde_json::Value::as_str);
    assert_eq!(command, Some("cynic-mcp"));
    assert!(
        command.is_none_or(|value| !value.ends_with(".sh") && !value.contains("bash")),
        "the canonical MCP startup surface must not delegate identity loading to a shell"
    );
}

#[test]
fn mcp_process_execution_terminates_in_an_injected_host_port() {
    let root = crate_root();
    let mut pending = vec![root.join("src")];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).expect("read MCP source directory") {
            let entry = entry.expect("read MCP source entry");
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let relative = path
                .strip_prefix(&root)
                .expect("MCP source remains under crate root")
                .to_string_lossy();
            let content = fs::read_to_string(&path).expect("read MCP Rust source");
            assert!(
                !content.contains("process::Command::new"),
                "MCP process execution must terminate behind the injected host argv port, not {relative}"
            );
        }
    }
}

#[test]
fn proxy_owns_router_composition_and_rest_forwarding() {
    let root = crate_root();
    let lib_rs = read_required_file(&root, "src/lib.rs");
    let proxy_rs = read_required_file(&root, "src/proxy.rs");
    let handler_rs = read_required_file(&root, "src/handler.rs");

    assert!(
        lib_rs.contains("pub mod local_tools;")
            && lib_rs.contains("pub mod kernel_session;")
            && lib_rs.contains("pub mod proxy;")
            && lib_rs.contains("pub mod types;"),
        "library surface must stay focused on local_tools, proxy, and types"
    );
    assert!(
        proxy_rs.contains("pub struct CynicMcpProxy")
            && proxy_rs.contains("pub fn tool_router() -> ToolRouter<Self>")
            && proxy_rs.contains("Self::tool_router_forward()")
            && proxy_rs.contains("Self::tool_router_local_agent_surfaces()")
            && proxy_rs.contains("Self::tool_router_local_lifecycle_inference()"),
        "proxy must own MCP router composition and proxy runtime state"
    );
    assert!(
        handler_rs.contains("#[tool_handler]")
            && handler_rs.contains("impl ServerHandler for CynicMcpProxy"),
        "handler.rs must own the ServerHandler implementation"
    );
}

#[test]
fn routers_and_local_tools_split_local_vs_forwarded_surfaces() {
    let root = crate_root();
    let local_router = read_required_file(&root, "src/routers/local.rs");
    let agent_surfaces = read_required_file(&root, "src/routers/local/agent_surfaces.rs");
    let forward_router = read_required_file(&root, "src/routers/forward.rs");
    let coord_router = read_required_file(&root, "src/routers/coord.rs");
    let docker_router = read_required_file(&root, "src/routers/docker.rs");
    let local_tools = read_required_file(&root, "src/local_tools.rs");

    assert!(
        local_router.contains("include!(\"local/agent_surfaces.rs\")")
            && agent_surfaces.contains("#[tool_router(router = tool_router_local_agent_surfaces",)
            && agent_surfaces.contains("async fn cynic_validate(")
            && agent_surfaces.contains("crate::local_tools::run_git"),
        "local router must project local validation planning and filesystem-local MCP tools"
    );
    assert!(
        forward_router.contains("#[tool_router(router = tool_router_forward"),
        "forward router must own forwarded MCP surfaces"
    );
    assert!(
        coord_router.contains("#[tool_router(router = tool_router_coord"),
        "coord router must own coordination MCP surfaces"
    );
    assert!(
        docker_router.contains("#[tool_router(router = tool_router_docker"),
        "docker router must own docker MCP surfaces"
    );
    assert!(
        local_tools.contains("ArgvExecutionPort")
            && local_tools.contains("ArgvExecutionRequest")
            && local_tools.contains("GitSourceControlAdapter")
            && local_tools.contains("run_docker_container_list")
            && !local_tools.contains("#[tool(")
            && !local_tools.contains("tokio::process::Command::new"),
        "local_tools.rs must own typed local command requests behind the injected argv port, not MCP annotations or direct process execution"
    );
}

#[test]
fn helper_and_types_modules_are_still_split_aggregators() {
    let root = crate_root();
    let helpers_rs = read_required_file(&root, "src/helpers.rs");
    let types_rs = read_required_file(&root, "src/types.rs");

    assert!(
        helpers_rs.contains("include!(\"helpers_split/network.rs\")")
            && helpers_rs.contains("include!(\"helpers_split/misc.rs\")"),
        "helpers.rs must remain an explicit aggregator over helpers_split modules"
    );
    assert!(
        types_rs.contains("include!(\"types_split/coord.rs\")")
            && types_rs.contains("include!(\"types_split/identity.rs\")"),
        "types.rs must remain an explicit aggregator over types_split modules"
    );
}

#[test]
fn mcp_project_root_resolution_stays_host_owned() {
    let root = crate_root();
    for relative in ["src/main.rs", "src/helpers_split/misc.rs"] {
        let content = read_required_file(&root, relative);
        assert!(
            !content.contains("repo_access::current_project_root")
                && !content.contains("use cynic_host::repo_access::current_project_root"),
            "{relative} must not bypass cynic-host runtime_registry for project root resolution"
        );
        assert!(
            !content.contains("resolve_runtime_project_root_or_cwd_fallback")
                && !content.contains("CYNIC_REPO_ROOT")
                && (content.contains("CYNIC_MCP_PROJECT_ROOT")
                    || content.contains("resolve_mcp_project_root")
                    || content.contains("current_dir()")
                    || content.contains("std::env::current_dir")),
            "{relative} must resolve project root through explicit env/cwd projection, not host runtime_registry"
        );
    }
}
