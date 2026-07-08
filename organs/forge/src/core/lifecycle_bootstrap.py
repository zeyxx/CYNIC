"""Pure bootstrap contracts for registry-driven lifecycle nodes."""

from __future__ import annotations


BUILD_RUNNER_BOOTSTRAP_SCHEMA = "cynic.lifecycle.build_runner_bootstrap.v1"


def build_runner_bootstrap_contract(
    lifecycle: dict,
    node: dict,
) -> dict:
    """Return the deterministic bootstrap contract for a build runner node."""

    build_policy = lifecycle.get("build_policy", {})
    bootstrap_policy = build_policy.get("bootstrap", {})
    runtime_paths = lifecycle.get("runtime_paths", {})
    repo_root = runtime_paths.get("repo_root")
    rust_toolchain = build_policy.get("rust_toolchain", "stable")
    rust_release_target = build_policy.get(
        "rust_release_target",
        "x86_64-unknown-linux-musl",
    )
    node_services = [
        service.get("name")
        for service in node.get("services", [])
        if isinstance(service, dict) and service.get("name")
    ]
    watchdog_services = bootstrap_policy.get("watchdog_services") or node_services

    return {
        "schema": BUILD_RUNNER_BOOTSTRAP_SCHEMA,
        "node": node.get("name"),
        "role": node.get("role"),
        "constellation": node.get("constellation", "forge"),
        "source": "infra/registry.json:lifecycle.kernel_release.build_policy",
        "access": {
            "preferred_transport": bootstrap_policy.get(
                "preferred_transport",
                "fleet_node_exec",
            ),
            "fallback_transport": bootstrap_policy.get(
                "fallback_transport",
                "parent_provider_exec",
            ),
            "transport_command_env": bootstrap_policy.get(
                "transport_command_env",
                "CYNIC_FLEET_NODE_EXEC_COMMAND",
            ),
            "transport_command_default": bootstrap_policy.get(
                "transport_command_default",
                "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
            ),
            "principal": bootstrap_policy.get("principal", "cynic"),
        },
        "repo": {
            "path": repo_root,
            "project_slug": bootstrap_policy.get("project_slug", "CYNIC"),
            "materialization": bootstrap_policy.get("repo_materialization", "rsync"),
            "source_path_env": bootstrap_policy.get("source_path_env", "CYNIC_PROJECT_ROOT"),
            "source": {
                "remote": bootstrap_policy.get("repo_source", {}).get("remote"),
                "ref": bootstrap_policy.get("repo_source", {}).get("ref", "main"),
                "ref_env": bootstrap_policy.get("repo_source", {}).get(
                    "ref_env",
                    "CYNIC_FORGEJO_BOOTSTRAP_REF",
                ),
                "transport": bootstrap_policy.get("repo_source", {}).get("transport", "ssh"),
                "host_role": bootstrap_policy.get("repo_source", {}).get("host_role", "forge_primary"),
                "fallback_remote_env": bootstrap_policy.get("repo_source", {}).get(
                    "fallback_remote_env",
                    "CYNIC_FORGEJO_BOOTSTRAP_REMOTE",
                ),
                "fallback_transport": bootstrap_policy.get("repo_source", {}).get(
                    "fallback_transport",
                    "local_tunnel",
                ),
            },
            "proof": lifecycle.get("runtime_path_proofs", {}).get("repo_root", {}),
        },
        "toolchain": {
            "manager": bootstrap_policy.get("toolchain_manager", "rustup"),
            "rust_toolchain": rust_toolchain,
            "rust_release_target": rust_release_target,
            "host_packages": bootstrap_policy.get(
                "host_packages",
                ["ca-certificates", "musl-tools"],
            ),
        },
        "build": {
            "artifact_targets": build_policy.get("artifact_targets", []),
            "commands": [
                f"cargo +{rust_toolchain} check -p cynic-node --target {rust_release_target}",
                f"cargo +{rust_toolchain} build -p cynic-node --release --target {rust_release_target}",
            ],
        },
        "watchdog": {
            "binary": f"target/{rust_release_target}/release/cynic-node",
            "config_path": bootstrap_policy.get(
                "watchdog_config_path",
                "%h/.config/cynic/node-watchdog.toml",
            ),
            "mode": "l0_watchdog",
            "requires_dog_backend": False,
            "kernel_url_source": "CYNIC_REST_ADDR",
            "api_key_env": "CYNIC_API_KEY",
            "services": watchdog_services,
        },
        "proofs": [
            "runtime_path_git_worktree_verified",
            "rust_musl_target_installed",
            "cynic_node_release_binary_static",
            "watchdog_observation_received",
        ],
    }
