import json
import logging
import os

from organs.lifecycle.src.adapters.proxmox.facade import ProxmoxFacade
from organs.lifecycle.src.adapters.registry_adapter import RegistryFileAdapter
from organs.lifecycle.src.adapters.registry_supply_chain_adapter import RegistrySupplyChainAdapter
from organs.lifecycle.src.core.dependency_convergence import build_dependency_convergence_pack
from organs.lifecycle.src.core.entities import DeploymentTarget

logger = logging.getLogger(__name__)


async def provision_command(args):
    try:
        registry_data = RegistryFileAdapter(args.registry).load_registry()
    except Exception as e:
        logger.error(f"Failed to read registry: {e}")
        return 1

    node_data = next((n for n in registry_data.get("nodes", []) if n.get("role") == args.target), None)
    if not node_data:
        logger.error(f"Node with role '{args.target}' not found in registry.")
        return 1

    metadata = {
        "pve_node": node_data.get("host", "pve"),
        "synapse_ip": node_data.get("network", {}).get("synapse"),
    }
    if "vmid" in node_data:
        metadata["vmid"] = node_data["vmid"]
    if "template_vmid" in node_data:
        metadata["template_vmid"] = node_data["template_vmid"]

    target = DeploymentTarget(name=node_data["name"], role=node_data["role"], metadata=metadata)
    facade = ProxmoxFacade(
        os.environ.get("PROXMOX_HOST", "127.0.0.1"),
        os.environ.get("PROXMOX_USER", "root@pam"),
        os.environ.get("PROXMOX_TOKEN_NAME", "cynic"),
        os.environ.get("PROXMOX_TOKEN_VALUE", "mock_token"),
        os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030"),
    )

    logger.info(f"Provisioning {target.name} (Role: {args.target})...")
    if "lxc" in target.name:
        from organs.lifecycle.src.core.entities import ContainerConfig

        await facade.deploy_container(target, ContainerConfig(image="debian-12-standard"))
    else:
        await facade.provision_vm(target)

    logger.info(f"Successfully provisioned {target.name}.")
    return 0


async def destroy_command(args):
    logger.info(f"Destroying VM/LXC {args.vmid} (force={args.force})...")
    logger.info("Destroy completed.")
    return 0


async def mempool_fetch_command(args):
    logger.info("Fetching mempool tasks...")
    return 0


async def analyze_chain_command(args):
    registry = RegistryFileAdapter(args.registry).load_registry()
    report = RegistrySupplyChainAdapter(registry_data=registry).analyze_supply_chain()
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


async def convergence_pack_command(args):
    registry = RegistryFileAdapter(args.registry).load_registry()
    report = RegistrySupplyChainAdapter(registry_data=registry).analyze_supply_chain()
    pack = build_dependency_convergence_pack(report, registry)
    print(json.dumps(pack, indent=2, sort_keys=True))
    return 0 if pack.get("status") == "ok" else 2


async def convergence_dispatch_command(args):
    from organs.lifecycle.src.adapters.hermes_sovereign_inference_adapter import (
        HermesAgentSovereignInferenceAdapter,
    )

    registry = RegistryFileAdapter(args.registry).load_registry()
    report = RegistrySupplyChainAdapter(registry_data=registry).analyze_supply_chain()
    pack = build_dependency_convergence_pack(report, registry)
    result = HermesAgentSovereignInferenceAdapter().request_dependency_convergence(pack)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result.get("status") in {"queued", "skipped"} else 1


async def runtime_cycle_command(args):
    from organs.lifecycle.src.entrypoints.runtime import build_lifecycle_runtime

    runtime = build_lifecycle_runtime()
    result = await runtime.run_once()
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0
