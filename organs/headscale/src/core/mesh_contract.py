from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class HeadscaleContract:
    name: str = "organ-headscale"
    role: str = "mesh_access_control"
    interfaces: tuple[str, ...] = (
        "list_nodes",
        "generate_preauthkey",
        "observe_node_liveness",
        "revoke_node",
    )
    rules: tuple[str, ...] = (
        "owns_access_mesh_only",
        "does_not_own_synapse_service_mesh",
        "does_not_run_kernel",
    )
    status: str = "active"
    dependencies: tuple[str, ...] = ("cynic-kernel",)
    evidence: tuple[str, ...] = field(default_factory=lambda: ("MANIFEST.yaml", "README.md"))

    def as_dict(self) -> dict[str, object]:
        return {
            "name": self.name,
            "role": self.role,
            "status": self.status,
            "dependencies": list(self.dependencies),
            "interfaces": list(self.interfaces),
            "rules": list(self.rules),
            "evidence": list(self.evidence),
        }

