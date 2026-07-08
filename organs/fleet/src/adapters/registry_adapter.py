from __future__ import annotations

import ipaddress
import os

from organs.contracts.src.surface_contracts import read_json_file_or_default

from .network_entities import MeshNode


class RegistryTopologyError(ValueError):
    pass


def _is_legacy_mesh_controller(item: dict) -> bool:
    if not isinstance(item, dict):
        return False
    return item.get("name") == "organ-wireguard" or item.get("role") in {
        "synapse_mesh_controller",
        "legacy_synapse_mesh_implementation",
    }


class RegistryAdapter:
    def __init__(self, registry_path: str):
        self.registry_path = registry_path

    def canonical_dns_records(self) -> list[dict]:
        data = read_json_file_or_default(self.registry_path, {})
        self._validate_network_policy(data)
        reservations = {
            item.get("name"): item
            for item in data.get("network_policy", {}).get("reservations", [])
            if isinstance(item, dict) and item.get("name")
        }
        vm_aliases = self._vm_aliases_by_name(data)
        records = []
        for node_data in data.get("nodes", []):
            network = node_data.get("network", {})
            synapse_ip = network.get("synapse")
            primary_name = node_data.get("name", "")
            if not primary_name or not synapse_ip or str(synapse_ip).startswith("<"):
                continue

            records.append(
                {
                    "node": primary_name,
                    "ip": synapse_ip,
                    "names": self._canonical_names_for(
                        primary_name,
                        node_data.get("aliases", []),
                        reservations.get(primary_name, {}).get("dns"),
                    ),
                }
            )

            aliases = []
            for alias in [*node_data.get("aliases", []), *vm_aliases.get(primary_name, [])]:
                if alias and alias not in aliases:
                    aliases.append(alias)
            for alias in aliases:
                records.append(
                    {
                        "node": alias,
                        "ip": synapse_ip,
                        "names": self._canonical_names_for(alias, [], None),
                    }
                )
        return records

    def fetch_static_topology(self) -> list[MeshNode]:
        if not os.path.exists(self.registry_path):
            raise FileNotFoundError(f"Registry not found at {self.registry_path}")

        nodes = []
        for record in self.canonical_dns_records():
            nodes.append(
                MeshNode(
                    id=record["node"],
                    name=record["node"],
                    public_key="",
                    mesh_ip=record["ip"],
                )
            )

        return nodes

    def network_policy_summary(self) -> dict:
        data = read_json_file_or_default(self.registry_path, {})
        policy = data.get("network_policy", {})
        synapse = policy.get("planes", {}).get("synapse", {})
        provider_planes = {
            name: plane
            for name, plane in policy.get("planes", {}).items()
            if isinstance(plane, dict) and plane.get("canonical") is False
        }
        legacy_mesh_controller = next(
            (
                item
                for item in data.get("nodes", [])
                if _is_legacy_mesh_controller(item)
            ),
            {},
        )
        return {
            "canonical_service_plane": policy.get("canonical_service_plane"),
            "synapse_cidr": synapse.get("cidr"),
            "synapse_dns_zone": synapse.get("dns_zone"),
            "legacy_mesh_controller_status": legacy_mesh_controller.get("status"),
            "organ_wireguard_status": legacy_mesh_controller.get("status"),
            "provider_fallback_planes": sorted(provider_planes),
        }

    def provider_fallback_records(self) -> list[dict]:
        data = read_json_file_or_default(self.registry_path, {})
        reservations = {
            item.get("name"): item
            for item in data.get("network_policy", {}).get("reservations", [])
            if isinstance(item, dict) and item.get("name")
        }
        records = []
        for node_data in data.get("nodes", []):
            name = node_data.get("name")
            local_ip = (node_data.get("network") or {}).get("local")
            if not name or not local_ip or local_ip == "dhcp":
                continue
            names = set(node_data.get("aliases", []))
            names.add(name)
            reservation = reservations.get(name, {})
            for dns_name in reservation.get("dns", []):
                if isinstance(dns_name, str) and dns_name.endswith(".synapse.cynic"):
                    names.add(dns_name)
            records.append(
                {
                    "node": name,
                    "role": node_data.get("role"),
                    "provider_plane": "proxmox_guest" if str(local_ip).startswith("10.10.10.") else "lan",
                    "ip": local_ip,
                    "names": sorted(names),
                    "canonical": False,
                }
            )
        return sorted(records, key=lambda item: item["node"])

    def provider_fallback_route(self) -> dict:
        data = read_json_file_or_default(self.registry_path, {})
        policy = data.get("network_policy", {})
        proxmox_guest = policy.get("planes", {}).get("proxmox_guest", {})
        hypervisor = next(
            (
                item
                for item in data.get("nodes", [])
                if item.get("role") == "hypervisor"
            ),
            {},
        )
        return {
            "cidr": proxmox_guest.get("cidr", "10.10.10.0/24"),
            "gateway": (hypervisor.get("network") or {}).get("local"),
            "canonical": False,
            "provider_plane": "proxmox_guest",
        }

    def provider_fallback_service_checks(self) -> list[dict]:
        data = read_json_file_or_default(self.registry_path, {})
        checks = []
        for record in self.provider_fallback_records():
            node = next(
                (item for item in data.get("nodes", []) if item.get("name") == record["node"]),
                {},
            )
            for service in node.get("services", []):
                port = service.get("port")
                if not isinstance(port, int):
                    continue
                names = [name for name in record["names"] if str(name).endswith(".synapse.cynic")]
                host = names[0] if names else record["ip"]
                checks.append(
                    {
                        "node": record["node"],
                        "role": record.get("role"),
                        "service": service.get("name"),
                        "host": host,
                        "ip": record["ip"],
                        "port": port,
                        "required": True,
                    }
                )
        return sorted(checks, key=lambda item: (item["node"], item["service"]))

    def _validate_network_policy(self, data: dict) -> None:
        policy = data.get("network_policy", {})
        if policy.get("canonical_service_plane") != "synapse":
            raise RegistryTopologyError("network_policy.canonical_service_plane must be synapse")

        synapse_plane = policy.get("planes", {}).get("synapse", {})
        cidr = synapse_plane.get("cidr")
        if not cidr:
            raise RegistryTopologyError("network_policy.planes.synapse.cidr is required")
        network = ipaddress.ip_network(cidr)

        reservations = {
            item.get("name"): item
            for item in policy.get("reservations", [])
            if isinstance(item, dict) and item.get("name")
        }
        if not reservations:
            raise RegistryTopologyError("network_policy.reservations must declare Synapse IPAM")

        seen_ips: dict[str, str] = {}
        for node_data in data.get("nodes", []):
            name = node_data.get("name", "")
            synapse_ip = (node_data.get("network") or {}).get("synapse")
            if not synapse_ip or str(synapse_ip).startswith("<"):
                continue

            try:
                ip = ipaddress.ip_address(synapse_ip)
            except ValueError as exc:
                raise RegistryTopologyError(f"{name}: invalid synapse IP {synapse_ip}") from exc
            if ip not in network:
                raise RegistryTopologyError(f"{name}: synapse IP {synapse_ip} outside {cidr}")

            previous = seen_ips.get(str(ip))
            if previous:
                raise RegistryTopologyError(
                    f"duplicate synapse IP {synapse_ip}: {previous} and {name}"
                )
            seen_ips[str(ip)] = name

            reservation = reservations.get(name)
            if not reservation:
                raise RegistryTopologyError(f"{name}: missing network_policy reservation")
            if reservation.get("synapse") != synapse_ip:
                raise RegistryTopologyError(
                    f"{name}: registry network.synapse {synapse_ip} does not match reservation "
                    f"{reservation.get('synapse')}"
                )

    def _vm_aliases_by_name(self, data: dict) -> dict[str, list[str]]:
        aliases: dict[str, list[str]] = {}
        for node_data in data.get("nodes", []):
            for vm_data in node_data.get("vms", []):
                name = vm_data.get("name")
                if not name:
                    continue
                aliases.setdefault(name, [])
                for alias in vm_data.get("aliases", []):
                    if alias and alias not in aliases[name]:
                        aliases[name].append(alias)
        return aliases

    def _canonical_names_for(
        self,
        primary_name: str,
        aliases: list[str],
        reserved_dns: list[str] | None,
    ) -> list[str]:
        names = {f"{primary_name}.synapse.cynic"}
        for alias in aliases:
            if alias:
                names.add(f"{alias}.synapse.cynic")
        for host in reserved_dns or []:
            if isinstance(host, str) and host:
                names.add(host)
        return sorted(names)
