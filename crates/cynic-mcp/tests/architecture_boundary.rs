use std::fs;
use std::path::{Path, PathBuf};

<<<<<<< HEAD
fn mcp_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn mcp_domain_is_pure_zero_io() {
    let cargo_toml =
        fs::read_to_string(mcp_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden_crates = ["cynic-host", "cynic-node"];
    for forbidden in &forbidden_crates {
        assert!(
            !cargo_toml.contains(forbidden),
            "Architecture violation: cynic-mcp must never depend on host or node orchestrators ({forbidden})"
=======
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
        main_rs.contains("resolve_rest_addr()")
            && main_rs.contains("cynic_mcp::proxy::CynicMcpProxy::new")
            && main_rs.contains("rmcp::transport::io::stdio()"),
        "main.rs must stay focused on REST target resolution, proxy bootstrap, and stdio transport wiring"
    );
}

#[test]
fn proxy_owns_router_composition_and_rest_forwarding() {
    let root = crate_root();
    let lib_rs = read_required_file(&root, "src/lib.rs");
    let proxy_rs = read_required_file(&root, "src/proxy.rs");
    let handler_rs = read_required_file(&root, "src/handler.rs");

    assert!(
        lib_rs.contains("pub mod local_tools;")
            && lib_rs.contains("pub mod proxy;")
            && lib_rs.contains("pub mod types;"),
        "library surface must stay focused on local_tools, proxy, and types"
    );
    assert!(
        proxy_rs.contains("pub struct CynicMcpProxy")
            && proxy_rs.contains("pub fn tool_router() -> ToolRouter<Self>")
            && proxy_rs.contains("Self::tool_router_forward()")
            && proxy_rs.contains("Self::tool_router_local()"),
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
    let forward_router = read_required_file(&root, "src/routers/forward.rs");
    let coord_router = read_required_file(&root, "src/routers/coord.rs");
    let docker_router = read_required_file(&root, "src/routers/docker.rs");
    let local_tools = read_required_file(&root, "src/local_tools.rs");

    assert!(
        local_router.contains("#[tool_router(router = tool_router_local")
            && local_router.contains("crate::local_tools::run_validate")
            && local_router.contains("crate::local_tools::run_git"),
        "local router must project filesystem-local MCP tools"
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
        local_tools.contains("tokio::process::Command::new(\"make\")")
            && local_tools.contains("arg(\"check\")")
            && local_tools.contains("tokio::process::Command::new(\"git\")")
            && local_tools.contains("tokio::process::Command::new(\"docker\")")
            && !local_tools.contains("#[tool("),
        "local_tools.rs must own typed local command execution (make check + git + docker), not MCP annotations"
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
>>>>>>> github/main
        );
    }
}
