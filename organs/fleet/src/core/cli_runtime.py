from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

from organs.contracts.src.surface_contracts import ReportSurfaceSpec, attach_surface_contract
from organs.fleet.src.adapters.coredns_adapter import CoreDNSAdapter
from organs.fleet.src.adapters.fleet_cli_runtime import build_report_payload
from organs.fleet.src.adapters.host_exec_adapter import HostExecPolicyFile
from organs.fleet.src.adapters.lxc_exec_bridge_adapter import LxcExecBridgePolicyFile
from organs.fleet.src.adapters.network_actuator_adapter import NetworkActuatorPolicyFile
from organs.fleet.src.adapters.registry_adapter import RegistryAdapter

from .network_surfaces import coredns_corefile
from .network_surfaces import coredns_systemd_unit
from .network_surfaces import dns_activation_plan
from .network_surfaces import dns_activation_readme
from .network_surfaces import dns_provider_policy
from .network_surfaces import dns_provider_status
from .network_surfaces import dns_records_from_hosts
from .network_surfaces import file_contains
from .network_surfaces import mesh_status_json
from .network_surfaces import mesh_status_payload
from .network_surfaces import observe_network_runtime
from .network_surfaces import provider_connectivity_status
from .network_surfaces import provider_route_status
from .network_surfaces import render_provider_fallback_hosts
from .network_surfaces import resolved_dropin


FLEET_DNS_PLAN_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.dns_activation_plan.v1",
    command="cynic dns plan",
)
FLEET_DNS_BUNDLE_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.dns_activation_bundle.v1",
    command="cynic dns bundle",
)
FLEET_DNS_STATUS_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.dns_activation_status.v1",
    command="cynic dns activation-status",
)
FLEET_MESH_SURFACE = ReportSurfaceSpec(
    schema="cynic.infrastructure_mcp.network_mesh_status.v1",
    command="cynic mesh status",
)
FLEET_PROVIDER_FALLBACK_HOSTS_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.provider_fallback_hosts.v1",
    command="cynic fallback-hosts",
)
FLEET_PROVIDER_FALLBACK_STATUS_SURFACE = ReportSurfaceSpec(
    schema="cynic.organ_fleet.provider_fallback_status.v1",
    command="cynic fallback-status",
)


def print_json(payload: dict[str, object]) -> int:
    print(json.dumps(payload, sort_keys=True))
    return 0


def _cynic_node_observe_payload(timeout: int = 30) -> dict[str, object] | None:
    return _cynic_node_json(["observe", "--json"], timeout=timeout)


def _cynic_node_json(args: list[str], timeout: int = 30) -> dict[str, object] | None:
    try:
        result = subprocess.run(
            ["cargo", "run", "-q", "-p", "cynic-node", "--", *args],
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return None
    if result.returncode != 0 or not result.stdout.strip():
        return None
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError:
        return None
    return payload if isinstance(payload, dict) else None


def _runtime_observation_from_cynic_node(payload: dict[str, object]) -> dict[str, object] | None:
    runtime_observation = payload.get("runtime_observation")
    if not isinstance(runtime_observation, dict):
        return None
    status = runtime_observation.get("status")
    schema = runtime_observation.get("schema")
    if not isinstance(status, str) or not status:
        return None
    if not isinstance(schema, str) or not schema:
        return None
    return runtime_observation


def bridge_policy(policy_path: Path) -> LxcExecBridgePolicyFile:
    return LxcExecBridgePolicyFile(policy_path)


def network_policy(policy_path: Path) -> NetworkActuatorPolicyFile:
    return NetworkActuatorPolicyFile(policy_path)


def host_policy(policy_path: Path) -> HostExecPolicyFile:
    return HostExecPolicyFile(policy_path)


def report(root: Path, args: argparse.Namespace) -> int:
    text_path: Path | None = args.proxmox_list_text
    if args.proxmox_list_text == Path("-"):
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=True) as handle:
            handle.write(sys.stdin.read())
            handle.flush()
            payload = build_report_payload(
                root=root,
                registry_path=args.registry,
                proxmox_json_path=args.proxmox_json,
                proxmox_text_path=Path(handle.name),
                watchdog_json_path=args.watchdog_json,
            )
    else:
        payload = build_report_payload(
            root=root,
            registry_path=args.registry,
            proxmox_json_path=args.proxmox_json,
            proxmox_text_path=text_path,
            watchdog_json_path=args.watchdog_json,
        )
    return print_json(payload)


def dns_plan(_: argparse.Namespace) -> int:
    cynic_node_payload = _cynic_node_json(["network", "dns-plan", "--json"])
    if cynic_node_payload is not None:
        attach_surface_contract(cynic_node_payload, FLEET_DNS_PLAN_SURFACE)
        print(json.dumps(cynic_node_payload, indent=2, sort_keys=True))
        return 0

    payload = dict(dns_activation_plan())
    payload["schema"] = FLEET_DNS_PLAN_SURFACE.schema
    attach_surface_contract(payload, FLEET_DNS_PLAN_SURFACE)
    return print_json(payload)


def dns_bundle(args: argparse.Namespace) -> int:
    cynic_node_payload = _cynic_node_json(
        [
            "network",
            "dns-bundle",
            "--json",
            "--registry",
            str(args.registry),
            "--output-dir",
            str(args.output_dir),
        ]
    )
    if cynic_node_payload is not None:
        attach_surface_contract(cynic_node_payload, FLEET_DNS_BUNDLE_SURFACE)
        print(json.dumps(cynic_node_payload, indent=2, sort_keys=True))
        return 0

    registry_adapter = RegistryAdapter(str(args.registry))
    out = args.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=True)
    hosts = CoreDNSAdapter(state_dir=str(out)).render_registry_records(
        registry_adapter.canonical_dns_records()
    )
    files = {
        "synapse.hosts": hosts,
        "Corefile": coredns_corefile(),
        "coredns-synapse.service": coredns_systemd_unit(),
        "cynic-synapse-dns.conf": resolved_dropin(),
        "README.activation.md": dns_activation_readme(),
    }
    for name, content in files.items():
        (out / name).write_text(content, encoding="utf-8")
    manifest = dict(dns_activation_plan())
    manifest["schema"] = FLEET_DNS_BUNDLE_SURFACE.schema
    manifest["bundle_dir"] = str(out)
    manifest["bundle_files"] = sorted([*files.keys(), "manifest.json"])
    attach_surface_contract(manifest, FLEET_DNS_BUNDLE_SURFACE)
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return print_json(manifest)


def dns_activation_status(args: argparse.Namespace) -> int:
    registry = str(getattr(args, "registry", "infra/registry.json"))
    cynic_node_args = [
        "network",
        "dns-activation-status",
        "--json",
        "--registry",
        registry,
        "--hosts-path",
        str(args.hosts_path),
        "--corefile-path",
        str(args.corefile_path),
        "--service-path",
        str(args.service_path),
        "--resolved-dropin-path",
        str(args.resolved_dropin_path),
    ]
    if args.resolve_dns:
        cynic_node_args.append("--resolve-dns")
    cynic_node_payload = _cynic_node_json(cynic_node_args)
    if cynic_node_payload is not None:
        attach_surface_contract(cynic_node_payload, FLEET_DNS_STATUS_SURFACE)
        print(json.dumps(cynic_node_payload, indent=2, sort_keys=True))
        return 0 if cynic_node_payload.get("status") == "active" else 1

    paths = {
        "hosts": args.hosts_path,
        "corefile": args.corefile_path,
        "service": args.service_path,
        "resolved_dropin": args.resolved_dropin_path,
    }
    checks = {
        "hosts_exists": paths["hosts"].exists(),
        "corefile_exists": paths["corefile"].exists(),
        "service_exists": paths["service"].exists(),
        "resolved_dropin_exists": paths["resolved_dropin"].exists(),
        "coredns_binary_available": dns_provider_status()["canonical_available"],
        "canonical_dns_provider_available": dns_provider_status()["canonical_available"],
        "hosts_contains_storage_record": file_contains(paths["hosts"], "organ-surrealdb.synapse.cynic")
        and file_contains(paths["hosts"], "10.20.0.3"),
        "corefile_references_hosts": file_contains(paths["corefile"], str(paths["hosts"]))
        or file_contains(paths["corefile"], "/etc/coredns/synapse.hosts"),
        "service_references_corefile": file_contains(paths["service"], str(paths["corefile"]))
        or file_contains(paths["service"], "/etc/coredns/Corefile"),
        "resolved_routes_synapse": file_contains(paths["resolved_dropin"], "~synapse.cynic"),
    }
    mesh_status: dict[str, object] = {}
    if args.resolve_dns:
        registry_adapter = RegistryAdapter(str(args.registry))
        mesh_status = json.loads(
            _capture_mesh_status(
                registry_adapter=registry_adapter,
                resolve_dns=True,
                include_records=False,
            )
        )
        checks["synapse_dns_resolves"] = mesh_status.get("dns_status") == "ok"
    missing = [name for name, ok in checks.items() if not ok]
    payload = {
        "schema": FLEET_DNS_STATUS_SURFACE.schema,
        "status": "active" if not missing else "blocked",
        "checks": checks,
        "provider_policy": dns_provider_policy(),
        "provider_status": dns_provider_status(),
        "missing": missing,
        "paths": {name: str(path) for name, path in paths.items()},
        "mesh_status": mesh_status,
    }
    attach_surface_contract(payload, FLEET_DNS_STATUS_SURFACE)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0 if not missing else 1


def mesh_status(args: argparse.Namespace) -> int:
    registry_adapter = RegistryAdapter(str(args.registry))
    runtime_observation = None
    if args.observe_runtime:
        observe_payload = _cynic_node_observe_payload()
        runtime_observation = (
            _runtime_observation_from_cynic_node(observe_payload)
            if isinstance(observe_payload, dict)
            else None
        )
        if runtime_observation is None:
            runtime_observation = observe_network_runtime(
                synapse_cidr=(registry_adapter.network_policy_summary().get("synapse_cidr") or "10.20.0.0/24"),
            )
    payload = mesh_status_payload(
        registry_ref=str(args.registry),
        records=_dns_records(registry_adapter),
        resolve_dns=args.resolve_dns,
        include_records=args.include_records,
        registry_policy=registry_adapter.network_policy_summary(),
        runtime_observation=runtime_observation,
    )
    attach_surface_contract(payload, FLEET_MESH_SURFACE)
    print(json.dumps(payload, indent=2, sort_keys=True))
    if args.observe_runtime and payload["runtime_status"] != "mesh_active":
        return 1
    return 0 if payload["dns_status"] in {"ok", "not_checked"} else 1


def provider_fallback_hosts(args: argparse.Namespace) -> int:
    cynic_node_payload = _cynic_node_json(
        [
            "network",
            "provider-fallback-hosts",
            "--json",
            "--registry",
            str(args.registry),
            "--output-dir",
            str(args.output_dir),
        ]
    )
    if cynic_node_payload is not None:
        attach_surface_contract(cynic_node_payload, FLEET_PROVIDER_FALLBACK_HOSTS_SURFACE)
        print(json.dumps(cynic_node_payload, indent=2, sort_keys=True))
        return 0 if cynic_node_payload.get("status") == "fallback_available" else 1

    registry_adapter = RegistryAdapter(str(args.registry))
    records = registry_adapter.provider_fallback_records()
    out = args.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=True)
    hosts = render_provider_fallback_hosts(records)
    hosts_path = out / "provider-fallback.hosts"
    hosts_path.write_text(hosts, encoding="utf-8")
    payload = {
        "schema": FLEET_PROVIDER_FALLBACK_HOSTS_SURFACE.schema,
        "status": "fallback_available" if records else "empty",
        "canonical": False,
        "mutates_system": False,
        "registry_ref": str(args.registry),
        "hosts_path": str(hosts_path),
        "record_count": len(records),
        "records": records,
        "warning": "Provider fallback DNS is temporary evidence plumbing, not the Synapse service contract.",
        "expires_on": "network_mesh_status.runtime_status == mesh_active",
    }
    attach_surface_contract(payload, FLEET_PROVIDER_FALLBACK_HOSTS_SURFACE)
    (out / "manifest.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0 if records else 1


def provider_fallback_status(args: argparse.Namespace) -> int:
    cynic_node_args = [
        "network",
        "provider-fallback-status",
        "--json",
        "--registry",
        str(args.registry),
        "--hosts-path",
        str(args.hosts_path),
    ]
    if args.check_connectivity:
        cynic_node_args.append("--check-connectivity")
    cynic_node_payload = _cynic_node_json(cynic_node_args)
    if cynic_node_payload is not None:
        attach_surface_contract(cynic_node_payload, FLEET_PROVIDER_FALLBACK_STATUS_SURFACE)
        print(json.dumps(cynic_node_payload, indent=2, sort_keys=True))
        return 0 if cynic_node_payload.get("status") == "active" else 1

    registry_adapter = RegistryAdapter(str(args.registry))
    records = registry_adapter.provider_fallback_records()
    route = registry_adapter.provider_fallback_route()
    service_checks = registry_adapter.provider_fallback_service_checks()
    hosts_path = Path(args.hosts_path)
    hosts_text = hosts_path.read_text(encoding="utf-8", errors="ignore") if hosts_path.exists() else ""
    hosts_missing = [
        name
        for record in records
        for name in record["names"]
        if str(name).endswith(".synapse.cynic") and name not in hosts_text
    ]
    route_status = provider_route_status(route)
    connectivity = provider_connectivity_status(service_checks) if args.check_connectivity else []
    missing = []
    if hosts_missing:
        missing.append("provider_fallback_hosts_entries")
    if not route_status["active"]:
        missing.append("provider_fallback_route_active")
    if not route_status["persistent"]:
        missing.append("provider_fallback_route_persistent")
    blocked_connectivity = [item for item in connectivity if item["network_status"] == "blocked"]
    if blocked_connectivity:
        missing.append("provider_fallback_network_connectivity")
    payload = {
        "schema": FLEET_PROVIDER_FALLBACK_STATUS_SURFACE.schema,
        "status": "active" if not missing else "blocked",
        "canonical": False,
        "registry_ref": str(args.registry),
        "hosts_path": str(hosts_path),
        "checks": {
            "hosts_file_exists": hosts_path.exists(),
            "hosts_entries_present": not hosts_missing,
            "route_active": route_status["active"],
            "route_persistent": route_status["persistent"],
            "connectivity_checked": args.check_connectivity,
            "network_connectivity_ok": not blocked_connectivity if args.check_connectivity else None,
        },
        "missing": missing,
        "hosts_missing": sorted(hosts_missing),
        "route": route_status,
        "connectivity": connectivity,
        "warning": "Provider fallback is a resurrection path only while Synapse/WireGuard remains inactive.",
    }
    attach_surface_contract(payload, FLEET_PROVIDER_FALLBACK_STATUS_SURFACE)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0 if not missing else 1


def _capture_mesh_status(
    *,
    registry_adapter: RegistryAdapter,
    resolve_dns: bool,
    include_records: bool,
) -> str:
    return mesh_status_json(
        registry_ref=str(registry_adapter.registry_path),
        records=_dns_records(registry_adapter),
        resolve_dns=resolve_dns,
        include_records=include_records,
        registry_policy=registry_adapter.network_policy_summary(),
    )


def _dns_records(registry_adapter: RegistryAdapter) -> list[dict]:
    rendered = CoreDNSAdapter(state_dir="/tmp").render_registry_records(
        registry_adapter.canonical_dns_records()
    )
    return dns_records_from_hosts(rendered)


def lxc_validate_policy(args: argparse.Namespace) -> int:
    try:
        policy = bridge_policy(args.policy).load()
        payload: dict[str, object] = {
            "schema": "cynic.lxc_exec_bridge.policy_validation.v1",
            "ok": True,
            "helper_path": policy.helper_path,
            "sudoers_rule": policy.sudoers_rule(),
        }
    except Exception as exc:  # noqa: BLE001
        payload = {
            "schema": "cynic.lxc_exec_bridge.policy_validation.v1",
            "ok": False,
            "error": str(exc),
        }
    return print_json(payload)


def lxc_render_sudoers(args: argparse.Namespace) -> int:
    print(bridge_policy(args.policy).render_sudoers(), end="")
    return 0


def lxc_install_plan(args: argparse.Namespace) -> int:
    return print_json(bridge_policy(args.policy).install_plan(args.helper))


def lxc_bundle(args: argparse.Namespace) -> int:
    return print_json(bridge_policy(args.policy).write_install_bundle(args.helper, args.output_dir))


def lxc_activation_status(args: argparse.Namespace) -> int:
    policy = bridge_policy(args.policy)
    snapshot = policy.filesystem_snapshot(
        helper_path=args.helper_path,
        policy_path=args.policy_path,
        sudoers_path=args.sudoers_path,
    )
    return print_json(policy.activation_status(snapshot))


def network_validate_policy(args: argparse.Namespace) -> int:
    try:
        policy = network_policy(args.policy).load()
        payload: dict[str, object] = {
            "schema": "cynic.network_actuator.policy_validation.v1",
            "ok": True,
            "helper_path": policy.helper_path,
            "sudoers_rule": policy.sudoers_rule(),
        }
    except Exception as exc:  # noqa: BLE001
        payload = {
            "schema": "cynic.network_actuator.policy_validation.v1",
            "ok": False,
            "error": str(exc),
        }
    return print_json(payload)


def network_render_sudoers(args: argparse.Namespace) -> int:
    print(network_policy(args.policy).render_sudoers(), end="")
    return 0


def network_install_plan(args: argparse.Namespace) -> int:
    return print_json(network_policy(args.policy).install_plan(args.helper))


def network_activation_plan(args: argparse.Namespace) -> int:
    return print_json(
        network_policy(args.policy).activation_plan(
            args.helper,
            bundle_dir=args.bundle_dir.resolve(),
        )
    )


def network_bundle(args: argparse.Namespace) -> int:
    return print_json(network_policy(args.policy).write_install_bundle(args.helper, args.output_dir))


def network_activation_status(args: argparse.Namespace) -> int:
    policy = network_policy(args.policy)
    snapshot = policy.filesystem_snapshot(
        helper_path=args.helper_path,
        policy_path=args.policy_path,
        sudoers_path=args.sudoers_path,
    )
    return print_json(
        policy.activation_status(
            snapshot,
            bundle_dir=args.bundle_dir.resolve() if args.bundle_dir else None,
        )
    )


def host_validate_policy(args: argparse.Namespace) -> int:
    try:
        policy = host_policy(args.policy).load()
        payload: dict[str, object] = {
            "schema": "cynic.host_exec.policy_validation.v1",
            "ok": True,
            "helper_path": policy.helper_path,
            "sudoers_rule": policy.sudoers_rule(),
        }
    except Exception as exc:  # noqa: BLE001
        payload = {
            "schema": "cynic.host_exec.policy_validation.v1",
            "ok": False,
            "error": str(exc),
        }
    return print_json(payload)


def host_render_sudoers(args: argparse.Namespace) -> int:
    print(host_policy(args.policy).render_sudoers(), end="")
    return 0


def host_install_plan(args: argparse.Namespace) -> int:
    return print_json(host_policy(args.policy).install_plan(args.helper))


def host_bundle(args: argparse.Namespace) -> int:
    return print_json(host_policy(args.policy).write_install_bundle(args.helper, args.output_dir))


def host_activation_plan(args: argparse.Namespace) -> int:
    return print_json(
        host_policy(args.policy).activation_plan(
            args.helper,
            bundle_dir=args.bundle_dir.resolve(),
        )
    )


def host_activation_status(args: argparse.Namespace) -> int:
    policy = host_policy(args.policy)
    snapshot = policy.filesystem_snapshot(
        helper_path=args.helper_path,
        policy_path=args.policy_path,
        sudoers_path=args.sudoers_path,
    )
    return print_json(
        policy.activation_status(
            snapshot,
            bundle_dir=args.bundle_dir.resolve() if args.bundle_dir else None,
        )
    )


def build_parser(root: Path) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="CYNIC organ-fleet")
    subparsers = parser.add_subparsers(dest="command", required=True)

    report_parser = subparsers.add_parser("report", help="Emit fleet truth report JSON")
    report_parser.add_argument("--json", action="store_true", help="Emit JSON; retained for stable operator usage")
    report_parser.add_argument("--registry", type=Path, default=root / "infra/registry.json")
    report_parser.add_argument("--proxmox-json", type=Path)
    report_parser.add_argument("--proxmox-list-text", type=Path)
    report_parser.add_argument("--watchdog-json", type=Path)
    report_parser.set_defaults(func=lambda args: report(root, args))

    register_dns_subcommands(subparsers, root)
    register_mesh_subcommands(subparsers, root)
    register_provider_fallback_subcommands(subparsers, root)
    register_lxc_subcommands(subparsers, root)
    register_network_subcommands(subparsers, root)
    register_host_subcommands(subparsers, root)
    return parser


def register_dns_subcommands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser], root: Path) -> None:
    dns = subparsers.add_parser("dns", help="Plan, bundle or inspect Synapse DNS activation")
    dns_subparsers = dns.add_subparsers(dest="dns_command", required=True)

    plan = dns_subparsers.add_parser("plan")
    plan.set_defaults(func=dns_plan)

    bundle = dns_subparsers.add_parser("bundle")
    bundle.add_argument("--registry", type=Path, default=root / "infra/registry.json")
    bundle.add_argument("--output-dir", type=Path, required=True)
    bundle.set_defaults(func=dns_bundle)

    status = dns_subparsers.add_parser("activation-status")
    status.add_argument("--hosts-path", type=Path, default=Path("/etc/coredns/synapse.hosts"))
    status.add_argument("--corefile-path", type=Path, default=Path("/etc/coredns/Corefile"))
    status.add_argument(
        "--service-path",
        type=Path,
        default=Path("/etc/systemd/system/coredns-synapse.service"),
    )
    status.add_argument(
        "--resolved-dropin-path",
        type=Path,
        default=Path("/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf"),
    )
    status.add_argument("--resolve-dns", action="store_true")
    status.set_defaults(func=dns_activation_status)


def register_mesh_subcommands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser], root: Path) -> None:
    mesh = subparsers.add_parser("mesh", help="Inspect Synapse mesh policy, DNS and runtime status")
    mesh_subparsers = mesh.add_subparsers(dest="mesh_command", required=True)

    status = mesh_subparsers.add_parser("status")
    status.add_argument("--registry", type=Path, default=root / "infra/registry.json")
    status.add_argument("--resolve-dns", action="store_true")
    status.add_argument("--include-records", action="store_true")
    status.add_argument("--observe-runtime", action="store_true")
    status.set_defaults(func=mesh_status)


def register_provider_fallback_subcommands(
    subparsers: argparse._SubParsersAction[argparse.ArgumentParser],
    root: Path,
) -> None:
    fallback_hosts = subparsers.add_parser(
        "fallback-hosts",
        help="Render temporary provider-plane fallback hosts bundle",
    )
    fallback_hosts.add_argument("--registry", type=Path, default=root / "infra/registry.json")
    fallback_hosts.add_argument("--output-dir", type=Path, required=True)
    fallback_hosts.set_defaults(func=provider_fallback_hosts)

    fallback_status = subparsers.add_parser(
        "fallback-status",
        help="Verify temporary provider-plane fallback hosts and route resurrection path",
    )
    fallback_status.add_argument("--registry", type=Path, default=root / "infra/registry.json")
    fallback_status.add_argument("--hosts-path", type=Path, default=Path("/etc/hosts"))
    fallback_status.add_argument("--check-connectivity", action="store_true")
    fallback_status.set_defaults(func=provider_fallback_status)


def register_lxc_subcommands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser], root: Path) -> None:
    bridge = subparsers.add_parser("lxc-exec-bridge", help="Inspect LXC exec bridge policy")
    bridge_subparsers = bridge.add_subparsers(dest="bridge_command", required=True)
    default_policy = root / "organs/fleet/policies/lxc-exec-bridge.policy.json"
    default_helper = root / "organs/fleet/src/entrypoints/lxc_exec_bridge_helper.py"

    validate = bridge_subparsers.add_parser("validate-policy")
    validate.add_argument("--json", action="store_true")
    validate.add_argument("--policy", type=Path, default=default_policy)
    validate.set_defaults(func=lxc_validate_policy)

    render = bridge_subparsers.add_parser("render-sudoers")
    render.add_argument("--policy", type=Path, default=default_policy)
    render.set_defaults(func=lxc_render_sudoers)

    plan = bridge_subparsers.add_parser("install-plan")
    plan.add_argument("--policy", type=Path, default=default_policy)
    plan.add_argument("--helper", type=Path, default=default_helper)
    plan.set_defaults(func=lxc_install_plan)

    bundle = bridge_subparsers.add_parser("bundle")
    bundle.add_argument("--policy", type=Path, default=default_policy)
    bundle.add_argument("--helper", type=Path, default=default_helper)
    bundle.add_argument("--output-dir", type=Path, required=True)
    bundle.set_defaults(func=lxc_bundle)

    status = bridge_subparsers.add_parser("activation-status")
    status.add_argument("--policy", type=Path, default=default_policy)
    status.add_argument("--helper-path", type=Path)
    status.add_argument("--policy-path", type=Path)
    status.add_argument("--sudoers-path", type=Path)
    status.set_defaults(func=lxc_activation_status)


def register_network_subcommands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser], root: Path) -> None:
    network = subparsers.add_parser("network-actuator", help="Inspect network actuator policy")
    network_subparsers = network.add_subparsers(dest="network_command", required=True)
    default_policy = root / "organs/fleet/policies/network-actuator.policy.json"
    default_helper = root / "organs/fleet/src/entrypoints/network_actuator_helper.py"

    validate = network_subparsers.add_parser("validate-policy")
    validate.add_argument("--json", action="store_true")
    validate.add_argument("--policy", type=Path, default=default_policy)
    validate.set_defaults(func=network_validate_policy)

    render = network_subparsers.add_parser("render-sudoers")
    render.add_argument("--policy", type=Path, default=default_policy)
    render.set_defaults(func=network_render_sudoers)

    plan = network_subparsers.add_parser("install-plan")
    plan.add_argument("--policy", type=Path, default=default_policy)
    plan.add_argument("--helper", type=Path, default=default_helper)
    plan.set_defaults(func=network_install_plan)

    activation_plan = network_subparsers.add_parser("activation-plan")
    activation_plan.add_argument("--policy", type=Path, default=default_policy)
    activation_plan.add_argument("--helper", type=Path, default=default_helper)
    activation_plan.add_argument("--bundle-dir", type=Path, default=Path("/tmp/cynic-network-actuator"))
    activation_plan.set_defaults(func=network_activation_plan)

    bundle = network_subparsers.add_parser("bundle")
    bundle.add_argument("--policy", type=Path, default=default_policy)
    bundle.add_argument("--helper", type=Path, default=default_helper)
    bundle.add_argument("--output-dir", type=Path, required=True)
    bundle.set_defaults(func=network_bundle)

    status = network_subparsers.add_parser("activation-status")
    status.add_argument("--policy", type=Path, default=default_policy)
    status.add_argument("--helper-path", type=Path)
    status.add_argument("--policy-path", type=Path)
    status.add_argument("--sudoers-path", type=Path)
    status.add_argument("--bundle-dir", type=Path)
    status.set_defaults(func=network_activation_status)


def register_host_subcommands(subparsers: argparse._SubParsersAction[argparse.ArgumentParser], root: Path) -> None:
    host = subparsers.add_parser("host-exec", help="Inspect host exec policy")
    host_subparsers = host.add_subparsers(dest="host_command", required=True)
    default_policy = root / "organs/fleet/policies/host-exec.policy.json"
    default_helper = root / "organs/fleet/src/entrypoints/host_exec_helper.py"

    validate = host_subparsers.add_parser("validate-policy")
    validate.add_argument("--json", action="store_true")
    validate.add_argument("--policy", type=Path, default=default_policy)
    validate.set_defaults(func=host_validate_policy)

    render = host_subparsers.add_parser("render-sudoers")
    render.add_argument("--policy", type=Path, default=default_policy)
    render.set_defaults(func=host_render_sudoers)

    plan = host_subparsers.add_parser("install-plan")
    plan.add_argument("--policy", type=Path, default=default_policy)
    plan.add_argument("--helper", type=Path, default=default_helper)
    plan.set_defaults(func=host_install_plan)

    activation_plan = host_subparsers.add_parser("activation-plan")
    activation_plan.add_argument("--policy", type=Path, default=default_policy)
    activation_plan.add_argument("--helper", type=Path, default=default_helper)
    activation_plan.add_argument("--bundle-dir", type=Path, default=Path("/tmp/cynic-host-exec"))
    activation_plan.set_defaults(func=host_activation_plan)

    bundle = host_subparsers.add_parser("bundle")
    bundle.add_argument("--policy", type=Path, default=default_policy)
    bundle.add_argument("--helper", type=Path, default=default_helper)
    bundle.add_argument("--output-dir", type=Path, required=True)
    bundle.set_defaults(func=host_bundle)

    status = host_subparsers.add_parser("activation-status")
    status.add_argument("--policy", type=Path, default=default_policy)
    status.add_argument("--helper-path", type=Path)
    status.add_argument("--policy-path", type=Path)
    status.add_argument("--sudoers-path", type=Path)
    status.add_argument("--bundle-dir", type=Path)
    status.set_defaults(func=host_activation_status)


def main(root: Path, argv: list[str] | None = None) -> int:
    parser = build_parser(root)
    args = parser.parse_args(argv)
    return args.func(args)
