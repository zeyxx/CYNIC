"""Validate Rust crate role taxonomy against the real workspace."""

from __future__ import annotations

from typing import Any

import organs.supervisor.src.core.repo_topology_host as host


CONTRACT_PATH = host.CONTRACTS_ROOT / "rust-crate-roles-v1.json"
WORKSPACE_TOML_PATH = host.WORKSPACE_ROOT / "Cargo.toml"
EXPECTED_SCHEMA = "cynic.rust_crate_roles.v1"


def validate_rust_crate_roles_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not CONTRACT_PATH.exists():
        return [
            host.Finding(
                "critical",
                "rust_crate_roles_contract_missing",
                host.relative(CONTRACT_PATH),
                "rust-crate-roles-v1.json must exist to classify visible Rust crates.",
            )
        ]

    contract = host.load_json(CONTRACT_PATH)
    if contract.get("schema") != EXPECTED_SCHEMA:
        return [
            host.Finding(
                "critical",
                "unexpected_rust_crate_roles_schema",
                host.relative(CONTRACT_PATH),
                f"expected {EXPECTED_SCHEMA}, got {contract.get('schema')!r}",
            )
        ]

    crates = contract.get("crates")
    if not isinstance(crates, list):
        return [
            host.Finding(
                "critical",
                "rust_crate_roles_entries_missing",
                host.relative(CONTRACT_PATH),
                "rust-crate-roles-v1.json must declare crates as a list.",
            )
        ]

    declared = _declared_crates(crates, findings)
    visible = _visible_rust_crates()
    workspace_members = _workspace_members()

    for rel, entry in sorted(declared.items()):
        crate_dir = host.WORKSPACE_ROOT / rel
        cargo_toml = crate_dir / "Cargo.toml"
        if entry.get("package") is None:
            if cargo_toml.exists():
                findings.append(
                    host.Finding(
                        "critical",
                        "rust_crate_roles_package_missing",
                        rel,
                        "crate entry points to a real Cargo.toml but package is null.",
                    )
                )
            continue
        if not cargo_toml.exists():
            findings.append(
                host.Finding(
                    "critical",
                    "rust_crate_roles_path_missing",
                    rel,
                    "declared crate path does not contain Cargo.toml.",
                )
            )
            continue
        manifest = host.load_toml(cargo_toml)
        manifest_package = str((manifest.get("package") or {}).get("name") or "")
        if manifest_package != str(entry.get("package")):
            findings.append(
                host.Finding(
                    "critical",
                    "rust_crate_roles_package_mismatch",
                    rel,
                    f"declared package {entry.get('package')!r} does not match Cargo.toml package.name {manifest_package!r}.",
                )
            )
        declared_member = bool(entry.get("workspace_member"))
        actual_member = rel in workspace_members
        if declared_member != actual_member:
            findings.append(
                host.Finding(
                    "critical",
                    "rust_crate_roles_workspace_member_mismatch",
                    rel,
                    f"workspace_member={declared_member!r} but Cargo workspace membership is {actual_member!r}.",
                )
            )

    for rel in sorted(visible - set(declared)):
        findings.append(
            host.Finding(
                "critical",
                "rust_crate_visible_unclassified",
                rel,
                "visible Rust crate must be classified in rust-crate-roles-v1.json.",
            )
        )

    for rel in sorted(set(declared) - visible):
        entry = declared[rel]
        if entry.get("package") is not None:
            findings.append(
                host.Finding(
                    "critical",
                    "rust_crate_roles_stale_path",
                    rel,
                    "crate role entry points to a Cargo crate path that is not visible on disk.",
                )
            )

    return findings


def _declared_crates(
    crates: list[Any],
    findings: list[host.Finding],
) -> dict[str, dict[str, Any]]:
    declared: dict[str, dict[str, Any]] = {}
    for index, item in enumerate(crates):
        pointer = f"{host.relative(CONTRACT_PATH)}:crates[{index}]"
        if not isinstance(item, dict):
            findings.append(
                host.Finding(
                    "critical",
                    "rust_crate_roles_entry_invalid",
                    pointer,
                    "each crates entry must be an object.",
                )
            )
            continue
        rel = str(item.get("path") or "")
        if not rel:
            findings.append(
                host.Finding(
                    "critical",
                    "rust_crate_roles_path_missing",
                    pointer,
                    "each crates entry must declare path.",
                )
            )
            continue
        if rel in declared:
            findings.append(
                host.Finding(
                    "critical",
                    "rust_crate_roles_duplicate_path",
                    rel,
                    "crate path is declared more than once.",
                )
            )
            continue
        declared[rel] = item
    return declared


def _workspace_members() -> set[str]:
    workspace = host.load_toml(WORKSPACE_TOML_PATH)
    members = (workspace.get("workspace") or {}).get("members") or []
    return {str(item).rstrip("/") for item in members if isinstance(item, str)}


def _visible_rust_crates() -> set[str]:
    visible = {
        host.relative(path.parent).rstrip("/")
        for path in sorted((host.WORKSPACE_ROOT / "crates").glob("*/Cargo.toml"))
    }
    ignis = host.WORKSPACE_ROOT / "services" / "ignis" / "program" / "Cargo.toml"
    if ignis.exists():
        visible.add("services/ignis/program")
    return visible
