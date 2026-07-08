"""Runtime artifact manifest for lifecycle propagation.

The manifest is the shared grammar for Rust binaries, Python organs, rendered
agent projections, and service units. It keeps MCP freshness from becoming a
one-off lifecycle path.
"""

from __future__ import annotations

from datetime import datetime, timezone


RUNTIME_ARTIFACT_MANIFEST_SCHEMA = "cynic.lifecycle.runtime_artifact_manifest.v1"

REQUIRED_STAGES = [
    "source",
    "build_or_package",
    "publish",
    "propagate",
    "activate",
    "prove",
]


def _utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat()


def build_runtime_artifact_manifest(
    registry: dict,
    *,
    observed: dict | None = None,
    now: str | None = None,
) -> dict:
    lifecycle = registry.get("lifecycle", {}).get("kernel_release", {})
    declared = lifecycle.get("runtime_artifacts", [])
    if not isinstance(declared, list):
        raise RuntimeError("lifecycle.kernel_release.runtime_artifacts must be a list")

    artifacts = []
    drift = []
    observed = observed or {}
    observed_artifacts = observed.get("artifacts", {}) if isinstance(observed, dict) else {}
    if not isinstance(observed_artifacts, dict):
        observed_artifacts = {}

    for artifact in declared:
        normalized = normalize_runtime_artifact(artifact)
        artifact_observed = observed_artifacts.get(normalized["name"], {})
        artifact_drift = compare_runtime_artifact(normalized, artifact_observed)
        if artifact_drift:
            drift.extend(artifact_drift)
        artifacts.append({**normalized, "observed": artifact_observed, "drift": artifact_drift})

    return {
        "schema": RUNTIME_ARTIFACT_MANIFEST_SCHEMA,
        "status": "ok" if not drift else "drift",
        "generated_at": now or _utc_now(),
        "lifecycle": "kernel_release",
        "artifact_count": len(artifacts),
        "artifacts": artifacts,
        "drift": drift,
    }


def normalize_runtime_artifact(artifact: dict) -> dict:
    if not isinstance(artifact, dict):
        raise RuntimeError("runtime_artifacts entries must be objects")

    name = artifact.get("name")
    kind = artifact.get("kind")
    if not isinstance(name, str) or not name:
        raise RuntimeError("runtime_artifacts[].name missing")
    if kind not in {"rust_binary", "python_organ", "agent_projection", "systemd_unit"}:
        raise RuntimeError(f"runtime_artifacts.{name}.kind is invalid")

    stages = artifact.get("lifecycle_stages", REQUIRED_STAGES)
    if stages != REQUIRED_STAGES:
        raise RuntimeError(f"runtime_artifacts.{name}.lifecycle_stages must be canonical")

    source_targets = artifact.get("source_targets", [])
    runtime_targets = artifact.get("runtime_targets", [])
    activation = artifact.get("activation", [])
    proof = artifact.get("proof", [])
    for field_name, value in [
        ("source_targets", source_targets),
        ("runtime_targets", runtime_targets),
        ("activation", activation),
        ("proof", proof),
    ]:
        if not isinstance(value, list) or not value:
            raise RuntimeError(f"runtime_artifacts.{name}.{field_name} must be a non-empty list")

    return {
        "name": name,
        "kind": kind,
        "owner": artifact.get("owner", "organ-forge"),
        "source_targets": source_targets,
        "build_targets": artifact.get("build_targets", []),
        "system_packages": artifact.get("system_packages", []),
        "publish": artifact.get("publish", {}),
        "runtime_targets": runtime_targets,
        "activation": activation,
        "proof": proof,
        "lifecycle_stages": stages,
        "request_profiles": artifact.get("request_profiles", []),
    }


def compare_runtime_artifact(declared: dict, observed: dict) -> list[dict]:
    if not observed:
        return [
            {
                "artifact": declared["name"],
                "kind": "proof_drift",
                "reason": "no observed runtime artifact evidence",
            }
        ]

    drift = []
    declared_hash = declared.get("publish", {}).get("hash")
    observed_hash = observed.get("hash")
    if declared_hash and observed_hash and declared_hash != observed_hash:
        drift.append(
            {
                "artifact": declared["name"],
                "kind": "artifact_hash_drift",
                "declared": declared_hash,
                "observed": observed_hash,
            }
        )

    for proof_name in declared.get("proof", []):
        proof_status = observed.get("proof", {}).get(proof_name)
        if proof_status is not True:
            drift.append(
                {
                    "artifact": declared["name"],
                    "kind": "proof_drift",
                    "proof": proof_name,
                    "observed": proof_status,
                }
            )
    return drift
