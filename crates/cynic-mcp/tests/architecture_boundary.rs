use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn mcp_binary_stays_a_thin_stdio_bootstrap_adapter() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
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
    let lib_rs = fs::read_to_string(root.join("src/lib.rs")).expect("read lib.rs");
    let proxy_rs = fs::read_to_string(root.join("src/proxy.rs")).expect("read proxy.rs");
    let handler_rs = fs::read_to_string(root.join("src/handler.rs")).expect("read handler.rs");

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
    let local_router =
        fs::read_to_string(root.join("src/routers/local.rs")).expect("read routers/local.rs");
    let forward_router =
        fs::read_to_string(root.join("src/routers/forward.rs")).expect("read routers/forward.rs");
    let coord_router =
        fs::read_to_string(root.join("src/routers/coord.rs")).expect("read routers/coord.rs");
    let docker_router =
        fs::read_to_string(root.join("src/routers/docker.rs")).expect("read routers/docker.rs");
    let local_tools =
        fs::read_to_string(root.join("src/local_tools.rs")).expect("read local_tools.rs");

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
    let helpers_rs = fs::read_to_string(root.join("src/helpers.rs")).expect("read helpers.rs");
    let types_rs = fs::read_to_string(root.join("src/types.rs")).expect("read types.rs");

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
        let content = fs::read_to_string(root.join(relative))
            .unwrap_or_else(|err| panic!("read {relative}: {err}"));
        assert!(
            !content.contains("repo_access::current_project_root")
                && !content.contains("use cynic_host::repo_access::current_project_root"),
            "{relative} must not bypass cynic-host runtime_registry for project root resolution"
        );
        assert!(
            content.contains("resolve_runtime_project_root_or_cwd_fallback"),
            "{relative} must use host-owned classified project root resolution"
        );
    }
}
