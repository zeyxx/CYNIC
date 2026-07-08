from __future__ import annotations

from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())


def registry_fixture():
    return {
        "lifecycle": {
            "kernel_release": {
                "orchestrator_organ": "organ-forge",
                "task_kind": "hermes",
                "task_domain": "organ-forge",
                "build_runner_role": "build_runner",
                "deploy_target_role": "kernel_primary",
                "deploy_target_flag": "deploy_target",
                "required_node_status": "active",
                "build_policy": {
                    "artifact_targets": [
                        "crates/cynic-kernel",
                        "crates/cynic-mcp",
                        "crates/cynic-node",
                        "crates/cynic-cognitive-sensor",
                    ],
                    "rust_toolchain": "1.88.0",
                    "rust_release_target": "x86_64-unknown-linux-musl",
                    "reason": "Release binaries must run across Debian 12 Proxmox/LXC nodes and Ubuntu guests without glibc coupling.",
                    "scope": "runtime Rust binaries only; web, mobile, tests, and local cortex checks keep their native toolchains.",
                    "owner": "organ-forge",
                    "bootstrap": {
                        "project_slug": "CYNIC",
                        "principal": "cynic",
                        "preferred_transport": "fleet_node_exec",
                        "fallback_transport": "parent_provider_exec",
                        "transport_command_env": "CYNIC_FLEET_NODE_EXEC_COMMAND",
                        "transport_command_default": "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                        "repo_materialization": "rsync",
                        "toolchain_manager": "rustup",
                        "host_packages": ["ca-certificates", "musl-tools", "pkg-config", "libxi-dev"],
                        "watchdog_config_path": "%h/.config/cynic/node-watchdog.toml",
                        "watchdog_services": [
                            "organ-forge.service",
                            "cargo-build",
                            "gate-2",
                        ],
                    },
                },
                "activation_requirements": {
                    "build_runner": {
                        "network": ["synapse"],
                        "filesystem": ["repo_root"],
                        "services": ["organ-forge", "cargo-build", "gate-2"],
                    },
                    "kernel_primary": {
                        "network": ["synapse"],
                        "filesystem": ["repo_root"],
                        "services": ["cynic-kernel", "cynic-portal"],
                    },
                    "storage_primary": {
                        "network": ["synapse"],
                        "filesystem": [],
                        "services": ["surrealdb"],
                    },
                },
                "fleet_requirements": {
                    "provider": "synapse",
                    "required": True,
                    "require_online": True,
                },
                "kernel_bus_probe": {
                    "path": "/health",
                    "accepted_status": [200, 503],
                    "requires_bearer_auth": True,
                    "timeout_seconds": 10,
                },
                "mcp_freshness_controller": {
                    "schema": "cynic.mcp_freshness_controller.v1",
                    "status": "active",
                    "owner": "organ-forge",
                    "consumer": "organ-forge-hermes-agent",
                    "check_command": "CYNIC_MCP_CHECK_ONLY=1 python3 ./.cortex/mcp/run_cynic_mcp.py",
                    "dispatch_profile": "mcp-reload",
                    "dispatch_tool": "cynic_lifecycle_dispatch",
                    "fallback_command": "cynic mcp-reload",
                    "legacy_fallback_command": "make mcp-reload-request",
                    "debounce_seconds": 900,
                    "trigger_sources": [
                        "crates/cynic-mcp/src",
                        "infra/agent-exposure.json",
                        ".cortex/mcp/run_cynic_mcp.py",
                        "organs/exposure",
                    ],
                    "stale_statuses": ["stale", "missing", "failed", "timeout"],
                    "proof_requirements": [
                        "mcp-reload dispatch returns a task id or duplicate pending read-back",
                        "completed task is visible through Kernel agent task read-back",
                        "CYNIC_MCP_CHECK_ONLY=1 python3 ./.cortex/mcp/run_cynic_mcp.py reports fresh",
                        "organ-forge emits an observation for refresh decision and refresh result",
                    ],
                },
                "runtime_artifacts": [
                    {
                        "name": "cynic-mcp",
                        "kind": "rust_binary",
                        "owner": "organ-forge",
                        "source_targets": [
                            "crates/cynic-mcp/src",
                            "infra/agent-exposure.json",
                            ".cortex/mcp/run_cynic_mcp.py",
                            "organs/exposure",
                        ],
                        "build_targets": ["crates/cynic-mcp"],
                        "publish": {"path": "%h/bin/cynic-kernel-mcp", "hash": None},
                        "runtime_targets": ["cortex"],
                        "activation": [
                            "install executable",
                            "verify launcher freshness",
                            "reload MCP client session or mark reload-required",
                        ],
                        "proof": [
                            "CYNIC_MCP_CHECK_ONLY=1 python3 ./.cortex/mcp/run_cynic_mcp.py reports fresh",
                            "MCP initialize responds",
                            "MCP tools/list returns cynic_coord_claim",
                        ],
                        "lifecycle_stages": [
                            "source",
                            "build_or_package",
                            "publish",
                            "propagate",
                            "activate",
                            "prove",
                        ],
                        "request_profiles": ["mcp-reload"],
                    },
                    {
                        "name": "cynic-cognitive-sensor",
                        "kind": "rust_binary",
                        "owner": "organ-forge",
                        "source_targets": [
                            "crates/cynic-cognitive-sensor",
                            "axioms/cognitive-signal-v1.json",
                        ],
                        "build_targets": ["crates/cynic-cognitive-sensor"],
                        "system_packages": ["pkg-config", "libxi-dev"],
                        "publish": {"path": "%h/bin/cynic-cognitive-sensor", "hash": None},
                        "runtime_targets": ["cortex"],
                        "activation": [
                            "install executable",
                            "verify cognitive signal schema",
                            "start only on an approved operator host",
                        ],
                        "proof": [
                            "cargo check --manifest-path crates/cynic-cognitive-sensor/Cargo.toml passes on build runner",
                            "binary starts without missing XInput libraries",
                            "emits cynic.cognitive_signal.v1",
                        ],
                        "lifecycle_stages": [
                            "source",
                            "build_or_package",
                            "publish",
                            "propagate",
                            "activate",
                            "prove",
                        ],
                        "request_profiles": ["gate-2"],
                    },
                    {
                        "name": "cynic-node",
                        "kind": "rust_binary",
                        "owner": "organ-forge",
                        "source_targets": [
                            "crates/cynic-node",
                            "infra/systemd/cynic-node.service",
                            "axioms/node-bootstrap-v1.json",
                            "axioms/network-infra-gaps-v1.json",
                        ],
                        "build_targets": ["crates/cynic-node"],
                        "publish": {"path": "%h/bin/cynic-node", "hash": None},
                        "runtime_targets": ["all_runtime_nodes"],
                        "activation": [
                            "install executable",
                            "install declared watchdog or service configuration for the node role",
                            "start cynic-node only on nodes that declare the runtime surface",
                        ],
                        "proof": [
                            "cynic-node observe --json emits binary_attestation",
                            "cynic-node --watchdog emits cynic-node-watchdog payload",
                            "systemd unit or watchdog surface resolves to %h/bin/cynic-node",
                        ],
                        "lifecycle_stages": [
                            "source",
                            "build_or_package",
                            "publish",
                            "propagate",
                            "activate",
                            "prove",
                        ],
                        "request_profiles": ["bootstrap", "gate-2", "deploy"],
                    },
                    {
                        "name": "organ-forge",
                        "kind": "python_organ",
                        "owner": "organ-forge",
                        "source_targets": [
                            "organs/forge",
                            "infra/registry.json",
                            "axioms/lifecycle-supply-chain-v1.json",
                        ],
                        "build_targets": ["organs/forge"],
                        "publish": {
                            "path": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}/organs/forge",
                            "hash": None,
                        },
                        "runtime_targets": ["build_runner"],
                        "activation": [
                            "materialize repo under runtime principal",
                            "install Python dependencies",
                            "start hermes-agent-executor.service",
                        ],
                        "proof": [
                            "python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers returns activation.v1",
                            "organ-forge daemon active",
                            "lifecycle task read-back works",
                        ],
                        "lifecycle_stages": [
                            "source",
                            "build_or_package",
                            "publish",
                            "propagate",
                            "activate",
                            "prove",
                        ],
                        "request_profiles": ["bootstrap", "gate-2", "deploy", "mcp-reload"],
                    },
                    {
                        "name": "agent-exposure-projections",
                        "kind": "agent_projection",
                        "owner": "organ-exposure",
                        "source_targets": [
                            "infra/agent-exposure.json",
                            "organs/exposure",
                            ".codex/config.toml",
                            ".claude/settings.json",
                            ".antigravity/settings.json",
                        ],
                        "build_targets": ["organs/exposure"],
                        "publish": {"path": "agent-local config projections", "hash": None},
                        "runtime_targets": ["cortex"],
                        "activation": [
                            "render projection from infra/agent-exposure.json",
                            "install projection in each agent config",
                        ],
                        "proof": [
                            "python3 organs/exposure/src/entrypoints/cli.py render --agent codex is stable",
                            "make lint-agent-exposure-contract passes",
                        ],
                        "lifecycle_stages": [
                            "source",
                            "build_or_package",
                            "publish",
                            "propagate",
                            "activate",
                            "prove",
                        ],
                        "request_profiles": ["mcp-reload"],
                    },
                    {
                        "name": "lifecycle-systemd-units",
                        "kind": "systemd_unit",
                        "owner": "organ-lifecycle",
                        "source_targets": [
                            "infra/systemd",
                            "infra/registry.json",
                            "organs/lifecycle",
                        ],
                        "build_targets": ["infra/systemd"],
                        "publish": {"path": "runtime user systemd units", "hash": None},
                        "runtime_targets": [
                            "build_runner",
                            "kernel_primary",
                            "storage_primary",
                        ],
                        "activation": [
                            "install unit under runtime principal",
                            "systemctl daemon-reload",
                            "start or restart declared unit",
                        ],
                        "proof": [
                            "unit file matches registry source",
                            "service declared",
                            "service active",
                        ],
                        "lifecycle_stages": [
                            "source",
                            "build_or_package",
                            "publish",
                            "propagate",
                            "activate",
                            "prove",
                        ],
                        "request_profiles": ["bootstrap", "deploy", "mcp-reload"],
                    },
                ],
                "requires_e2e": True,
                "runtime_paths": {
                    "repo_root": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}",
                },
                "runtime_path_proofs": {
                    "repo_root": {
                        "kind": "git_worktree",
                        "marker": ".git",
                        "sentinels": ["AGENTS.md", "infra/registry.json"],
                    },
                },
                "activation_evidence_contract": {
                    "patch_field": "activation_evidence_patch",
                    "node_field": "node",
                    "runtime_paths_field": "activation_evidence.runtime_paths",
                    "services_field": "activation_evidence.services",
                    "verified_requires": ["marker_exists", "sentinels_verified"],
                    "service_active_requires": ["declared", "active"],
                    "ingestion_mode": "validated_merge",
                },
                "repo_targets": [
                    "crates/cynic-kernel",
                    "crates/cynic-mcp",
                    "infra/registry.json",
                    "Makefile",
                ],
                "request_profiles": {
                    "bootstrap": ["bootstrap-build-runner"],
                    "gate-2": ["build-and-test", "integration-test"],
                    "deploy": ["deploy-kernel"],
                    "mcp-reload": ["build-and-reload-mcp"],
                },
                "workflow": {
                    "name": "kernel_release",
                    "owner": "organ-forge",
                    "state_layers": {
                        "desired": {
                            "source": "human_intent",
                            "description": "What the operator wants the organism to do.",
                        },
                        "declared": {
                            "source": "infra/registry.json",
                            "description": "What the registry says should happen.",
                        },
                        "observed": {
                            "source": "runtime_probes",
                            "description": "What Proxmox, Synapse, and service evidence prove right now.",
                        },
                        "effective": {
                            "source": "organ-forge",
                            "description": "What the lifecycle organ authorizes for execution.",
                        },
                        "applied": {
                            "source": "organ-lifecycle",
                            "description": "What the registry-selected runner/runtime has executed through declared ports.",
                        },
                        "verified": {
                            "source": "kernel_readback",
                            "description": "What the Kernel, service health, fleet evidence, or activation evidence proves after execution.",
                        },
                    },
                    "transition_contract": {
                        "schema": "cynic.lifecycle_transition_contract.v1",
                        "layers": [
                            "desired",
                            "declared",
                            "observed",
                            "effective",
                            "applied",
                            "verified",
                        ],
                        "rule": "A lifecycle state may advance only when the next layer has machine-readable evidence and required read-back.",
                        "transitions": [
                            {
                                "from": "desired",
                                "to": "declared",
                                "authority": "infra/registry.json",
                                "evidence": ["request_profiles"],
                                "read_back_required": False,
                            },
                            {
                                "from": "declared",
                                "to": "observed",
                                "authority": "runtime_probes",
                                "evidence": ["fleet_inventory"],
                                "read_back_required": False,
                            },
                            {
                                "from": "observed",
                                "to": "effective",
                                "authority": "organ-forge",
                                "evidence": ["lifecycle_nodes_ready"],
                                "read_back_required": False,
                            },
                            {
                                "from": "effective",
                                "to": "applied",
                                "authority": "organ-lifecycle",
                                "evidence": ["claimed_agent_task"],
                                "read_back_required": True,
                            },
                            {
                                "from": "applied",
                                "to": "verified",
                                "authority": "kernel_readback",
                                "evidence": ["completed_agent_task"],
                                "read_back_required": True,
                            },
                        ],
                    },
                    "canonical_stages": [
                        {
                            "name": "local_validation",
                            "scope": "cortex-local",
                            "make_target": "check",
                            "allowed_when_lifecycle_planned": True,
                        },
                        {
                            "name": "bootstrap_request",
                            "scope": "organ-forge",
                            "make_target": "bootstrap-request",
                            "request_profile": "bootstrap",
                            "allowed_when_lifecycle_planned": True,
                        },
                        {
                            "name": "activation_request_generation",
                            "scope": "organ-forge",
                            "make_target": "lifecycle-activation-requests",
                            "allowed_when_lifecycle_planned": True,
                        },
                        {
                            "name": "activation_request_dispatch",
                            "scope": "organ-forge",
                            "make_target": "lifecycle-activation-dispatch",
                            "allowed_when_lifecycle_planned": False,
                        },
                        {
                            "name": "gate_2_request",
                            "scope": "organ-forge",
                            "make_target": "gate-2-request",
                            "request_profile": "gate-2",
                            "allowed_when_lifecycle_planned": False,
                        },
                        {
                            "name": "mcp_reload_request",
                            "scope": "organ-forge",
                            "make_target": "mcp-reload-request",
                            "request_profile": "mcp-reload",
                            "allowed_when_lifecycle_planned": False,
                        },
                        {
                            "name": "deploy_request",
                            "scope": "organ-forge",
                            "make_target": "deploy-request",
                            "request_profile": "deploy",
                            "allowed_when_lifecycle_planned": False,
                        },
                    ],
                    "planned_policy": {
                        "blocks_workflow_critical_edits": True,
                        "critical_targets": [
                            "infra/registry.json",
                            "Makefile",
                            "crates/cynic-kernel",
                            "crates/cynic-mcp",
                            "organs/forge",
                            "organs/forge/src/adapters/kernel_adapter.py",
                            "organs/forge/src/core/lifecycle_workflow.py",
                            "organs/forge/src/core/lifecycle_payloads.py",
                            "organs/forge/src/core/lifecycle_runtime.py",
                            "organs/forge/src/core/lifecycle_evidence.py",
                            "organs/forge/src/core/lifecycle_activation.py",
                            "organs/forge/src/core/lifecycle_readiness.py",
                        ],
                    },
                },
                "intents": {
                    "bootstrap-build-runner": {
                        "objective": "bootstrap registry build runner enough to run cargo, gate-2, and organ-forge lifecycle tasks",
                        "deploy": False,
                        "bootstrap_phase": True,
                        "targets": ["organs/forge", "infra/registry.json"],
                        "coord_targets": ["organs/forge", "infra/registry.json"],
                    },
                    "build-and-test": {
                        "objective": "run gate-2 validation on registry build runner",
                        "deploy": False,
                    },
                    "integration-test": {
                        "objective": "run integration tests from registry build runner against kernel primary",
                        "deploy": False,
                    },
                    "deploy-kernel": {
                        "objective": "build kernel release on registry build runner and deploy to kernel primary",
                        "deploy": True,
                    },
                    "build-and-reload-mcp": {
                        "objective": "build cynic MCP on registry build runner and reload the cortex MCP runtime",
                        "deploy": False,
                        "targets": [
                            "crates/cynic-mcp",
                            ".cortex/mcp/run_cynic_mcp.py",
                        ],
                        "coord_targets": [
                            "crates/cynic-mcp",
                            ".cortex/mcp/run_cynic_mcp.py",
                            ".codex/config.toml",
                        ],
                    },
                },
            }
        },
        "nodes": [
            {
                "name": "cynic-vm-anvil",
                "role": "build_runner",
                "network": {"synapse": "<SYNAPSE_ANVIL>"},
                "services": [
                    {"name": "organ-forge"},
                    {"name": "cargo-build"},
                    {"name": "gate-2"},
                ],
                "status": "planned",
            },
            {
                "name": "cynic-vm-core",
                "role": "kernel_primary",
                "deploy_target": True,
                "network": {"synapse": "<SYNAPSE_CORE>"},
                "services": [
                    {"name": "cynic-kernel"},
                    {"name": "cynic-portal"},
                ],
                "status": "planned",
            },
            {
                "name": "cynic-lxc-surrealdb",
                "role": "storage_primary",
                "network": {"synapse": "<SYNAPSE_STORAGE>"},
                "services": [{"name": "surrealdb"}],
                "status": "planned",
            },
            {
                "name": "cynic-host-cortex",
                "role": "cortex_host",
                "network": {"synapse": "10.20.0.8"},
                "host_exec_transport": {
                    "schema": "cynic.host_exec_transport.v1",
                    "kind": "manual_operator_surface",
                    "status": "declared",
                    "principal": "cynic",
                    "required_bridge": "declare and implement a canonical host exec transport for cortex/manual hosts before using fleet_node_exec on this node",
                    "reason": "This host is a native cortex operator surface, not a Proxmox guest resource.",
                },
                "status": "declared",
            },
        ],
        "organs": [{"name": "organ-forge"}],
    }


def registry_with_ready_anvil():
    registry = registry_fixture()
    node = registry["nodes"][0]
    node["network"]["synapse"] = "100.64.0.10"
    node["activation_evidence"] = {
        "runtime_paths": {
            "repo_root": {
                "verified": True,
                "verified_at": "2026-06-15T00:00:00Z",
                "value": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}",
                "expanded": "/" + "home/cynic/.cynic/repos/CYNIC",
                "proof_kind": "git_worktree",
                "marker_exists": True,
                "sentinels_verified": True,
            }
        },
        "services": {
            "organ-forge": {
                "declared": True,
                "active": True,
                "checked_at": "2026-06-15T00:00:00Z",
            },
            "cargo-build": {
                "declared": True,
                "active": True,
                "checked_at": "2026-06-15T00:00:00Z",
            },
            "gate-2": {
                "declared": True,
                "active": True,
                "checked_at": "2026-06-15T00:00:00Z",
            },
        },
    }
    return registry
