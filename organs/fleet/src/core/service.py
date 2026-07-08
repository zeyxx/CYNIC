from __future__ import annotations

from dataclasses import asdict
from typing import Any

from .entities import Anomaly, FleetReport, Node, ObservedResource


class FleetTruthService:
    """Compatibility mirror for the Rust fleet-truth domain in cynic-node."""

    def build_report(
        self,
        declared_nodes: list[Node],
        observed_resources: list[ObservedResource],
        registry_organs: set[str],
        organ_dirs: set[str],
    ) -> FleetReport:
        observed_by_identity: dict[str, ObservedResource] = {}
        for resource in observed_resources:
            for identity in resource.identities:
                observed_by_identity[identity] = resource

        anomalies: list[Anomaly] = []
        effective: list[dict[str, Any]] = []

        for node in declared_nodes:
            observed = self._match_observed(node, observed_by_identity)
            if node.status == "active" and observed is None:
                anomalies.append(
                    Anomaly(
                        severity="critical",
                        kind="active_without_runtime_proof",
                        target=node.name,
                        message="Node is declared active but no provider observation matched its name, aliases, or Proxmox id.",
                    ),
                )
            if not node.energy_profile:
                anomalies.append(
                    Anomaly(
                        severity="warning",
                        kind="missing_energy_profile",
                        target=node.name,
                        message="Node has no energy_profile; intentional sleep cannot be distinguished from failure.",
                    ),
                )

            effective.append(
                {
                    "name": node.name,
                    "role": node.role,
                    "constellation": node.constellation,
                    "declared_status": node.status,
                    "observed_status": observed.status if observed else "unobserved",
                    "effective_status": self._effective_status(node, observed),
                    "provider": observed.provider if observed else None,
                    "observed_constellation": observed.constellation if observed else None,
                },
            )

        declared_identities = set()
        for node in declared_nodes:
            declared_identities.update(node.identities)

        for resource in observed_resources:
            if declared_identities.isdisjoint(resource.identities):
                anomalies.append(
                    Anomaly(
                        severity="warning",
                        kind="observed_unregistered_resource",
                        target=resource.name,
                        message=f"{resource.provider} observed resource is not declared in registry.",
                    ),
                )

        for directory in sorted(organ_dirs):
            canonical = directory if directory.startswith("organ-") else f"organ-{directory}"
            if directory not in registry_organs and canonical not in registry_organs:
                anomalies.append(
                    Anomaly(
                        severity="warning",
                        kind="organ_dir_without_registry_state",
                        target=directory,
                        message="Organ directory exists without active/planned/absorbed/archived registry state.",
                    ),
                )

        for organ in sorted(registry_organs):
            candidates = {organ, organ.removeprefix("organ-")}
            if organ != "organ-fleet" and organ not in organ_dirs and candidates.isdisjoint(organ_dirs):
                anomalies.append(
                    Anomaly(
                        severity="warning",
                        kind="registry_organ_without_directory",
                        target=organ,
                        message="Registry declares organ but no matching organs/ directory exists.",
                    ),
                )

        energy = self._energy_summary(declared_nodes)
        anomaly_dicts = [asdict(anomaly) for anomaly in anomalies]

        return FleetReport(
            summary=self._summary(declared_nodes, observed_resources, anomaly_dicts),
            desired=[asdict(node) for node in declared_nodes],
            declared=[asdict(node) for node in declared_nodes],
            observed=[asdict(resource) for resource in observed_resources],
            effective=effective,
            anomalies=anomaly_dicts,
            energy=energy,
        )

    def _match_observed(
        self,
        node: Node,
        observed_by_identity: dict[str, ObservedResource],
    ) -> ObservedResource | None:
        ordered_identities = [
            node.name,
            *node.aliases,
            *([node.proxmox_id] if node.proxmox_id else []),
        ]
        for identity in ordered_identities:
            if identity in observed_by_identity:
                return observed_by_identity[identity]
        return None

    def _effective_status(self, node: Node, observed: ObservedResource | None) -> str:
        if node.status in {"planned", "planned_obsolescence", "archived"}:
            return node.status
        if observed and observed.status == "running":
            return "active"
        if node.status == "active":
            return "stale"
        return node.status

    def _energy_summary(self, nodes: list[Node]) -> dict[str, Any]:
        by_class: dict[str, int] = {}
        missing = []
        for node in nodes:
            energy_class = node.energy_profile.get("class")
            if not energy_class:
                missing.append(node.name)
                continue
            by_class[energy_class] = by_class.get(energy_class, 0) + 1
        return {
            "classes": by_class,
            "missing_profiles": missing,
        }

    def _summary(
        self,
        nodes: list[Node],
        observed_resources: list[ObservedResource],
        anomalies: list[dict[str, Any]],
    ) -> dict[str, Any]:
        critical = [
            anomaly
            for anomaly in anomalies
            if anomaly.get("severity") == "critical"
        ]
        return {
            "status": "degraded" if critical else "ok",
            "declared_nodes": len(nodes),
            "observed_resources": len(observed_resources),
            "constellations": self._constellation_summary(nodes, observed_resources),
            "anomaly_count": len(anomalies),
            "critical_anomaly_count": len(critical),
            "critical_targets": [anomaly["target"] for anomaly in critical],
        }

    def _constellation_summary(
        self,
        nodes: list[Node],
        observed_resources: list[ObservedResource],
    ) -> dict[str, Any]:
        declared: dict[str, int] = {}
        observed: dict[str, int] = {}
        for node in nodes:
            if node.constellation:
                declared[node.constellation] = declared.get(node.constellation, 0) + 1
        for resource in observed_resources:
            if resource.constellation:
                observed[resource.constellation] = observed.get(resource.constellation, 0) + 1
        return {
            "declared": declared,
            "observed": observed,
        }
